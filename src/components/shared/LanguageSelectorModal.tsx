import React, { useState, useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Search, Globe, AlertCircle } from "lucide-react";
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
import { useSettings } from "@/hooks/useSettings";
import {
  WHISPER_LANGUAGES,
  WHISPER_LANGUAGE_COUNT,
  getLanguageByCode,
} from "@/lib/constants/languageData";

// Maximum number of languages user can select
const MAX_SELECTED_LANGUAGES = 8;

interface LanguageSelectorModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

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

export const LanguageSelectorModal: React.FC<LanguageSelectorModalProps> = ({
  open,
  onOpenChange,
}) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting } = useSettings();

  // Internal state
  const [searchQuery, setSearchQuery] = useState("");
  const [autoDetect, setAutoDetect] = useState(false);
  
  // pendingLanguages = the list of "favorites"
  const [pendingLanguages, setPendingLanguages] = useState<string[]>([]);
  // tempActiveLanguage = the one currently selected as "Primary"
  const [tempActiveLanguage, setTempActiveLanguage] = useState<string>("auto");

  // Get current settings
  const selectedLanguage = getSetting("selected_language") || "auto";
  const savedLanguages = (getSetting("saved_languages") as string[]) || ["en"];

  // Initialize state when modal opens
  useEffect(() => {
    if (open) {
      setPendingLanguages([...savedLanguages]);
      // If selectedLanguage is "auto", enabled autoDetect. 
      // Otherwise, autoDetect is off, and tempActive is the specific language.
      const isAuto = selectedLanguage === "auto";
      setAutoDetect(isAuto);
      setTempActiveLanguage(selectedLanguage);
      setSearchQuery("");
    }
  }, [open, selectedLanguage, savedLanguages]);

  // Filter and sort languages
  // ... (keep usage of useMemo same as before, but sort based on tempActive too if desired, 
  // or just keep pendingLanguages boost)
  const filteredLanguages = useMemo(() => {
    let languages = WHISPER_LANGUAGES;

    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      languages = languages.filter(
        (lang) =>
          lang.label.toLowerCase().includes(query) ||
          lang.code.toLowerCase().includes(query)
      );
    }

    return [...languages].sort((a, b) => {
      const aSelected = pendingLanguages.includes(a.code);
      const bSelected = pendingLanguages.includes(b.code);

      if (aSelected && !bSelected) return -1;
      if (!aSelected && bSelected) return 1;
      return a.label.localeCompare(b.label);
    });
  }, [searchQuery, pendingLanguages]);

  // Save and close
  const handleSaveAndClose = async () => {
    // 1. Save the list
    await updateSetting("saved_languages", pendingLanguages);

    // 2. Save the active language
    if (autoDetect) {
      await updateSetting("selected_language", "auto");
    } else {
      // If for some reason tempActiveLanguage is not in pendingLanguages (edge case),
      // we should probably default to the first one in pending, or add it.
      // But UI should prevent this state.
      // Let's ensure consistency:
      let finalActive = tempActiveLanguage;
      if (!pendingLanguages.includes(finalActive) && pendingLanguages.length > 0) {
        finalActive = pendingLanguages[0];
      }
      await updateSetting("selected_language", finalActive);
    }

    onOpenChange(false);
  };

  const isAtLimit = pendingLanguages.length >= MAX_SELECTED_LANGUAGES;

  const handleToggleLanguage = (code: string) => {
    setPendingLanguages((prev) => {
      // If removing
      if (prev.includes(code)) {
        // Prevent removing if it's the LAST one and not in auto-detect mode
        if (prev.length <= 1 && !autoDetect) {
          return prev; 
        }
        
        const newList = prev.filter((c) => c !== code);
        
        // If we removed the currently active language, switch active to the first available
        if (code === tempActiveLanguage && !autoDetect) {
             setTempActiveLanguage(newList[0] || "auto");
        }
        return newList;
      } 
      // If adding
      else {
        if (prev.length >= MAX_SELECTED_LANGUAGES) {
          return prev;
        }
        const newList = [...prev, code];
        // If this is the only language (was empty), make it active
        if (prev.length === 0 && !autoDetect) {
            setTempActiveLanguage(code);
        }
        return newList;
      }
    });
  };

  // Called when user clicks a language in the sidebar to make it active
  const handleMakeActive = (code: string) => {
    if (!autoDetect && pendingLanguages.includes(code)) {
      setTempActiveLanguage(code);
    }
  };

  const handleAutoDetectToggle = (checked: boolean) => {
    setAutoDetect(checked);
    if (checked) {
      setTempActiveLanguage("auto");
      // Optional: fill pending with all? Or just leave as is?
      // Existing logic was: setPendingLanguages(WHISPER_LANGUAGES.map((l) => l.code));
      // BUT, UX-wise, maybe better to just keep user's favorites list intact?
      // The requirement "show them ... if all 8 are selected" implies we might want to just keeping them.
      // Let's stick to: "If auto, we don't enforce the list limit/selection for the backend, but we keep the list for UI"
      // Wait, original code reset list on auto. Let's preserve the list this time, safer for "toggling back".
      // EXCEPT if the list was empty/invalid? 
      // Let's keep the existing logic of "Select All" visual if that was desired, 
      // but usually "Auto" means "I don't care". 
      // Let's just keep the list as is. If user goes back to manual, they have their list.
    } else {
      // Switching from Auto -> Manual
      // If list is empty, default to English
      if (pendingLanguages.length === 0) {
          setPendingLanguages(["en"]);
          setTempActiveLanguage("en");
      } else {
          // If we have a list, ensure tempActive is valid
          if (tempActiveLanguage === "auto" || !pendingLanguages.includes(tempActiveLanguage)) {
              setTempActiveLanguage(pendingLanguages[0]);
          }
      }
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="sm:max-w-[700px] h-[600px] flex flex-col"
        showCloseButton={false}
      >
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
          <div className="flex-1 flex flex-col min-h-0">
            {autoDetect ? (
              <div className="p-4 bg-muted/50 rounded-lg mb-4 text-sm text-muted-foreground flex items-start gap-2">
                 <AlertCircle className="h-4 w-4 shrink-0 mt-0.5" />
                 <div className="flex flex-col gap-1">
                    <p>
                        {t("onboarding.languageSelect.modal.autoDetectMessage", {
                            appName: t("appName"),
                        })}
                    </p>
                    <p className="text-amber-500 font-medium">
                        {t("onboarding.languageSelect.modal.autoDetectWarning")}
                    </p>
                 </div>
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

            <ScrollArea className="flex-1 h-[350px]">
              <div
                className={`grid grid-cols-3 gap-2 pr-4 ${
                  autoDetect ? "opacity-60 pointer-events-none" : ""
                }`}
              >
                {filteredLanguages.map((lang) => {
                  const isSelected = pendingLanguages.includes(lang.code);
                  const isActive = !autoDetect && tempActiveLanguage === lang.code;
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

          <div className="w-48 flex flex-col">
            <div className="flex items-center justify-between mb-3">
              <h4 className="text-sm font-medium text-foreground">
                {t("onboarding.languageSelect.modal.selected")}
              </h4>
              {!autoDetect && (
                <span
                  className={`text-xs ${
                    isAtLimit ? "text-amber-500" : "text-muted-foreground"
                  }`}
                >
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
                    const isActive = tempActiveLanguage === code;
                    
                    return (
                      <div
                        key={code}
                        className={`flex items-center justify-between text-sm p-2 rounded-md cursor-pointer transition-colors ${
                            isActive ? "bg-primary/10 border border-primary/20" : "hover:bg-muted"
                        }`}
                        onClick={() => handleMakeActive(code)}
                      >
                        <div className="flex items-center gap-2 overflow-hidden">
                          <span>{lang.flag}</span>
                          <span className={`truncate ${isActive ? "font-medium text-primary" : ""}`}>
                              {lang.label}
                          </span>
                        </div>
                        
                        {/* Only show remove button if NOT active (or change logic to allow remove active -> auto switch) */}
                        {/* Actually, let's allow removing from here too. The toggle logic handles re-assigning active. */}
                        <button
                          type="button"
                          onClick={(e) => {
                              e.stopPropagation();
                              handleToggleLanguage(code);
                          }}
                          className="text-muted-foreground hover:text-destructive transition-colors px-1"
                          disabled={
                            pendingLanguages.length === 1
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
            
            {!autoDetect && pendingLanguages.length > 1 && (
                <div className="mt-2 text-xs text-muted-foreground text-center">
                    {t("onboarding.languageSelect.modal.clickToSetActive")}
                </div>
            )}
          </div>
        </div>

        <div className="flex justify-end mt-4 pt-4 border-t">
          <Button onClick={handleSaveAndClose}>
            {t("onboarding.languageSelect.modal.saveAndClose")}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
};
