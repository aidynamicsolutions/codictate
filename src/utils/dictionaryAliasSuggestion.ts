import type { CustomWordEntry } from "@/bindings";
import {
  dictionaryEntryIdentity,
  normalizeAliases,
  normalizeDictionaryTerm,
} from "./dictionaryUtils";

const SPLIT_WINDOW_SIZES = [2, 3] as const;
const MIN_COLLAPSED_LEN = 5;
const MIN_LEN_RATIO = 0.65;
const MAX_LEN_RATIO = 1.35;
const MAX_DISTANCE_SCORE = 0.34;
const DIFFICULT_SPLIT_MAX_DISTANCE_SCORE = 0.52;
const FIRST_CHAR_MISMATCH_HARD_LIMIT = 0.18;

const GUARD_WORDS = new Set([
  "a",
  "an",
  "and",
  "are",
  "as",
  "at",
  "be",
  "for",
  "from",
  "in",
  "is",
  "it",
  "of",
  "on",
  "or",
  "the",
  "to",
  "with",
]);

export interface AliasSuggestion {
  entryIndex: number;
  entryIdentity: string;
  entryInput: string;
  alias: string;
  score: number;
}

function stripOuterPunctuation(token: string): string {
  return token.replace(/^[^\p{L}\p{N}]+|[^\p{L}\p{N}]+$/gu, "");
}

function tokenizeTranscript(text: string): string[] {
  return text
    .split(/\s+/)
    .map((token) => stripOuterPunctuation(token))
    .filter((token) => token.length > 0);
}

function sharedPrefixLength(a: string, b: string): number {
  const maxLen = Math.min(a.length, b.length);
  let prefixLen = 0;
  while (prefixLen < maxLen && a[prefixLen] === b[prefixLen]) {
    prefixLen += 1;
  }
  return prefixLen;
}

function levenshteinDistance(a: string, b: string): number {
  const rows = a.length + 1;
  const cols = b.length + 1;
  const matrix = Array.from({ length: rows }, () =>
    Array<number>(cols).fill(0),
  );

  for (let i = 0; i < rows; i += 1) {
    matrix[i][0] = i;
  }
  for (let j = 0; j < cols; j += 1) {
    matrix[0][j] = j;
  }

  for (let i = 1; i < rows; i += 1) {
    for (let j = 1; j < cols; j += 1) {
      const substitutionCost = a[i - 1] === b[j - 1] ? 0 : 1;
      matrix[i][j] = Math.min(
        matrix[i - 1][j] + 1,
        matrix[i][j - 1] + 1,
        matrix[i - 1][j - 1] + substitutionCost,
      );
    }
  }

  return matrix[a.length][b.length];
}

function normalizedAliasWindow(tokens: string[]): string {
  return tokens.map((token) => normalizeDictionaryTerm(token)).join(" ");
}

function toCollapsed(normalizedValue: string): string {
  return normalizedValue.replace(/\s+/g, "");
}

function knownTerms(entry: CustomWordEntry): Set<string> {
  const terms = new Set<string>();
  const canonical = normalizeDictionaryTerm(entry.input);
  if (canonical) {
    terms.add(canonical);
  }
  for (const alias of normalizeAliases(entry.aliases ?? [], entry.input)) {
    terms.add(normalizeDictionaryTerm(alias));
  }
  return terms;
}

function shouldSkipByLength(
  candidateCollapsed: string,
  targetCollapsed: string,
): boolean {
  if (candidateCollapsed.length < MIN_COLLAPSED_LEN) {
    return true;
  }
  const lenRatio = candidateCollapsed.length / targetCollapsed.length;
  return lenRatio < MIN_LEN_RATIO || lenRatio > MAX_LEN_RATIO;
}

