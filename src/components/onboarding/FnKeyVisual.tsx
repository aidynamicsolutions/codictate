import React from "react";
import { useTranslation } from "react-i18next";

interface FnKeyVisualProps {
  isPressed: boolean;
}

/**
 * CSS-styled keyboard Fn key with globe icon
 * Changes background color when pressed
 */
export const FnKeyVisual: React.FC<FnKeyVisualProps> = ({ isPressed }) => {
  const { t } = useTranslation();

  return (
    <div
      className={`
        relative w-16 h-16 rounded-lg
        transition-all duration-150 ease-out
        shadow-[0_4px_0_0_rgba(0,0,0,0.15),inset_0_-2px_4px_rgba(0,0,0,0.1)]
        border border-border/50
        ${
          isPressed
            ? "bg-red-500 shadow-[0_2px_0_0_rgba(0,0,0,0.15)] translate-y-0.5 text-white"
            : "bg-muted hover:bg-muted/80 text-foreground"
        }
      `}
    >
      {/* fn label - positioned top-right */}
      <span className="absolute top-0 right-2 text-[13px] font-medium opacity-70">
        {t("onboarding.hotkeySetup.subtitleFnKey")}
      </span>

      {/* Globe icon - positioned bottom-left */}
      <svg
        className="absolute bottom-1.5 left-2 w-5 h-5"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <circle cx="12" cy="12" r="10" />
        <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
        <path d="M2 12h20" />
        <path d="M4 7c2.5 1.5 5 2 8 2s5.5-.5 8-2" />
        <path d="M4 17c2.5-1.5 5-2 8-2s5.5.5 8 2" />
      </svg>
    </div>
  );
};

export default FnKeyVisual;
