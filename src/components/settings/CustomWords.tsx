import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../hooks/useSettings";
import { Input } from "@/components/shared/ui/input";
import { Button } from "@/components/shared/ui/button";
import { SettingContainer } from "../ui/SettingContainer";
import { logInfo } from "@/utils/logging";
import { X } from "lucide-react";

interface CustomWordsProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const CustomWords: React.FC<CustomWordsProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();
    const [newWord, setNewWord] = useState("");
    const customWords = getSetting("custom_words") || [];

    const handleAddWord = () => {
      const trimmedWord = newWord.trim();
      const sanitizedWord = trimmedWord.replace(/[<>"'&]/g, "");
      if (
        sanitizedWord &&
        !sanitizedWord.includes(" ") &&
        sanitizedWord.length <= 50 &&
        !customWords.includes(sanitizedWord)
      ) {
        logInfo(`Added custom word: ${sanitizedWord}`, "fe");
        updateSetting("custom_words", [...customWords, sanitizedWord]);
        setNewWord("");
      }
    };

    const handleRemoveWord = (wordToRemove: string) => {
      logInfo(`Removed custom word: ${wordToRemove}`, "fe");
      updateSetting(
        "custom_words",
        customWords.filter((word) => word !== wordToRemove),
      );
    };

    const handleKeyPress = (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        handleAddWord();
      }
    };

    return (
      <div className="space-y-4">
        <SettingContainer
          title={t("settings.advanced.customWords.title")}
          description={t("settings.advanced.customWords.description")}
          descriptionMode={descriptionMode}
          grouped={grouped}
        >
          <div className="flex items-center gap-2">
            <Input
              type="text"
              className="max-w-xs"
              value={newWord}
              onChange={(e) => setNewWord(e.target.value)}
              onKeyDown={handleKeyPress}
              placeholder={t("settings.advanced.customWords.placeholder")}
              disabled={isUpdating("custom_words")}
            />
            <Button
              onClick={handleAddWord}
              disabled={
                !newWord.trim() ||
                newWord.includes(" ") ||
                newWord.trim().length > 50 ||
                isUpdating("custom_words")
              }
              variant="default"
            >
              {t("settings.advanced.customWords.add")}
            </Button>
          </div>
        </SettingContainer>
        
        {customWords.length > 0 && (
          <div className="bg-accent/30 rounded-lg p-6 flex flex-col gap-2">
            <p className="text-xs text-muted-foreground font-medium uppercase tracking-wider mb-2">
              {t("settings.advanced.customWords.addedWords")}
            </p>
            <div className="flex flex-wrap gap-2">
              {customWords.map((word) => (
                <div
                  key={word}
                  className="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm rounded-full border transition-all bg-background border-border hover:bg-muted/50 hover:border-destructive/50 group"
                >
                  <span className="text-foreground font-medium">{word}</span>
                  <button
                    type="button"
                    onClick={() => handleRemoveWord(word)}
                    disabled={isUpdating("custom_words")}
                    className="ml-1 text-muted-foreground hover:text-destructive transition-colors focus:outline-none"
                    aria-label={t("settings.advanced.customWords.remove", { word })}
                  >
                    <X className="w-3.5 h-3.5" />
                  </button>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    );
  },
);
