import { describe, expect, it } from "vitest";

import { resolveOverlayVisualVariant } from "@/overlay/overlayVisualState";

describe("resolveOverlayVisualVariant", () => {
  it("maps connecting state to the neutral pre-ready shell", () => {
    expect(resolveOverlayVisualVariant("connecting", false, false)).toBe(
      "pre_ready_shell",
    );
  });

  it("keeps recording bars as the first ready-to-speak visual", () => {
    expect(resolveOverlayVisualVariant("recording", false, false)).toBe("bars");
  });

  it("shows status messaging for transcribing and processing states", () => {
    expect(resolveOverlayVisualVariant("transcribing", false, false)).toBe(
      "status_message",
    );
    expect(resolveOverlayVisualVariant("processing", false, false)).toBe(
      "status_message",
    );
  });

  it("prioritizes undo card presentation over all operation variants", () => {
    expect(resolveOverlayVisualVariant("recording", true, false)).toBe(
      "status_message",
    );
  });

  it("shows correction layout only when correction data exists", () => {
    expect(resolveOverlayVisualVariant("correcting", false, true)).toBe(
      "correction",
    );
    expect(resolveOverlayVisualVariant("correcting", false, false)).toBe(
      "status_message",
    );
  });
});
