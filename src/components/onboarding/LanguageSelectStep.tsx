import React, { useState, useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { ArrowLeft, Plus, Search, Globe } from "lucide-react";
import { Button } from "@/components/shared/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from "@/components/shared/ui/dialog";
import { Switch } from "@/components/shared/ui/switch";
import { Input } from "@/components/shared/ui/input";
import { ScrollArea } from "@/components/shared/ui/scroll-area";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/shared/ui/tooltip";
import OnboardingLayout from "./OnboardingLayout";
import { useSettings } from "@/hooks/useSettings";
import {
  WHISPER_LANGUAGES,
  WHISPER_LANGUAGE_COUNT,
  getLanguageByCode,
  getLanguageFlag,
  getLanguageLabel,
} from "@/lib/constants/languageData";

// Maximum number of languages user can select (prevents UI overflow and improves UX)
const MAX_SELECTED_LANGUAGES = 8;

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

/**
 * Language grid item for the modal.
 */
const LanguageGridItem: React.FC<{
  code: string;
  label: string;
  flag: string;
  isSelected: boolean;
  isDisabled: boolean;
  onClick: () => void;
}> = ({ label, flag, isSelected, isDisabled, onClick }) => (
  <button
    type="button"
    onClick={onClick}
    disabled={isDisabled}
    className={`flex items-center gap-2 px-3 py-2 text-sm rounded-lg border transition-all w-full text-left ${
      isSelected
        ? "bg-primary/10 border-primary text-primary"
        : isDisabled
          ? "opacity-50 cursor-not-allowed bg-muted/30 border-transparent"
          : "bg-background border-border hover:bg-muted hover:border-primary/50"
    }`}
  >
    <span className="text-base">{flag}</span>
    <span className="truncate">{label}</span>
  </button>
);

export const LanguageSelectStep: React.FC<LanguageSelectStepProps> = ({
  onContinue,
  onBack,
}) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting } = useSettings();

  // Modal state
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [autoDetect, setAutoDetect] = useState(false);

  // Pending changes in modal (not saved until "Save and close")
  const [pendingLanguages, setPendingLanguages] = useState<string[]>([]);

  // Get current settings
  const selectedLanguage = getSetting("selected_language") || "auto";
  const savedLanguages = (getSetting("saved_languages") as string[]) || ["en"];

  // Initialize auto-detect from current selected_language
  useEffect(() => {
    setAutoDetect(selectedLanguage === "auto");
  }, [selectedLanguage]);

  // Filter and sort languages: selected first, then alphabetical
  const filteredLanguages = useMemo(() => {
    let languages = WHISPER_LANGUAGES;
    
    // Apply search filter
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      languages = languages.filter(
        (lang) =>
          lang.label.toLowerCase().includes(query) ||
          lang.code.toLowerCase().includes(query)
      );
    }
    
    // Sort: selected languages first, then rest alphabetically
    return [...languages].sort((a, b) => {
      const aSelected = pendingLanguages.includes(a.code);
      const bSelected = pendingLanguages.includes(b.code);
      
      if (aSelected && !bSelected) return -1;
      if (!aSelected && bSelected) return 1;
      return a.label.localeCompare(b.label);
    });
  }, [searchQuery, pendingLanguages]);

  // Open modal and initialize pending state
  const handleOpenModal = () => {
    setPendingLanguages([...savedLanguages]);
    setAutoDetect(selectedLanguage === "auto");
    setSearchQuery("");
    setIsModalOpen(true);
  };

  // Save and close modal
  const handleSaveAndClose = async () => {
    // Update saved languages
    await updateSetting("saved_languages", pendingLanguages);

    // Update selected language based on auto-detect or first saved language
    if (autoDetect) {
      await updateSetting("selected_language", "auto");
    } else if (pendingLanguages.length > 0) {
      // Active language is the first one in the list
      await updateSetting("selected_language", pendingLanguages[0]);
    }

    setIsModalOpen(false);
  };

  // Check if selection limit is reached
  const isAtLimit = pendingLanguages.length >= MAX_SELECTED_LANGUAGES;

  // Toggle language selection in modal
  const handleToggleLanguage = (code: string) => {
    setPendingLanguages((prev) => {
      if (prev.includes(code)) {
        // Remove language (but keep at least one if not auto-detect)
        if (prev.length > 1 || autoDetect) {
          return prev.filter((c) => c !== code);
        }
        return prev;
      } else {
        // Add language only if under the limit
        if (prev.length >= MAX_SELECTED_LANGUAGES) {
          return prev;
        }
        return [...prev, code];
      }
    });
  };

  // Make a language active (move to front of list)
  const handleMakeActive = async (code: string) => {
    const newList = [code, ...savedLanguages.filter((c) => c !== code)];
    await updateSetting("saved_languages", newList);
    await updateSetting("selected_language", code);
  };

  // Handle auto-detect toggle
  const handleAutoDetectToggle = (checked: boolean) => {
    setAutoDetect(checked);
    if (checked) {
      // When enabling auto-detect, select all languages
      setPendingLanguages(WHISPER_LANGUAGES.map((l) => l.code));
    } else {
      // When disabling, keep current selection or default to English
      if (pendingLanguages.length === WHISPER_LANGUAGE_COUNT) {
        setPendingLanguages(["en"]);
      }
    }
  };

  const handleContinue = () => {
    onContinue();
  };

  const handleBack = () => {
    onBack();
  };

  // Determine which language to show as active
  const activeLanguage = selectedLanguage === "auto" ? null : selectedLanguage;

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

            {/* Language Selection Modal */}
            <Dialog open={isModalOpen} onOpenChange={setIsModalOpen}>
              <DialogContent className="sm:max-w-[700px] h-[600px] flex flex-col" showCloseButton={false}>
                <DialogHeader>
                  <div className="flex items-center justify-between">
                    <div>
                      <DialogTitle>
                        {t("onboarding.languageSelect.modal.title")}
                      </DialogTitle>
                      <DialogDescription>
                        {t("onboarding.languageSelect.modal.subtitle", {
                          appName: t("appName"),
                        })}
                      </DialogDescription>
                    </div>

                    {/* Auto-detect toggle */}
                    <div className="flex items-center gap-2">
                      <span className="text-sm text-muted-foreground">
                        {t("onboarding.languageSelect.modal.autoDetect")}
                      </span>
                      <Switch
                        checked={autoDetect}
                        onCheckedChange={handleAutoDetectToggle}
                      />
                    </div>
                  </div>
                </DialogHeader>

                <div className="flex gap-6 flex-1 min-h-0 mt-4">
                  {/* Left: Language grid with search */}
                  <div className="flex-1 flex flex-col min-h-0">
                    {/* Auto-detect message or search */}
                    {autoDetect ? (
                      <div className="p-4 bg-muted/50 rounded-lg mb-4 text-sm text-muted-foreground">
                        {t("onboarding.languageSelect.modal.autoDetectMessage", {
                          appName: t("appName"),
                        })}
                      </div>
                    ) : (
                      <div className="relative mb-4">
                        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                        <Input
                          type="text"
                          placeholder={t(
                            "onboarding.languageSelect.modal.searchPlaceholder"
                          )}
                          value={searchQuery}
                          onChange={(e) => setSearchQuery(e.target.value)}
                          className="pl-9"
                        />
                      </div>
                    )}

                    {/* Language grid */}
                    <ScrollArea className="flex-1 h-[350px]">
                      <div
                        className={`grid grid-cols-3 gap-2 pr-4 ${autoDetect ? "opacity-60" : ""}`}
                      >
                        {filteredLanguages.map((lang) => {
                          const isSelected = pendingLanguages.includes(lang.code);
                          // Disable if auto-detect is on OR if limit reached and not already selected
                          const isDisabled = autoDetect || (isAtLimit && !isSelected);
                          return (
                            <LanguageGridItem
                              key={lang.code}
                              code={lang.code}
                              label={lang.label}
                              flag={lang.flag}
                              isSelected={isSelected}
                              isDisabled={isDisabled}
                              onClick={() => handleToggleLanguage(lang.code)}
                            />
                          );
                        })}
                      </div>
                    </ScrollArea>
                  </div>

                  {/* Right: Selected panel */}
                  <div className="w-48 flex flex-col">
                    <div className="flex items-center justify-between mb-3">
                      <h4 className="text-sm font-medium text-foreground">
                        {t("onboarding.languageSelect.modal.selected")}
                      </h4>
                      {!autoDetect && (
                        <span className={`text-xs ${isAtLimit ? 'text-amber-500' : 'text-muted-foreground'}`}>
                          {pendingLanguages.length}/{MAX_SELECTED_LANGUAGES}
                        </span>
                      )}
                    </div>


                    {autoDetect ? (
                      <div className="flex items-center gap-2 text-sm text-muted-foreground">
                        <Globe className="h-4 w-4" />
                        <span>
                          {t("onboarding.languageSelect.modal.languagesCount", {
                            count: WHISPER_LANGUAGE_COUNT,
                          })}
                        </span>
                      </div>
                    ) : (
                      <ScrollArea className="flex-1">
                        <div className="flex flex-col gap-2">
                          {pendingLanguages.map((code) => {
                            const lang = getLanguageByCode(code);
                            if (!lang) return null;
                            return (
                              <div
                                key={code}
                                className="flex items-center justify-between text-sm"
                              >
                                <div className="flex items-center gap-2">
                                  <span>{lang.flag}</span>
                                  <span className="truncate">{lang.label}</span>
                                </div>
                                <button
                                  type="button"
                                  onClick={() => handleToggleLanguage(code)}
                                  className="text-muted-foreground hover:text-destructive transition-colors"
                                  disabled={
                                    pendingLanguages.length === 1 && !autoDetect
                                  }
                                >
                                  —
                                </button>
                              </div>
                            );
                          })}
                        </div>
                      </ScrollArea>
                    )}
                  </div>
                </div>

                {/* Save button */}
                <div className="flex justify-end mt-4 pt-4 border-t">
                  <Button onClick={handleSaveAndClose}>
                    {t("onboarding.languageSelect.modal.saveAndClose")}
                  </Button>
                </div>
              </DialogContent>
            </Dialog>
          </div>
        }
      />
    </TooltipProvider>
  );
};

export default LanguageSelectStep;
