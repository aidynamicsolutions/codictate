import { describe, it, expect } from "vitest";
import {
  deriveIntentFromEntry,
  isDuplicateEntry,
  isReplacementOutputValid,
  isShortSingleWordFuzzyBlocked,
  normalizeAliases,
  normalizeDictionaryForMatching,
} from "./dictionaryUtils";
import type { CustomWordEntry } from "@/bindings";

// Helper to create a CustomWordEntry for tests
function createEntry(
  input: string,
  replacement?: string,
  isReplacement = false,
  aliases: string[] = [],
  fuzzyEnabled?: boolean | null,
): CustomWordEntry {
  return {
    input,
    aliases,
    replacement: replacement ?? input,
    is_replacement: isReplacement,
    fuzzy_enabled: fuzzyEnabled ?? null,
  };
}

describe("isDuplicateEntry", () => {
  describe("basic duplicate detection", () => {
    it("should return false for empty input", () => {
      const entries = [createEntry("test")];
      expect(isDuplicateEntry("", [], entries)).toBe(false);
      expect(isDuplicateEntry("   ", [], entries)).toBe(false);
    });

    it("should return false when no entries exist", () => {
      expect(isDuplicateEntry("test", [], [])).toBe(false);
    });

    it("should detect exact match as duplicate", () => {
      const entries = [createEntry("hello"), createEntry("world")];
      expect(isDuplicateEntry("hello", [], entries)).toBe(true);
      expect(isDuplicateEntry("world", [], entries)).toBe(true);
    });

    it("should return false for non-matching input", () => {
      const entries = [createEntry("hello"), createEntry("world")];
      expect(isDuplicateEntry("foo", [], entries)).toBe(false);
    });
  });

  describe("case-insensitive matching", () => {
    it("should detect case variations as duplicates", () => {
      const entries = [createEntry("Hello")];
      expect(isDuplicateEntry("hello", [], entries)).toBe(true);
      expect(isDuplicateEntry("HELLO", [], entries)).toBe(true);
      expect(isDuplicateEntry("hElLo", [], entries)).toBe(true);
    });

    it("should detect duplicate regardless of entry case", () => {
      const entries = [createEntry("BTW", "by the way", true)];
      expect(isDuplicateEntry("btw", [], entries)).toBe(true);
      expect(isDuplicateEntry("Btw", [], entries)).toBe(true);
    });
  });

  describe("whitespace handling", () => {
    it("should trim whitespace from input", () => {
      const entries = [createEntry("test")];
      expect(isDuplicateEntry("  test  ", [], entries)).toBe(true);
      expect(isDuplicateEntry("\ttest\n", [], entries)).toBe(true);
    });

    it("should match phrases with spaces", () => {
      const entries = [createEntry("my email", "john@example.com", true)];
      expect(isDuplicateEntry("my email", [], entries)).toBe(true);
      expect(isDuplicateEntry("MY EMAIL", [], entries)).toBe(true);
    });

    it("should treat repeated spaces as duplicates", () => {
      const entries = [createEntry("my email", "john@example.com", true)];
      expect(isDuplicateEntry("my   email", [], entries)).toBe(true);
      expect(isDuplicateEntry("my email", ["my   email"], entries)).toBe(true);
    });
  });

  describe("exclude entry for editing", () => {
    it("should not flag the original entry when editing", () => {
      const existingEntry = createEntry("hello");
      const entries = [existingEntry, createEntry("world")];

      // When editing "hello", it shouldn't be flagged as duplicate
      expect(isDuplicateEntry("hello", [], entries, existingEntry)).toBe(false);
      expect(isDuplicateEntry("HELLO", [], entries, existingEntry)).toBe(false);
    });

    it("should still flag other duplicates when editing", () => {
      const existingEntry = createEntry("hello");
      const entries = [existingEntry, createEntry("world")];

      // When editing "hello", changing it to "world" should be flagged
      expect(isDuplicateEntry("world", [], entries, existingEntry)).toBe(true);
    });

    it("should allow changing case of the same entry", () => {
      const existingEntry = createEntry("hello");
      const entries = [existingEntry];

      // Changing "hello" to "Hello" should be allowed
      expect(isDuplicateEntry("Hello", [], entries, existingEntry)).toBe(false);
    });
  });

  describe("alias collisions", () => {
    it("should detect collision with existing alias", () => {
      const entries = [createEntry("shadcn", "shadcn", false, ["shad cn"])];
      expect(isDuplicateEntry("chatgpt", ["shad cn"], entries)).toBe(true);
    });

    it("should detect collision when input matches existing alias", () => {
      const entries = [createEntry("shadcn", "shadcn", false, ["shad c n"])];
      expect(isDuplicateEntry("shad c n", [], entries)).toBe(true);
    });

    it("should ignore aliases that normalize to canonical input", () => {
      const entries = [createEntry("shadcn")];
      expect(isDuplicateEntry("shadcn", ["Shadcn", "shadcn"], entries)).toBe(
        true,
      );
      expect(
        normalizeAliases(["Shadcn", "shadcn", "shad cn"], "shadcn"),
      ).toEqual(["shad cn"]);
    });
  });

  describe("edge cases", () => {
    it("should handle entries with different is_replacement flags", () => {
      const entries = [
        createEntry("test", "test", false), // vocabulary
        createEntry("btw", "by the way", true), // replacement
      ];
      expect(isDuplicateEntry("test", [], entries)).toBe(true);
      expect(isDuplicateEntry("btw", [], entries)).toBe(true);
    });

    it("should handle unicode characters", () => {
      const entries = [createEntry("café")];
      expect(isDuplicateEntry("café", [], entries)).toBe(true);
      expect(isDuplicateEntry("CAFÉ", [], entries)).toBe(true);
    });

    it("should handle special characters", () => {
      const entries = [createEntry("C++")];
      expect(isDuplicateEntry("C++", [], entries)).toBe(true);
      expect(isDuplicateEntry("c++", [], entries)).toBe(true);
    });

    it("should handle empty exclude entry", () => {
      const entries = [createEntry("test")];
      expect(isDuplicateEntry("test", [], entries, undefined)).toBe(true);
    });

    it("should handle large number of entries", () => {
      const entries = Array.from({ length: 1000 }, (_, i) =>
        createEntry(`word${i}`),
      );
      expect(isDuplicateEntry("word500", [], entries)).toBe(true);
      expect(isDuplicateEntry("notfound", [], entries)).toBe(false);
    });
  });
});

