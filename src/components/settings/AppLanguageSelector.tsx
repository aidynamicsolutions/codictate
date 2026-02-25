import React from "react";
import { Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Dropdown } from "../ui/Dropdown";
import { SettingContainer } from "../ui/SettingContainer";
import {
  SUPPORTED_LANGUAGES,
  changeLanguageSafely,
  type SupportedLanguageCode,
} from "../../i18n";
import { useSettings } from "@/hooks/useSettings";

interface AppLanguageSelectorProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const AppLanguageSelector: React.FC<AppLanguageSelectorProps> =
  React.memo(({ descriptionMode = "tooltip", grouped = false }) => {
    const { t, i18n } = useTranslation();
    const { settings, updateSetting } = useSettings();
    const [isLanguageChanging, setIsLanguageChanging] = React.useState(false);

    const currentLanguage = (settings?.app_language ||
      i18n.language) as SupportedLanguageCode;

    const languageOptions = SUPPORTED_LANGUAGES.map((lang) => ({
      value: lang.code,
      label: `${lang.nativeName} (${lang.name})`,
    }));

    const handleLanguageChange = async (langCode: string) => {
      if (isLanguageChanging || langCode === currentLanguage) {
        return;
      }

      setIsLanguageChanging(true);
      try {
        const languageApplied = await changeLanguageSafely(langCode);
        if (languageApplied) {
          await updateSetting("app_language", langCode);
        }
      } finally {
        setIsLanguageChanging(false);
      }
    };

    return (
      <SettingContainer
        title={t("appLanguage.title")}
        description={t("appLanguage.description", { appName: t("appName") })}
        descriptionMode={descriptionMode}
        grouped={grouped}
        className={grouped ? "px-0" : undefined}
      >
        <div className="min-w-[200px]">
          <Dropdown
            options={languageOptions}
            selectedValue={currentLanguage}
            onSelect={(value) => {
              void handleLanguageChange(value);
            }}
            disabled={isLanguageChanging}
          />
          {isLanguageChanging && (
            <div
              className="mt-2 flex items-center gap-1.5 text-xs text-muted-foreground"
              aria-live="polite"
            >
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
              <span>{t("common.loading")}</span>
            </div>
          )}
        </div>
      </SettingContainer>
    );
  });

AppLanguageSelector.displayName = "AppLanguageSelector";
