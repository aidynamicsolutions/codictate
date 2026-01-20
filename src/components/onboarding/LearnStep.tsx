import React from "react";
import { useTranslation } from "react-i18next";
import { ArrowLeft } from "lucide-react";
import { Button } from "@/components/shared/ui/button";
import OnboardingLayout from "./OnboardingLayout";

interface LearnStepProps {
  onComplete: () => void;
  onBack?: () => void;
}

export const LearnStep: React.FC<LearnStepProps> = ({ onComplete, onBack }) => {
  const { t } = useTranslation();

  return (
    <OnboardingLayout
      currentStep="learn"
      leftContent={
        <div className="flex flex-col h-full">
          {/* Back button - positioned at top */}
          {onBack && (
            <button
              type="button"
              onClick={onBack}
              className="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground transition-colors w-fit mb-auto"
            >
              <ArrowLeft className="h-4 w-4" />
              {t("onboarding.learn.back")}
            </button>
          )}

          {/* Content centered vertically */}
          <div className="flex flex-col gap-6 my-auto">
            <div className="flex flex-col gap-2">
              <h1 className="text-3xl font-semibold tracking-tight text-foreground lg:text-4xl">
                {t("onboarding.learn.title")}
              </h1>
              <p className="text-muted-foreground">
                {t("onboarding.learn.subtitle")}
              </p>
            </div>

            {/* Placeholder for learn content */}
            <div className="rounded-lg border border-border bg-accent/50 p-6">
              <p className="text-sm text-muted-foreground">
                {t("onboarding.learn.placeholder")}
              </p>
            </div>
          </div>

          {/* Complete button at bottom */}
          <Button onClick={onComplete} size="lg" className="mt-auto w-fit">
            {t("onboarding.learn.complete")}
          </Button>
        </div>
      }
      rightContent={
        <img
          src="/src-tauri/resources/svg/undraw_welcome-cats_tw36.svg"
          alt="Learn illustration"
          className="h-auto max-h-[400px] w-auto max-w-full object-contain"
        />
      }
    />
  );
};

export default LearnStep;

