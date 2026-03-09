export type OnboardingStep =
  | "welcome"
  | "attribution"
  | "tellUsAboutYou"
  | "typingUseCases"
  | "permissions"
  | "downloadModel"
  | "microphoneCheck"
  | "hotkeySetup"
  | "languageSelect"
  | "learn"
  | "success"
  | "referral";

export type OnboardingAnalyticsStep =
  | "welcome"
  | "attribution"
  | "tell_us_about_you"
  | "typing_use_cases"
  | "permissions"
  | "download_model"
  | "microphone_check"
  | "hotkey_setup"
  | "language_select"
  | "learn"
  | "success"
  | "referral";

export type OnboardingAnalyticsPhase =
  | "profile"
  | "setup"
  | "activation"
  | "celebration"
  | "growth";

export const CORE_ONBOARDING_STEPS: OnboardingStep[] = [
  "welcome",
  "attribution",
  "tellUsAboutYou",
  "typingUseCases",
  "permissions",
  "downloadModel",
  "microphoneCheck",
  "hotkeySetup",
  "languageSelect",
  "learn",
];

export const ALL_ONBOARDING_STEPS: OnboardingStep[] = [
  ...CORE_ONBOARDING_STEPS,
  "success",
  "referral",
];

const STEP_TO_ANALYTICS_STEP: Record<OnboardingStep, OnboardingAnalyticsStep> = {
  welcome: "welcome",
  attribution: "attribution",
  tellUsAboutYou: "tell_us_about_you",
  typingUseCases: "typing_use_cases",
  permissions: "permissions",
  downloadModel: "download_model",
  microphoneCheck: "microphone_check",
  hotkeySetup: "hotkey_setup",
  languageSelect: "language_select",
  learn: "learn",
  success: "success",
  referral: "referral",
};

const STEP_TO_PHASE: Record<OnboardingStep, OnboardingAnalyticsPhase> = {
  welcome: "profile",
  attribution: "profile",
  tellUsAboutYou: "profile",
  typingUseCases: "profile",
  permissions: "setup",
  downloadModel: "setup",
  microphoneCheck: "setup",
  hotkeySetup: "setup",
  languageSelect: "setup",
  learn: "activation",
  success: "celebration",
  referral: "growth",
};

export function toAnalyticsOnboardingStep(
  step: OnboardingStep,
): OnboardingAnalyticsStep {
  return STEP_TO_ANALYTICS_STEP[step];
}

export function getOnboardingPhase(
  step: OnboardingStep,
): OnboardingAnalyticsPhase {
  return STEP_TO_PHASE[step];
}

export function getCoreOnboardingStepPosition(step: OnboardingStep): number {
  const index = CORE_ONBOARDING_STEPS.indexOf(step);
  if (index >= 0) {
    return index + 1;
  }

  return CORE_ONBOARDING_STEPS.length;
}
