import { describe, expect, it } from "vitest";
import { type HistoryEntry } from "@/bindings";
import {
  getHistoryCopyText,
  getHistoryPrimaryText,
  getHistoryRawText,
  isRawOnlyHistoryMatch,
  shouldShowOriginalTranscript,
} from "@/components/shared/historyDisplayUtils";

function buildEntry(overrides: Partial<HistoryEntry> = {}): HistoryEntry {
  return {
    id: 1,
    file_name: "codictate-1.wav",
    timestamp: 1_700_000_000,
    saved: false,
    title: "Recording",
    transcription_text: "raw transcript",
    post_processed_text: "refined transcript",
    inserted_text: "inserted transcript",
    effective_text: "inserted transcript",
    raw_text: "raw transcript",
    post_process_prompt: null,
    duration_ms: 2000,
    file_path: "/tmp/codictate-1.wav",
    ...overrides,
  };
}

describe("historyDisplayUtils", () => {
  it("uses inserted text as primary when available", () => {
    const entry = buildEntry();
    expect(getHistoryPrimaryText(entry)).toBe("inserted transcript");
  });

  it("falls back to post-processed text when inserted text is missing", () => {
    const entry = buildEntry({
      inserted_text: null,
      effective_text: "refined transcript",
    });
    expect(getHistoryPrimaryText(entry)).toBe("refined transcript");
  });

  it("falls back to raw text when inserted and post-processed text are missing", () => {
    const entry = buildEntry({
      inserted_text: null,
      post_processed_text: null,
      effective_text: "raw transcript",
    });
    expect(getHistoryPrimaryText(entry)).toBe("raw transcript");
    expect(getHistoryRawText(entry)).toBe("raw transcript");
  });

  it("shows original transcript toggle only when raw differs from primary", () => {
    const different = buildEntry();
    const identical = buildEntry({
      inserted_text: null,
      post_processed_text: null,
      effective_text: "raw transcript",
    });

    expect(shouldShowOriginalTranscript(different)).toBe(true);
    expect(shouldShowOriginalTranscript(identical)).toBe(false);
  });

  it("copies primary/effective text", () => {
    const entry = buildEntry({
      inserted_text: null,
      effective_text: "refined transcript",
    });
    expect(getHistoryCopyText(entry)).toBe("refined transcript");
  });

  it("flags raw-only search matches", () => {
    const entry = buildEntry({
      inserted_text: "final output",
      effective_text: "final output",
      raw_text: "debug token",
      transcription_text: "debug token",
    });

    expect(isRawOnlyHistoryMatch(entry, "debug token")).toBe(true);
    expect(isRawOnlyHistoryMatch(entry, "final output")).toBe(false);
    expect(isRawOnlyHistoryMatch(entry, "")).toBe(false);
  });
});