describe("normalizeDictionaryForMatching", () => {
  it("expands symbols and strips punctuation", () => {
    expect(normalizeDictionaryForMatching("C++")).toBe("cplusplus");
    expect(normalizeDictionaryForMatching("Node.js")).toBe("nodejs");
    expect(normalizeDictionaryForMatching("R&D")).toBe("randd");
  });

  it("keeps unicode letters and numbers", () => {
    expect(normalizeDictionaryForMatching("猫咪")).toBe("猫咪");
    expect(normalizeDictionaryForMatching("CAFÉ")).toBe("café");
  });
});

describe("isShortSingleWordFuzzyBlocked", () => {
  it("blocks short ascii single-word terms", () => {
    expect(isShortSingleWordFuzzyBlocked("qwen")).toBe(true);
  });

  it("blocks short non-ascii single-word terms by character length", () => {
    expect(isShortSingleWordFuzzyBlocked("猫咪")).toBe(true);
  });

  it("does not block long single-word terms", () => {
    expect(isShortSingleWordFuzzyBlocked("anthropic")).toBe(false);
    expect(isShortSingleWordFuzzyBlocked("C++")).toBe(false);
  });

  it("does not block multi-word terms, even when normalized length is short", () => {
    expect(isShortSingleWordFuzzyBlocked("c l i")).toBe(false);
  });
});

describe("deriveIntentFromEntry", () => {
  it("defaults to recognize when entry is undefined", () => {
    expect(deriveIntentFromEntry()).toBe("recognize");
  });

  it("returns recognize for non-replacement entries", () => {
    expect(deriveIntentFromEntry(createEntry("ChatGPT"))).toBe("recognize");
  });

  it("returns replace for explicit replacement entries", () => {
    expect(
      deriveIntentFromEntry(
        createEntry("btw", "by the way", true, ["by the way shortcut"]),
      ),
    ).toBe("replace");
  });

  it("returns replace when replacement differs from input", () => {
    expect(deriveIntentFromEntry(createEntry("my email", "john@example.com"))).toBe(
      "replace",
    );
  });
});

describe("isReplacementOutputValid", () => {
  it("rejects empty input or output", () => {
    expect(isReplacementOutputValid("", "by the way")).toBe(false);
    expect(isReplacementOutputValid("btw", "")).toBe(false);
  });

  it("rejects output equal to input after trimming", () => {
    expect(isReplacementOutputValid("btw", "btw")).toBe(false);
    expect(isReplacementOutputValid("btw", " btw ")).toBe(false);
  });

  it("accepts output that differs from input", () => {
    expect(isReplacementOutputValid("btw", "by the way")).toBe(true);
  });

  it("treats case-only differences as valid output", () => {
    expect(isReplacementOutputValid("chatgpt", "ChatGPT")).toBe(true);
  });
});
