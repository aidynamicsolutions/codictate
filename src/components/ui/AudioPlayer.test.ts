import { describe, expect, it } from "vitest";

import {
  shouldBlockAudioSeek,
  shouldBlockPlaybackToggle,
} from "@/components/ui/AudioPlayer";

describe("AudioPlayer interaction guards", () => {
  const mockAudio = {} as HTMLAudioElement;

  it("blocks playback toggle when disabled", () => {
    expect(shouldBlockPlaybackToggle(true, false, mockAudio)).toBe(true);
  });

  it("blocks playback toggle while loading", () => {
    expect(shouldBlockPlaybackToggle(false, true, mockAudio)).toBe(true);
  });

  it("blocks playback toggle when audio element is unavailable", () => {
    expect(shouldBlockPlaybackToggle(false, false, null)).toBe(true);
  });

  it("allows playback toggle only when enabled, not loading, and audio is present", () => {
    expect(shouldBlockPlaybackToggle(false, false, mockAudio)).toBe(false);
  });

  it("blocks seek interactions when disabled", () => {
    expect(shouldBlockAudioSeek(true)).toBe(true);
    expect(shouldBlockAudioSeek(false)).toBe(false);
  });
});
