import { describe, expect, it, vi } from "vitest";

import { recordOnboardingActivationAttempt } from "./onboardingRuntime";

describe("recordOnboardingActivationAttempt", () => {
  it("logs and stays recoverable when the activation command throws", async () => {
    const setLearnActivated = vi.fn();
    const markActivationRecorded = vi.fn();
    const trackLearnCompleted = vi.fn(async () => {});
    const logError = vi.fn();

    await recordOnboardingActivationAttempt({
      activationRecorded: false,
      surface: "learn_mock_chat",
      recordActivation: vi.fn(async () => {
        throw new Error("invoke failed");
      }),
      setLearnActivated,
      markActivationRecorded,
      trackLearnCompleted,
      logError,
    });

    expect(markActivationRecorded).not.toHaveBeenCalled();
    expect(setLearnActivated).not.toHaveBeenCalled();
    expect(trackLearnCompleted).not.toHaveBeenCalled();
    expect(logError).toHaveBeenCalledWith(
      expect.stringContaining("event=onboarding_activation_record_failed surface=learn_mock_chat error=Error: invoke failed"),
    );
  });

  it("marks activation and tracks learn completion after a successful first activation", async () => {
    const setLearnActivated = vi.fn();
    const markActivationRecorded = vi.fn();
    const trackLearnCompleted = vi.fn(async () => {});
    const logError = vi.fn();

    await recordOnboardingActivationAttempt({
      activationRecorded: false,
      surface: "learn_mock_chat",
      recordActivation: vi.fn(async () => ({
        status: "ok" as const,
        data: true,
      })),
      setLearnActivated,
      markActivationRecorded,
      trackLearnCompleted,
      logError,
    });

    expect(markActivationRecorded).toHaveBeenCalledTimes(1);
    expect(setLearnActivated).toHaveBeenCalledWith(true);
    expect(trackLearnCompleted).toHaveBeenCalledTimes(1);
    expect(logError).not.toHaveBeenCalled();
  });
});
