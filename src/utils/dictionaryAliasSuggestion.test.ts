import { describe, expect, it } from "vitest";
import type { CustomWordEntry } from "@/bindings";
import { suggestAliasFromTranscript } from "./dictionaryAliasSuggestion";

function entry(
  input: string,
  aliases: string[] = [],
  replacement?: string,
  isReplacement = false,
): CustomWordEntry {
  return {
    input,
    aliases,
    replacement: replacement ?? input,
    is_replacement: isReplacement,
  };
}

describe("suggestAliasFromTranscript", () => {
  it("suggests exact split form for a single-token canonical term", () => {
    const suggestion = suggestAliasFromTranscript(
      "what is a shad cn component",
      [entry("shadcn")],
    );
    expect(suggestion).not.toBeNull();
    expect(suggestion?.entryInput).toBe("shadcn");
    expect(suggestion?.alias).toBe("shad cn");
    expect(suggestion?.score).toBe(0);
  });

  it("normalizes punctuation in split-token candidates", () => {
    const suggestion = suggestAliasFromTranscript("what is shad c n?", [
      entry("shadcn"),
    ]);
    expect(suggestion).not.toBeNull();
    expect(suggestion?.alias).toBe("shad c n");
  });

  it("does not suggest when canonical term already appears exactly", () => {
    const suggestion = suggestAliasFromTranscript(
      "create me a shadcn component",
      [entry("shadcn", ["shad cn"])],
    );
    expect(suggestion).toBeNull();
  });

  it("skips candidates that only add guard words around a known term", () => {
    const suggestion = suggestAliasFromTranscript(
      "this is a sat cn component",
      [entry("shadcn", ["sat cn"])],
    );
    expect(suggestion).toBeNull();
  });

  it("suggests close fuzzy split form", () => {
    const suggestion = suggestAliasFromTranscript(
      "this uses shaf c n component",
      [entry("shadcn")],
    );
    expect(suggestion).not.toBeNull();
    expect(suggestion?.entryInput).toBe("shadcn");
    expect(suggestion?.alias).toBe("shaf c n");
    expect(suggestion?.score ?? 1).toBeLessThanOrEqual(0.34);
  });

  it("rejects distant false positives", () => {
    const suggestion = suggestAliasFromTranscript(
      "what is a chef cn component",
      [entry("shadcn")],
    );
    expect(suggestion).toBeNull();
  });

  it("accepts difficult two-token variants with strong prefix evidence", () => {
    const suggestion = suggestAliasFromTranscript(
      "what is a shatsey n component",
      [entry("shadcn", ["shad cn"])],
    );
    expect(suggestion).not.toBeNull();
    expect(suggestion?.alias).toBe("shatsey n");
  });

  it("does not suggest aliases that already exist", () => {
    const suggestion = suggestAliasFromTranscript("what is shad cn component", [
      entry("shadcn", ["shad cn"]),
    ]);
    expect(suggestion).toBeNull();
  });

  it("ignores multi-word canonical entries for split-token suggestions", () => {
    const suggestion = suggestAliasFromTranscript(
      "chat gee pee tee for this task",
      [entry("chat gpt", ["chat gee pee tee"], "ChatGPT", true)],
    );
    expect(suggestion).toBeNull();
  });

  it("returns null for empty transcript", () => {
    const suggestion = suggestAliasFromTranscript("   ", [entry("shadcn")]);
    expect(suggestion).toBeNull();
  });
});
