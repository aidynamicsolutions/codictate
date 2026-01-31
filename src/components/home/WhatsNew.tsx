import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { X } from "lucide-react";
import { Button } from "@/components/shared/ui/button";

export function WhatsNew() {
  const { t } = useTranslation();
  // What's New visibility: Tied to specific versions (e.g. v1, v2)
  // Currently v0 (hidden placeholders), will be enabled in future updates
  const WHATS_NEW_VERSION_KEY = "home.showWhatsNew_v0";
  const [isVisible, setIsVisible] = useState(() => {
    // Default to false for now, as user requested it to be blank/hidden initially
    return localStorage.getItem(WHATS_NEW_VERSION_KEY) === "true";
  });

  const handleDismiss = () => {
    setIsVisible(false);
    localStorage.setItem(WHATS_NEW_VERSION_KEY, "false");
  };

  if (!isVisible) return null;

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold text-muted-foreground">
          {t("home.whatsNew.title") || "What's New"}
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
      {/* Content for What's New will go here in future updates */}
      <div className="flex flex-col gap-2">
        <div className="p-4 bg-muted/30 rounded-lg text-sm text-muted-foreground">
          {t("home.whatsNew.placeholder") || "No new updates at the moment."}
        </div>
      </div>
    </div>
  );
}