function isDifficultSplitCandidate(
  candidateTokens: string[],
  candidateCollapsed: string,
  targetCollapsed: string,
  score: number,
): boolean {
  if (candidateTokens.length !== 2) {
    return false;
  }
  if (score > DIFFICULT_SPLIT_MAX_DISTANCE_SCORE) {
    return false;
  }

  const [firstToken, secondToken] = candidateTokens;
  if (firstToken.length < 4 || secondToken.length > 2) {
    return false;
  }

  const prefixLen = sharedPrefixLength(candidateCollapsed, targetCollapsed);
  if (prefixLen < 3) {
    return false;
  }

  return (
    candidateCollapsed[candidateCollapsed.length - 1] ===
    targetCollapsed[targetCollapsed.length - 1]
  );
}

export function suggestAliasFromTranscript(
  transcript: string,
  dictionaryEntries: CustomWordEntry[],
): AliasSuggestion | null {
  if (!transcript.trim() || dictionaryEntries.length === 0) {
    return null;
  }

  const tokens = tokenizeTranscript(transcript);
  if (tokens.length < 2) {
    return null;
  }
  const normalizedTokenSet = new Set(
    tokens.map((token) => normalizeDictionaryTerm(token)),
  );

  let best: AliasSuggestion | null = null;
  let bestWeightedScore = Number.POSITIVE_INFINITY;

  for (
    let entryIndex = 0;
    entryIndex < dictionaryEntries.length;
    entryIndex += 1
  ) {
    const entry = dictionaryEntries[entryIndex];
    const canonicalNormalized = normalizeDictionaryTerm(entry.input);
    if (!canonicalNormalized || canonicalNormalized.includes(" ")) {
      continue;
    }
    if (normalizedTokenSet.has(canonicalNormalized)) {
      // Canonical term already appears in transcript as intended.
      continue;
    }

    const canonicalCollapsed = toCollapsed(canonicalNormalized);
    if (canonicalCollapsed.length < MIN_COLLAPSED_LEN) {
      continue;
    }

    const existingTerms = knownTerms(entry);

    for (const windowSize of SPLIT_WINDOW_SIZES) {
      if (tokens.length < windowSize) {
        continue;
      }

      for (let start = 0; start <= tokens.length - windowSize; start += 1) {
        const windowTokens = tokens.slice(start, start + windowSize);
        const normalizedCandidate = normalizedAliasWindow(windowTokens);
        if (!normalizedCandidate) {
          continue;
        }

        const candidateTokens = normalizedCandidate.split(" ");
        if (candidateTokens.every((token) => GUARD_WORDS.has(token))) {
          continue;
        }
        if (
          GUARD_WORDS.has(candidateTokens[0]) ||
          GUARD_WORDS.has(candidateTokens[candidateTokens.length - 1])
        ) {
          continue;
        }
        if (candidateTokens.includes(canonicalNormalized)) {
          continue;
        }

        if (existingTerms.has(normalizedCandidate)) {
          continue;
        }

        const candidateCollapsed = toCollapsed(normalizedCandidate);
        if (shouldSkipByLength(candidateCollapsed, canonicalCollapsed)) {
          continue;
        }

        const distance = levenshteinDistance(
          candidateCollapsed,
          canonicalCollapsed,
        );
        const score =
          distance /
          Math.max(candidateCollapsed.length, canonicalCollapsed.length);
        const difficultSplitMatch = isDifficultSplitCandidate(
          candidateTokens,
          candidateCollapsed,
          canonicalCollapsed,
          score,
        );
        if (!difficultSplitMatch && score > MAX_DISTANCE_SCORE) {
          continue;
        }

        if (
          candidateCollapsed[0] !== canonicalCollapsed[0] &&
          score > FIRST_CHAR_MISMATCH_HARD_LIMIT &&
          !difficultSplitMatch
        ) {
          continue;
        }

        // Slightly favor 3-token windows for split-word terms.
        const weightedScore =
          score +
          (windowSize === 2 ? 0.015 : 0) +
          (difficultSplitMatch ? 0.08 : 0);
        if (weightedScore < bestWeightedScore) {
          bestWeightedScore = weightedScore;
          best = {
            entryIndex,
            entryIdentity: dictionaryEntryIdentity(entry),
            entryInput: entry.input,
            alias: normalizedCandidate,
            score,
          };
        }
      }
    }
  }

  return best;
}
