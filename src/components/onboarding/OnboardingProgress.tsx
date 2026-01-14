import React from "react";
import { ChevronRight } from "lucide-react";
import { useTranslation } from "react-i18next";

export type OnboardingStep = "welcome" | "attribution" | "tellUsAboutYou" | "permissions" | "setup" | "learn";

interface OnboardingProgressProps {
  currentStep: OnboardingStep;
}

const STEPS: OnboardingStep[] = ["welcome", "permissions", "setup", "learn"];

export const OnboardingProgress: React.FC<OnboardingProgressProps> = ({
  currentStep,
}) => {
  const { t } = useTranslation();

  const getStepIndex = (step: OnboardingStep) => {
    // Map "attribution" and "tellUsAboutYou" to "welcome" for progress display (they are part of welcome visually)
    if (step === "attribution" || step === "tellUsAboutYou") return 0;
    return STEPS.indexOf(step);
  };

  const currentIndex = getStepIndex(currentStep);

  return (
    <div className="flex items-center justify-center gap-2 py-4">
      {STEPS.map((step, index) => {
        const isActive = index === currentIndex;
        const isPast = index < currentIndex;

        return (
          <React.Fragment key={step}>
            <span
              className={`text-sm font-medium uppercase tracking-wide transition-colors ${
                isActive
                  ? "text-foreground"
                  : isPast
                    ? "text-muted-foreground"
                    : "text-muted-foreground/50"
              }`}
            >
              {t(`onboarding.progress.${step}`)}
            </span>
            {index < STEPS.length - 1 && (
              <ChevronRight className="h-4 w-4 text-muted-foreground/50" />
            )}
          </React.Fragment>
        );
      })}
    </div>
  );
};

export default OnboardingProgress;
