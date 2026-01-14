import React, { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/shared/ui/button";
import { Input } from "@/components/shared/ui/input";
import OnboardingLayout from "./OnboardingLayout";
import { MAX_INPUT_LENGTH } from "@/constants";

// Typing use case options
const TYPING_USE_CASES = [
  "ai_chat",
  "messaging",
  "coding",
  "emails",
  "documents",
  "notes",
  "social_posts",
  "other",
] as const;

type TypingUseCase = (typeof TYPING_USE_CASES)[number];

interface TypingUseCasesStepProps {
  onContinue: (useCases: string[], otherText?: string) => void;
  initialUseCases?: string[];
  initialOtherText?: string;
}

export const TypingUseCasesStep: React.FC<TypingUseCasesStepProps> = ({
  onContinue,
  initialUseCases = [],
  initialOtherText = "",
}) => {
  const { t } = useTranslation();
  const [selectedUseCases, setSelectedUseCases] =
    useState<string[]>(initialUseCases);
  const [otherText, setOtherText] = useState<string>(initialOtherText);

  useEffect(() => {
    setSelectedUseCases(initialUseCases);
    setOtherText(initialOtherText);
  }, [initialUseCases, initialOtherText]);

  const toggleUseCase = (useCase: string) => {
    setSelectedUseCases((prev) => {
      if (prev.includes(useCase)) {
        // Deselecting - also clear other text if deselecting "other"
        if (useCase === "other") {
          setOtherText("");
        }
        return prev.filter((uc) => uc !== useCase);
      } else {
        return [...prev, useCase];
      }
    });
  };

  const handleOtherTextChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value.slice(0, MAX_INPUT_LENGTH);
    setOtherText(value);
  };

  const handleContinue = () => {
    onContinue(selectedUseCases, otherText || undefined);
  };

  // Show other input when "other" is selected
  const showOtherInput = selectedUseCases.includes("other");

  // Button styling - unified border-highlight style for multi-select
  const getButtonClass = (isSelected: boolean) =>
    `rounded-full border-2 px-4 py-2 text-sm font-medium transition-all cursor-pointer ${
      isSelected
        ? "border-primary text-primary bg-primary/5"
        : "border-border bg-background text-foreground hover:border-muted-foreground/50"
    }`;

  return (
    <OnboardingLayout
      currentStep="typingUseCases"
      leftContent={
        <div className="flex flex-col gap-8">
          {/* Title */}
          <div className="flex flex-col gap-8">
            <h1 className="text-3xl font-semibold tracking-tight text-foreground lg:text-4xl">
              {t("onboarding.typingUseCases.title")}
            </h1>
            <div className="flex flex-col gap-0">
              <p className="text-base text-muted-foreground">
                {t("onboarding.typingUseCases.subtitle")}
              </p>
              <p className="text-base text-muted-foreground">
                {t("onboarding.typingUseCases.subtitleHint")}
              </p>
            </div>
          </div>

          {/* Use Cases Selection */}
          <div className="flex flex-col gap-3">
            <div className="flex flex-wrap gap-2">
              {TYPING_USE_CASES.map((useCase) => {
                const isSelected = selectedUseCases.includes(useCase);
                return (
                  <button
                    key={useCase}
                    onClick={() => toggleUseCase(useCase)}
                    className={getButtonClass(isSelected)}
                  >
                    {t(`onboarding.typingUseCases.options.${useCase}`)}
                  </button>
                );
              })}
            </div>
          </div>

          {/* Other input field - shown when "other" is selected */}
          {showOtherInput && (
            <div className="flex flex-col gap-2">
              <p className="text-sm font-medium text-muted-foreground">
                {t("onboarding.typingUseCases.pleaseSpecify")}
              </p>
              <div className="relative max-w-sm">
                <Input
                  type="text"
                  value={otherText}
                  onChange={handleOtherTextChange}
                  placeholder={t("onboarding.typingUseCases.otherPlaceholder")}
                  maxLength={MAX_INPUT_LENGTH}
                />
                <span className="absolute right-3 top-1/2 -translate-y-1/2 text-xs text-muted-foreground">
                  {otherText.length}/{MAX_INPUT_LENGTH}
                </span>
              </div>
            </div>
          )}

          <Button
            onClick={handleContinue}
            size="lg"
            className="mt-2 w-fit"
            disabled={selectedUseCases.length === 0}
          >
            {t("onboarding.typingUseCases.continue")}
          </Button>
        </div>
      }
      rightContent={
        <div className="relative flex h-full w-full items-center justify-center overflow-hidden px-4">
          {/* Mood board - organic scattered layout with overlapping images */}
          <div className="relative h-[500px] w-[600px]">
            {/* Blogging - top left, larger, slight rotation */}
            <div
              className="absolute left-0 top-4 rotate-[-6deg] transition-transform duration-300 hover:rotate-[-3deg] hover:scale-105"
              style={{ zIndex: 2 }}
            >
              <img
                src="/src-tauri/resources/svg/undraw_blogging_38kl.svg"
                alt="Blogging"
                className="h-[180px] w-auto drop-shadow-lg"
              />
            </div>

            {/* Programming - top right, overlapping slightly with blogging */}
            <div
              className="absolute right-8 top-0 rotate-[8deg] transition-transform duration-300 hover:rotate-[5deg] hover:scale-105"
              style={{ zIndex: 3 }}
            >
              <img
                src="/src-tauri/resources/svg/undraw_programming_j1zw.svg"
                alt="Programming"
                className="h-[200px] w-auto drop-shadow-lg"
              />
            </div>

            {/* Key Points - center bottom left, overlapping with others */}
            <div
              className="absolute bottom-16 left-16 rotate-[4deg] transition-transform duration-300 hover:rotate-[1deg] hover:scale-105"
              style={{ zIndex: 4 }}
            >
              <img
                src="/src-tauri/resources/svg/undraw_key-points_iiic.svg"
                alt="Key Points"
                className="h-[220px] w-auto drop-shadow-xl"
              />
            </div>

            {/* Personal Text - bottom right, overlapping */}
            <div
              className="absolute bottom-8 right-4 rotate-[-5deg] transition-transform duration-300 hover:rotate-[-2deg] hover:scale-105"
              style={{ zIndex: 1 }}
            >
              <img
                src="/src-tauri/resources/svg/undraw_personal-text_090t.svg"
                alt="Personal Text"
                className="h-[170px] w-auto drop-shadow-lg"
              />
            </div>
          </div>
        </div>
      }
    />
  );
};

export default TypingUseCasesStep;
