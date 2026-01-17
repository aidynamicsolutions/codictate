import React from "react";
import { useTranslation } from "react-i18next";
import { ArrowLeft } from "lucide-react";
import { Button } from "@/components/shared/ui/button";
import OnboardingLayout from "./OnboardingLayout";

interface HotkeySetupStepProps {
  onContinue: () => void;
  onBack: () => void;
}

export const HotkeySetupStep: React.FC<HotkeySetupStepProps> = ({
  onContinue,
  onBack,
}) => {
  const { t } = useTranslation();

  return (
    <OnboardingLayout
      currentStep="hotkeySetup"
      leftContent={
        <div className="flex flex-col gap-6">
          {/* Back button */}
          <button
            type="button"
            onClick={onBack}
            className="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground transition-colors w-fit"
          >
            <ArrowLeft className="h-4 w-4" />
            {t("onboarding.hotkeySetup.back")}
          </button>

          <div className="flex flex-col gap-2">
            <h1 className="text-3xl font-semibold tracking-tight text-foreground lg:text-4xl max-w-[380px]">
              {t("onboarding.hotkeySetup.title")}
            </h1>
            <p className="text-muted-foreground">
              {t("onboarding.hotkeySetup.subtitle")}
            </p>
          </div>

          {/* Placeholder for hotkey configuration */}
          <div className="rounded-lg border border-border bg-accent/50 p-6">
            <p className="text-sm text-muted-foreground">
              {t("onboarding.hotkeySetup.placeholder")}
            </p>
          </div>

          <Button onClick={onContinue} size="lg" className="w-fit">
            {t("onboarding.hotkeySetup.continue")}
          </Button>
        </div>
      }
      rightContent={
        <img
          src="/src-tauri/resources/svg/undraw_welcome-cats_tw36.svg"
          alt="Hotkey setup illustration"
          className="h-auto max-h-[400px] w-auto max-w-full object-contain"
        />
      }
    />
  );
};

export default HotkeySetupStep;
