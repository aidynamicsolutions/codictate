import React from "react";
import { Globe } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/shared/ui/dropdown-menu";
import { SUPPORTED_LANGUAGES } from "@/i18n";

/**
 * Subtle language switcher for the onboarding flow.
 * Displays a globe icon + current language code, opens a dropdown with all supported languages.
 */
export const OnboardingLanguageSwitcher: React.FC = () => {
  const { i18n } = useTranslation();
  const currentLang = SUPPORTED_LANGUAGES.find(
    (lang) => lang.code === i18n.language
  );

  const handleLanguageChange = (code: string) => {
    i18n.changeLanguage(code);
  };

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          className="flex items-center gap-1.5 rounded-md px-2 py-1 text-xs font-medium text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          aria-label="Change language"
        >
          <Globe className="h-3.5 w-3.5" />
          <span className="uppercase">{currentLang?.code || "EN"}</span>
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="min-w-[160px]">
        {SUPPORTED_LANGUAGES.map((lang) => (
          <DropdownMenuItem
            key={lang.code}
            onClick={() => handleLanguageChange(lang.code)}
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
