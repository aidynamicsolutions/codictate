import React, { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/shared/ui/button";
import { Input } from "@/components/shared/ui/input";
import OnboardingLayout from "./OnboardingLayout";
import { MAX_INPUT_LENGTH } from "@/constants";

// Work roles
const WORK_ROLES = [
  "founder_ceo",
  "consultant",
  "operations",
  "developer",
  "product",
  "data_analysis",
  "sales",
  "marketing",
  "customer_support",
  "recruiting",
  "creator",
  "writer",
  "educator",
  "student",
  "legal",
  "healthcare",
  "other",
] as const;

// Roles that should NOT show the professional level question
const ROLES_WITHOUT_LEVEL = ["student", "writer", "customer_support", "other"];

// Professional levels
const PROFESSIONAL_LEVELS = [
  "executive",
  "director",
  "manager",
  "individual_contributor",
  "freelancer",
  "other",
] as const;

type WorkRole = (typeof WORK_ROLES)[number];
type ProfessionalLevel = (typeof PROFESSIONAL_LEVELS)[number];

interface TellUsAboutYouStepProps {
  onContinue: (workRole: string, professionalLevel?: string, otherText?: string) => void;
  initialWorkRole?: string;
  initialProfessionalLevel?: string;
  initialOtherText?: string;
}

export const TellUsAboutYouStep: React.FC<TellUsAboutYouStepProps> = ({
  onContinue,
  initialWorkRole = "",
  initialProfessionalLevel = "",
  initialOtherText = "",
}) => {
  const { t } = useTranslation();
  const [selectedWorkRole, setSelectedWorkRole] =
    useState<string>(initialWorkRole);
  const [selectedProfessionalLevel, setSelectedProfessionalLevel] =
    useState<string>(initialProfessionalLevel);
  const [otherText, setOtherText] = useState<string>(initialOtherText);

  useEffect(() => {
    setSelectedWorkRole(initialWorkRole);
    setSelectedProfessionalLevel(initialProfessionalLevel);
    setOtherText(initialOtherText);
  }, [initialWorkRole, initialProfessionalLevel, initialOtherText]);

  const selectWorkRole = (role: string) => {
    if (selectedWorkRole === role) {
      setSelectedWorkRole("");
      setSelectedProfessionalLevel("");
      setOtherText("");
    } else {
      setSelectedWorkRole(role);
      setSelectedProfessionalLevel("");
      setOtherText("");
    }
  };

  const handleOtherTextChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value.slice(0, MAX_INPUT_LENGTH);
    setOtherText(value);
  };

  const selectProfessionalLevel = (level: string) => {
    if (selectedProfessionalLevel === level) {
      setSelectedProfessionalLevel("");
    } else {
      setSelectedProfessionalLevel(level);
    }
  };

  const handleContinue = () => {
    onContinue(selectedWorkRole, selectedProfessionalLevel, otherText);
  };

  // Determine if we should show the professional level question
  const showProfessionalLevel =
    selectedWorkRole && !ROLES_WITHOUT_LEVEL.includes(selectedWorkRole);

  // Show other input when "other" role is selected
  const showOtherInput = selectedWorkRole === "other";

  // Button styling - unified border-highlight style
  const getButtonClass = (isSelected: boolean) =>
    `rounded-full border-2 px-4 py-2 text-sm font-medium transition-all cursor-pointer ${
      isSelected
        ? "border-primary text-primary bg-primary/5"
        : "border-border bg-background text-foreground hover:border-muted-foreground/50"
    }`;

  return (
    <OnboardingLayout
      currentStep="tellUsAboutYou"
      leftContent={
        <div className="flex flex-col gap-8">
          {/* Title */}
          <h1 className="text-3xl font-semibold tracking-tight text-foreground lg:text-4xl">
            {t("onboarding.tellUsAboutYou.title")}
          </h1>

          {/* Work Role Section */}
          <div className="flex flex-col gap-3">
            <p className="text-base font-medium text-muted-foreground">
              {t("onboarding.tellUsAboutYou.workQuestion")}
            </p>
            <div className="flex flex-wrap gap-2">
              {WORK_ROLES.map((role) => {
                const isSelected = selectedWorkRole === role;
                return (
                  <button
                    key={role}
                    onClick={() => selectWorkRole(role)}
                    className={getButtonClass(isSelected)}
                  >
                    {t(`onboarding.tellUsAboutYou.workRoles.${role}`)}
                  </button>
                );
              })}
            </div>
          </div>

          {/* Other input field - shown when "other" is selected */}
          {showOtherInput && (
            <div className="flex flex-col gap-2">
              <p className="text-sm font-medium text-muted-foreground">
                {t("onboarding.tellUsAboutYou.pleaseSpecify")}
              </p>
              <div className="relative max-w-sm">
                <Input
                  type="text"
                  value={otherText}
                  onChange={handleOtherTextChange}
                  placeholder={t("onboarding.tellUsAboutYou.otherPlaceholder")}
                  maxLength={MAX_INPUT_LENGTH}
                />
                <span className="absolute right-3 top-1/2 -translate-y-1/2 text-xs text-muted-foreground">
                  {otherText.length}/{MAX_INPUT_LENGTH}
                </span>
              </div>
            </div>
          )}

          {/* Professional Level Section - only shown for certain roles */}
          {showProfessionalLevel && (
            <div className="flex flex-col gap-3">
              <p className="text-base font-medium text-muted-foreground">
                {t("onboarding.tellUsAboutYou.levelQuestion")}
              </p>
              <div className="flex flex-wrap gap-2">
                {PROFESSIONAL_LEVELS.map((level) => {
                  const isSelected = selectedProfessionalLevel === level;
                  return (
                    <button
                      key={level}
                      onClick={() => selectProfessionalLevel(level)}
                      className={getButtonClass(isSelected)}
                    >
                      {t(`onboarding.tellUsAboutYou.professionalLevels.${level}`)}
                    </button>
                  );
                })}
              </div>
            </div>
          )}

          <Button
            onClick={handleContinue}
            size="lg"
            className="mt-2 w-fit"
            disabled={!selectedWorkRole}
          >
            {t("onboarding.tellUsAboutYou.continue")}
          </Button>
        </div>
      }
      rightContent={
        <div className="relative flex h-full w-full items-center justify-center px-4">
          {/* Three polaroid-style images arranged with artistic tilts */}
          <div className="relative h-[420px] w-[520px]">
            {/* Artist at work - left, tilted left */}
            <div
              className="absolute left-0 top-1/2 -translate-y-1/2 rotate-[-8deg] rounded-md bg-white p-3 shadow-lg transition-transform hover:rotate-[-5deg] hover:scale-105"
              style={{
                boxShadow:
                  "0 4px 20px rgba(0,0,0,0.15), 0 2px 8px rgba(0,0,0,0.1)",
              }}
            >
              <img
                src="/src-tauri/resources/svg/undraw_artist-at-work_yos7.svg"
                alt="Artist at work"
                className="h-[160px] w-auto object-contain"
              />
            </div>

            {/* Business call - center front, slight tilt */}
            <div
              className="absolute left-1/2 top-1/2 z-10 -translate-x-1/2 -translate-y-1/2 rotate-[3deg] rounded-md bg-white p-3 shadow-xl transition-transform hover:rotate-[1deg] hover:scale-105"
              style={{
                boxShadow:
                  "0 8px 30px rgba(0,0,0,0.2), 0 4px 12px rgba(0,0,0,0.15)",
              }}
            >
              <img
                src="/src-tauri/resources/svg/undraw_business-call_w1gr.svg"
                alt="Business call"
                className="h-[200px] w-auto object-contain"
              />
            </div>

            {/* Designer - right, tilted right */}
            <div
              className="absolute right-0 top-1/2 -translate-y-1/2 rotate-[10deg] rounded-md bg-white p-3 shadow-lg transition-transform hover:rotate-[7deg] hover:scale-105"
              style={{
                boxShadow:
                  "0 4px 20px rgba(0,0,0,0.15), 0 2px 8px rgba(0,0,0,0.1)",
              }}
            >
              <img
                src="/src-tauri/resources/svg/undraw_designer_efwz.svg"
                alt="Designer"
                className="h-[160px] w-auto object-contain"
              />
            </div>
          </div>
        </div>
      }
    />
  );
};

export default TellUsAboutYouStep;

