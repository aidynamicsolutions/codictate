import React from "react";
import { useTranslation } from "react-i18next";
import {
  CORE_ONBOARDING_STEPS,
  getCoreOnboardingStepPosition,
  type OnboardingStep,
  toAnalyticsOnboardingStep,
} from "./flow";

interface OnboardingProgressProps {
  currentStep: OnboardingStep;
  /** Optional trailing element (e.g. language switcher) rendered inline after the step label. */
  trailing?: React.ReactNode;
}

export const OnboardingProgress: React.FC<OnboardingProgressProps> = ({
  currentStep,
  trailing,
}) => {
  const { t } = useTranslation();
  const currentPosition = getCoreOnboardingStepPosition(currentStep);
  const progressPercentage =
    (currentPosition / CORE_ONBOARDING_STEPS.length) * 100;
  const labelKey = toAnalyticsOnboardingStep(currentStep);

  return (
    <div className="flex flex-col gap-2 px-6 py-4">
      <div className="flex items-center gap-4">
        <span className="text-sm font-medium text-foreground">
          {t("onboarding.progress.stepCounter", {
            current: currentPosition,
            total: CORE_ONBOARDING_STEPS.length,
          })}
        </span>
        <span className="ml-auto text-sm font-medium text-foreground">
          {t(`onboarding.progress.steps.${labelKey}`)}
        </span>
        {trailing && (
          <span className="shrink-0">{trailing}</span>
        )}
      </div>
      <div className="h-2 w-full overflow-hidden rounded-full bg-muted">
        <div
          className="h-full rounded-full bg-primary transition-[width] duration-300 ease-out"
          style={{ width: `${progressPercentage}%` }}
        />
      </div>
    </div>
  );
};

export default OnboardingProgress;
export type { OnboardingStep } from "./flow";
