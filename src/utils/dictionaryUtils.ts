import type { CustomWordEntry } from "@/bindings";

/**
 * Check if a dictionary entry input already exists (case-insensitive match)
 * 
 * @param input - The input string to check for duplicates
 * @param existingEntries - Array of existing dictionary entries
 * @param excludeEntry - Optional entry to exclude from comparison (for edit mode)
 * @returns true if the input is a duplicate, false otherwise
 */
export function isDuplicateEntry(
  input: string,
  existingEntries: CustomWordEntry[],
  excludeEntry?: CustomWordEntry
): boolean {
  const normalizedInput = input.trim().toLowerCase();
  if (!normalizedInput) return false;

  return existingEntries.some((entry) => {
    // If editing, exclude the original entry from comparison
    if (excludeEntry && entry.input.toLowerCase() === excludeEntry.input.toLowerCase()) {
      return false;
    }
    return entry.input.toLowerCase() === normalizedInput;
  });
}
