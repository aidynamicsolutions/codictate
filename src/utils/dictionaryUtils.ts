import type { CustomWordEntry } from "@/bindings";

function normalizeSpaces(value: string): string {
  return value.trim().replace(/\s+/g, " ");
}

export function normalizeDictionaryTerm(value: string): string {
  return normalizeSpaces(value).toLowerCase();
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
