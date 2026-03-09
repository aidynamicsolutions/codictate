import type { Result } from "@/bindings";

export type OnboardingActivationSurface = "learn_mock_chat" | "post_onboarding";

interface RecordOnboardingActivationArgs {
  activationRecorded: boolean;
  surface: OnboardingActivationSurface;
  recordActivation: (
    surface: OnboardingActivationSurface,
  ) => Promise<Result<boolean, string>>;
  setLearnActivated: (value: boolean) => void;
  markActivationRecorded: () => void;
  trackLearnCompleted: () => Promise<void>;
  logError: (message: string) => void;
}

export async function recordOnboardingActivationAttempt({
  activationRecorded,
  surface,
  recordActivation,
  setLearnActivated,
  markActivationRecorded,
  trackLearnCompleted,
  logError,
}: RecordOnboardingActivationArgs): Promise<void> {
  if (activationRecorded) {
    setLearnActivated(true);
    return;
  }

  try {
    const result = await recordActivation(surface);
    if (result.status === "error") {
      logError(
        `event=onboarding_activation_record_failed surface=${surface} error=${result.error}`,
      );
      return;
    }

    markActivationRecorded();
    setLearnActivated(true);

    if (result.data) {
      await trackLearnCompleted();
    }
  } catch (error) {
    logError(
      `event=onboarding_activation_record_failed surface=${surface} error=${error}`,
    );
  }
}
