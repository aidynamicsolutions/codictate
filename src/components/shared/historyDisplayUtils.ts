import { type HistoryEntry } from "@/bindings";

function normalizedContains(text: string, query: string): boolean {
  if (!query.trim()) return false;
  return text.toLowerCase().includes(query.toLowerCase());
}

export function getHistoryPrimaryText(entry: HistoryEntry): string {
  return (
    entry.effective_text ??
    entry.inserted_text ??
    entry.post_processed_text ??
    entry.transcription_text
  );
}

export function getHistoryRawText(entry: HistoryEntry): string {
  return entry.raw_text ?? entry.transcription_text;
}

export function shouldShowOriginalTranscript(entry: HistoryEntry): boolean {
  return getHistoryRawText(entry) !== getHistoryPrimaryText(entry);
}

export function isRawOnlyHistoryMatch(
  entry: HistoryEntry,
  query: string,
): boolean {
  const primary = getHistoryPrimaryText(entry);
  const raw = getHistoryRawText(entry);
  return normalizedContains(raw, query) && !normalizedContains(primary, query);
}

export function getHistoryCopyText(entry: HistoryEntry): string {
  return getHistoryPrimaryText(entry);
}
