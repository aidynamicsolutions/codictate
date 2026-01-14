import React, { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/shared/ui/button";
import { Input } from "@/components/shared/ui/input";
import OnboardingLayout from "./OnboardingLayout";
import { useSettings } from "@/hooks/useSettings";

interface WelcomeStepProps {
  onContinue: (name: string) => void;
  initialName?: string;
}

export const WelcomeStep: React.FC<WelcomeStepProps> = ({
  onContinue,
  initialName = "",
}) => {
  const { t } = useTranslation();
  const [name, setName] = useState(initialName);

  useEffect(() => {
    if (initialName) {
      setName(initialName);
    }
  }, [initialName]);

  const handleContinue = () => {
    onContinue(name.trim());
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      handleContinue();
    }
  };

  return (
    <OnboardingLayout
      currentStep="welcome"
      leftContent={
        <div className="flex flex-col gap-6">
          <div className="flex flex-col gap-2">
            <h1 className="text-3xl font-semibold tracking-tight text-foreground lg:text-4xl">
              {t("onboarding.welcome.title")}
            </h1>
            <p className="text-muted-foreground">
              {t("onboarding.welcome.subtitle")}
            </p>
          </div>

          <div className="flex flex-col gap-4">
            <Input
              type="text"
              placeholder={t("onboarding.welcome.namePlaceholder")}
              value={name}
              onChange={(e) => setName(e.target.value)}
              onKeyDown={handleKeyDown}
              className="h-12 text-base"
              autoFocus
            />

            <Button
              onClick={handleContinue}
              size="lg"
              className="w-fit"
            >
              {t("onboarding.welcome.continue")}
            </Button>
          </div>
        </div>
      }
      rightContent={
        <img
          src="/src-tauri/resources/svg/undraw_hey-by-basecamp_61xm.svg"
          alt="Welcome illustration"
          className="h-auto max-h-[400px] w-auto max-w-full object-contain"
        />
      }
    />
  );
};

export default WelcomeStep;
