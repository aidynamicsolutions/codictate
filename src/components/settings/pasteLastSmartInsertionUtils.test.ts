import { describe, expect, it } from "vitest";
import { isPasteLastSmartInsertionEnabled } from "./pasteLastSmartInsertionUtils";

describe("isPasteLastSmartInsertionEnabled", () => {
  it("returns true only for explicit true", () => {
    expect(isPasteLastSmartInsertionEnabled(true)).toBe(true);
    expect(isPasteLastSmartInsertionEnabled(false)).toBe(false);
  });

  it("treats undefined as disabled", () => {
    expect(isPasteLastSmartInsertionEnabled(undefined)).toBe(false);
  });

  it("treats null as disabled", () => {
    expect(isPasteLastSmartInsertionEnabled(null)).toBe(false);
  });
});
