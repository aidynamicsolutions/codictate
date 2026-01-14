import React, { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/shared/ui/button";
import { Input } from "@/components/shared/ui/input";
import OnboardingLayout from "./OnboardingLayout";
import { useSettings } from "@/hooks/useSettings";
import { MAX_INPUT_LENGTH } from "@/constants";

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

  const handleNameChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value.slice(0, MAX_INPUT_LENGTH);
    setName(value);
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
            <div className="relative">
              <Input
                type="text"
                placeholder={t("onboarding.welcome.namePlaceholder")}
                value={name}
                onChange={handleNameChange}
                onKeyDown={handleKeyDown}
                className="h-12 text-base"
                maxLength={MAX_INPUT_LENGTH}
                autoFocus
              />
              <span className="absolute right-3 top-1/2 -translate-y-1/2 text-xs text-muted-foreground">
                {name.length}/{MAX_INPUT_LENGTH}
              </span>
            </div>

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
