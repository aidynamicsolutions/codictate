import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { Mic, Keyboard, Book, X } from "lucide-react";
import { Button } from "@/components/shared/ui/button";


interface GettingStartedProps {
  onNavigate?: (section: string) => void;
}

const GetStartedItem = ({
  icon: Icon,
  title,
  subtitle,
  onClick,
  shortcut,
}: {
  icon: any;
  title: string;
  subtitle: string;
  onClick?: () => void;
  shortcut?: React.ReactNode;
}) => (
  <div
    className={`flex items-start gap-4 p-2 rounded-lg transition-colors group ${
      onClick ? "hover:bg-muted/50 cursor-pointer" : ""
    }`}
    onClick={onClick}
  >
    <div className="mt-1 text-muted-foreground group-hover:text-foreground transition-colors">
      <Icon size={20} />
    </div>
    <div className="flex-1">
      <div className="flex items-center justify-between">
        <h3 className="font-medium text-foreground">{title}</h3>
        {shortcut && <div className="text-muted-foreground">{shortcut}</div>}
      </div>
      <p className="text-sm text-muted-foreground">{subtitle}</p>
    </div>
  </div>
);

export function GettingStarted({ onNavigate }: GettingStartedProps) {
  const { t } = useTranslation();
  // Getting Started visibility: Once hidden, it stays hidden forever
  const [isVisible, setIsVisible] = useState(() => {
    return localStorage.getItem("home.showGetStarted") !== "false";
  });

  const handleDismiss = () => {
    setIsVisible(false);
    localStorage.setItem("home.showGetStarted", "false");
  };

  if (!isVisible) return null;

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold text-muted-foreground">
          {t("home.getStarted.title")}
        </h2>
        <Button
          variant="ghost"
          size="sm"
          className="h-8 px-2 text-muted-foreground hover:text-foreground"
          onClick={handleDismiss}
          title="Hide section"
        >
          <X size={16} />
        </Button>
      </div>
      <div className="flex flex-col gap-2">
        <GetStartedItem
          icon={Mic}
          title={t("home.getStarted.startRecording.title")}
          subtitle={t("home.getStarted.startRecording.subtitle")}
          shortcut={
            <div className="flex gap-1">
              <kbd className="px-2 py-0.5 bg-muted rounded text-xs font-mono">
                fn
              </kbd>
            </div>
          }
          // Not clickable as requested
        />
        <GetStartedItem
          icon={Keyboard}
          title={t("home.getStarted.customizeShortcuts.title")}
          subtitle={t("home.getStarted.customizeShortcuts.subtitle")}
          onClick={() => onNavigate?.("settings")}
        />
        <GetStartedItem
          icon={Book}
          title={t("home.getStarted.addVocabulary.title")}
          subtitle={t("home.getStarted.addVocabulary.subtitle")}
          onClick={() => onNavigate?.("advanced")} // Custom words are in Advanced settings
        />
      </div>
    </div>
  );
}
