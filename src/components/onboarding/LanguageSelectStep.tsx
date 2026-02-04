import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { ArrowLeft, Plus, Globe } from "lucide-react";
import { Button } from "@/components/shared/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/shared/ui/tooltip";
import OnboardingLayout from "./OnboardingLayout";
import { useSettings } from "@/hooks/useSettings";
import {
  getLanguageLabel,
  getLanguageFlag,
} from "@/lib/constants/languageData";
import { LanguageSelectorModal } from "@/components/shared/LanguageSelectorModal";

interface LanguageSelectStepProps {
  onContinue: () => void;
  onBack: () => void;
}

/**
 * Language chip component for the main UI.
 * Active chip is highlighted, inactive chips show tooltip on hover.
 */
const LanguageChip: React.FC<{
  code: string;
  isActive: boolean;
  onClick: () => void;
  showTooltip?: boolean;
  tooltipText?: string;
}> = ({ code, isActive, onClick, showTooltip = false, tooltipText }) => {
  const { t } = useTranslation();
  const flag = getLanguageFlag(code);
  const label = getLanguageLabel(code) ?? t("settings.general.language.auto");

  const chipContent = (
    <button
      type="button"
      onClick={onClick}
      className={`inline-flex items-center gap-1.5 px-3 py-1.5 text-sm rounded-full border transition-all ${
        isActive
          ? "bg-primary text-primary-foreground border-primary font-medium"
          : "bg-muted/50 text-foreground border-border hover:bg-muted hover:border-primary/50"
      }`}
    >
      <span>{flag}</span>
      <span>{label}</span>
    </button>
  );

  if (showTooltip && tooltipText) {
    return (
      <Tooltip>
        <TooltipTrigger asChild>{chipContent}</TooltipTrigger>
        <TooltipContent>
          <p>{tooltipText}</p>
        </TooltipContent>
      </Tooltip>
    );
  }

  return chipContent;
};

export const LanguageSelectStep: React.FC<LanguageSelectStepProps> = ({
  onContinue,
  onBack,
}) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting } = useSettings();

  // Modal state
  const [isModalOpen, setIsModalOpen] = useState(false);

  // Get current settings
  const selectedLanguage = getSetting("selected_language") || "auto";
  const savedLanguages = (getSetting("saved_languages") as string[]) || ["en"];

  // Make a language active (move to front of list)
  const handleMakeActive = async (code: string) => {
    const newList = [code, ...savedLanguages.filter((c) => c !== code)];
    await updateSetting("saved_languages", newList);
    await updateSetting("selected_language", code);
  };

  const handleContinue = () => {
    onContinue();
  };

  const handleBack = () => {
    onBack();
  };

  const handleOpenModal = () => setIsModalOpen(true);

  return (
    <TooltipProvider>
      <OnboardingLayout
        currentStep="languageSelect"
        leftContent={
          <div className="flex flex-col h-full">
            {/* Back button */}
            <button
              type="button"
              onClick={handleBack}
              className="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground transition-colors w-fit mb-auto"
            >
              <ArrowLeft className="h-4 w-4" />
              {t("onboarding.languageSelect.back")}
            </button>

            {/* Content centered */}
            <div className="flex flex-col gap-4 my-auto">
              <h1 className="mb-4 text-3xl font-semibold tracking-tight text-foreground lg:text-4xl max-w-[380px]">
                {t("onboarding.languageSelect.title")}
              </h1>
              <p className="text-muted-foreground">
                {t("onboarding.languageSelect.subtitle", {
                  appName: t("appName"),
                })}
              </p>
              <p className="text-muted-foreground text-sm">
                {t("onboarding.languageSelect.description", {
                  appName: t("appName"),
                })}
              </p>
            </div>

            <div className="mb-auto" />
          </div>
        }
        rightContent={
          <div className="flex items-center justify-center h-full w-full">
            {/* Main card */}
            <div className="bg-background rounded-xl shadow-lg p-8 max-w-md w-full">
              <div className="flex flex-col gap-6">
                {/* Section title */}
                <p className="text-center text-foreground font-medium">
                  {selectedLanguage === "auto" || savedLanguages.length <= 1
                    ? t("onboarding.languageSelect.yourSelectedLanguage")
                    : t("onboarding.languageSelect.yourSelectedLanguages")}
                </p>

                {/* Language chips area */}
                <div className="bg-accent/30 rounded-lg p-6 flex items-center justify-center min-h-[80px]">
                  <div className="flex flex-wrap items-center gap-2 justify-center">
                    {selectedLanguage === "auto" ? (
                      // Auto-detect mode: show globe icon
                      <div className="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm rounded-full border bg-primary text-primary-foreground border-primary font-medium">
                        <Globe className="h-4 w-4" />
                        <span>{t("settings.general.language.auto")}</span>
                      </div>
                    ) : (
                      // Show saved languages as chips
                      savedLanguages.map((code, index) => (
                        <LanguageChip
                          key={code}
                          code={code}
                          isActive={index === 0}
                          onClick={() => {
                            if (index !== 0) handleMakeActive(code);
                          }}
                          showTooltip={index !== 0}
                          tooltipText={t(
                            "onboarding.languageSelect.makeActiveHint"
                          )}
                        />
                      ))
                    )}

                    {/* Add button */}
                    {selectedLanguage !== "auto" && (
                      <button
                        type="button"
                        onClick={handleOpenModal}
                        className="inline-flex items-center justify-center w-8 h-8 rounded-full border border-dashed border-muted-foreground/50 text-muted-foreground hover:border-primary hover:text-primary transition-colors"
                      >
                        <Plus className="h-4 w-4" />
                      </button>
                    )}
                  </div>
                </div>

                {/* Action buttons */}
                <div className="flex items-center justify-end gap-3">
                  <Button variant="outline" onClick={handleOpenModal}>
                    {t("onboarding.languageSelect.changeLanguages")}
                  </Button>
                  <Button onClick={handleContinue} className="min-w-[80px]">
                    {t("onboarding.languageSelect.continue")}
                  </Button>
                </div>
              </div>
            </div>

            {/* Reusable Language Selection Modal */}
            <LanguageSelectorModal 
              open={isModalOpen} 
              onOpenChange={setIsModalOpen} 
            />
          </div>
        }
      />
    </TooltipProvider>
  );
};

export default LanguageSelectStep;
