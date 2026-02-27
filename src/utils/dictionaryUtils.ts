import type { CustomWordEntry } from "@/bindings";

function normalizeSpaces(value: string): string {
  return value.trim().replace(/\s+/g, " ");
}

function trimValue(value: string): string {
  return value.trim();
}

const DICTIONARY_SHORT_TARGET_FUZZY_BLOCK_LEN = 4;
const ALPHANUMERIC_CHAR_RE = /[\p{L}\p{N}]/u;
const WHITESPACE_CHAR_RE = /\s/u;

export type DictionaryEntryIntent = "recognize" | "replace";

export function normalizeDictionaryTerm(value: string): string {
  return normalizeSpaces(value).toLowerCase();
}

export function normalizeDictionaryForMatching(value: string): string {
  const lower = value.toLowerCase();
  let expanded = "";

  for (const ch of lower) {
    switch (ch) {
      case "+":
        expanded += " plus ";
        break;
      case "#":
        expanded += " sharp ";
        break;
      case "&":
        expanded += " and ";
        break;
      default:
        if (ALPHANUMERIC_CHAR_RE.test(ch) || WHITESPACE_CHAR_RE.test(ch)) {
          expanded += ch;
        } else {
          expanded += " ";
        }
        break;
    }
  }

  return [...expanded].filter((ch) => ALPHANUMERIC_CHAR_RE.test(ch)).join("");
}

export function isShortSingleWordFuzzyBlocked(input: string): boolean {
  const wordCount = normalizeSpaces(input).split(" ").filter(Boolean).length;
  if (wordCount !== 1) {
    return false;
  }
  return (
    [...normalizeDictionaryForMatching(input)].length <=
    DICTIONARY_SHORT_TARGET_FUZZY_BLOCK_LEN
  );
}

export function deriveIntentFromEntry(
  entry?: CustomWordEntry,
): DictionaryEntryIntent {
  if (!entry) {
    return "recognize";
  }

  if (entry.is_replacement) {
    return "replace";
  }

  const trimmedInput = trimValue(entry.input);
  const trimmedReplacement = trimValue(entry.replacement);
  if (
    trimmedReplacement.length > 0 &&
    trimmedReplacement !== trimmedInput
  ) {
    return "replace";
  }

  return "recognize";
}

export function isReplacementOutputValid(
  input: string,
  replacement: string,
): boolean {
  const trimmedInput = trimValue(input);
  const trimmedReplacement = trimValue(replacement);

  if (trimmedInput.length === 0 || trimmedReplacement.length === 0) {
    return false;
  }

  return trimmedInput !== trimmedReplacement;
}

export function normalizeAliases(aliases: string[], input: string): string[] {
  const canonical = normalizeDictionaryTerm(input);
  const seen = new Set<string>();
  const normalizedAliases: string[] = [];

  for (const alias of aliases) {
    const cleaned = normalizeSpaces(alias);
    const normalized = cleaned.toLowerCase();
    if (!cleaned || normalized === canonical || seen.has(normalized)) {
      continue;
    }
    seen.add(normalized);
    normalizedAliases.push(cleaned);
  }

  return normalizedAliases;
}

function normalizedEntryTerms(entry: CustomWordEntry): Set<string> {
  const terms = new Set<string>();
  const inputTerm = normalizeDictionaryTerm(entry.input);
  if (inputTerm) {
    terms.add(inputTerm);
  }
  for (const alias of normalizeAliases(entry.aliases ?? [], entry.input)) {
    terms.add(normalizeDictionaryTerm(alias));
  }
  return terms;
}

export function dictionaryEntryIdentity(entry: CustomWordEntry): string {
  const aliases = normalizeAliases(entry.aliases ?? [], entry.input)
    .map((alias) => normalizeDictionaryTerm(alias))
    .sort()
    .join("|");
  return `${normalizeDictionaryTerm(entry.input)}::${normalizeDictionaryTerm(entry.replacement)}::${entry.is_replacement}::${aliases}`;
}

/**
 * Check if a dictionary entry input/alias already exists (case-insensitive match)
 *
 * @param input - Canonical input to check for duplicates
 * @param aliases - Aliases to check for duplicates
 * @param existingEntries - Array of existing dictionary entries
 * @param excludeEntry - Optional entry to exclude from comparison (for edit mode)
 * @returns true if any term conflicts with existing canonical inputs or aliases
 */
export function isDuplicateEntry(
  input: string,
  aliases: string[],
  existingEntries: CustomWordEntry[],
  excludeEntry?: CustomWordEntry,
): boolean {
  const normalizedInput = normalizeDictionaryTerm(input);
  if (!normalizedInput) return false;
  const candidateTerms = new Set<string>([
    normalizedInput,
    ...normalizeAliases(aliases, input).map((alias) =>
      normalizeDictionaryTerm(alias),
    ),
  ]);
  const excludedIdentity = excludeEntry
    ? dictionaryEntryIdentity(excludeEntry)
    : null;

  return existingEntries.some((entry) => {
    if (
      excludeEntry &&
      (entry === excludeEntry ||
        dictionaryEntryIdentity(entry) === excludedIdentity)
    ) {
      return false;
    }
    const terms = normalizedEntryTerms(entry);
    for (const term of candidateTerms) {
      if (terms.has(term)) {
        return true;
      }
    }
    return false;
  });
}
