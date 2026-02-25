import React from "react";
import { Globe, Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/shared/ui/dropdown-menu";
import { SUPPORTED_LANGUAGES, changeLanguageSafely } from "@/i18n";

/**
 * Subtle language switcher for the onboarding flow.
 * Displays a globe icon + current language code, opens a dropdown with all supported languages.
 */
export const OnboardingLanguageSwitcher: React.FC = () => {
  const { t, i18n } = useTranslation();
  const [isLanguageChanging, setIsLanguageChanging] = React.useState(false);
  const currentLang = SUPPORTED_LANGUAGES.find(
    (lang) => lang.code === i18n.language
  );

  const handleLanguageChange = async (code: string) => {
    if (isLanguageChanging || code === i18n.language) {
      return;
    }

    setIsLanguageChanging(true);
    try {
      await changeLanguageSafely(code);
    } finally {
      setIsLanguageChanging(false);
    }
  };

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          className={`flex items-center gap-1.5 rounded-md px-2 py-1 text-xs font-medium text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus:outline-none focus-visible:ring-2 focus-visible:ring-ring ${isLanguageChanging ? "cursor-wait opacity-70" : ""}`}
          aria-label={t("appLanguage.title")}
          aria-busy={isLanguageChanging}
          disabled={isLanguageChanging}
        >
          {isLanguageChanging ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
          ) : (
            <Globe className="h-3.5 w-3.5" />
          )}
          <span className="uppercase">{currentLang?.code || "EN"}</span>
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="min-w-[160px]">
        {isLanguageChanging && (
          <DropdownMenuItem disabled>
            <Loader2 className="mr-2 h-3.5 w-3.5 animate-spin" />
            <span>{t("common.loading")}</span>
          </DropdownMenuItem>
        )}
        {SUPPORTED_LANGUAGES.map((lang) => (
          <DropdownMenuItem
            key={lang.code}
            onClick={() => {
              void handleLanguageChange(lang.code);
            }}
            disabled={isLanguageChanging}
            className={
              lang.code === i18n.language
                ? "bg-accent font-medium"
                : undefined
            }
          >
            <span className="mr-2">{lang.flag}</span>
            <span>{lang.nativeName}</span>
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
};

export default OnboardingLanguageSwitcher;
