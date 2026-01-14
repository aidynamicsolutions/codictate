import React from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/shared/ui/button";
import OnboardingLayout from "./OnboardingLayout";

interface PermissionsStepProps {
  onContinue: () => void;
}

export const PermissionsStep: React.FC<PermissionsStepProps> = ({
  onContinue,
}) => {
  const { t } = useTranslation();

  return (
    <OnboardingLayout
      currentStep="permissions"
      leftContent={
        <div className="flex flex-col gap-6">
          <div className="flex flex-col gap-2">
            <h1 className="text-3xl font-semibold tracking-tight text-foreground lg:text-4xl">
              {t("onboarding.permissions.title")}
            </h1>
            <p className="text-muted-foreground">
              {t("onboarding.permissions.subtitle")}
            </p>
          </div>

          {/* Placeholder for permissions content */}
          <div className="rounded-lg border border-border bg-accent/50 p-6">
            <p className="text-sm text-muted-foreground">
              {t("onboarding.permissions.placeholder")}
            </p>
          </div>

          <Button onClick={onContinue} size="lg" className="w-fit">
            {t("onboarding.permissions.continue")}
          </Button>
        </div>
      }
      rightContent={
        <img
          src="/src-tauri/resources/svg/undraw_welcome-cats_tw36.svg"
          alt="Permissions illustration"
          className="h-auto max-h-[400px] w-auto max-w-full object-contain"
        />
      }
    />
  );
};

export default PermissionsStep;
