import React from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/shared/ui/button";
import OnboardingLayout from "./OnboardingLayout";

interface SetupStepProps {
  onContinue: () => void;
}

export const SetupStep: React.FC<SetupStepProps> = ({ onContinue }) => {
  const { t } = useTranslation();

  return (
    <OnboardingLayout
      currentStep="setup"
      leftContent={
        <div className="flex flex-col gap-6">
          <div className="flex flex-col gap-2">
            <h1 className="text-3xl font-semibold tracking-tight text-foreground lg:text-4xl">
              {t("onboarding.setup.title")}
            </h1>
            <p className="text-muted-foreground">
              {t("onboarding.setup.subtitle")}
            </p>
          </div>

          {/* Placeholder for setup content */}
          <div className="rounded-lg border border-border bg-accent/50 p-6">
            <p className="text-sm text-muted-foreground">
              {t("onboarding.setup.placeholder")}
            </p>
          </div>

          <Button onClick={onContinue} size="lg" className="w-fit">
            {t("onboarding.setup.continue")}
          </Button>
        </div>
      }
      rightContent={
        <img
          src="/src-tauri/resources/svg/undraw_welcome-cats_tw36.svg"
          alt="Setup illustration"
          className="h-auto max-h-[400px] w-auto max-w-full object-contain"
        />
      }
    />
  );
};

export default SetupStep;
