import React from "react";
import { Button } from "@/components/shared/ui/button";

interface SettingsRowProps {
  /** Title of the setting row */
  title: string;
  /** Description text (can include formatted content) */
  description?: React.ReactNode;
  /** Label for the action button */
  buttonLabel: string;
  /** Handler for the action button click */
  onButtonClick: () => void;
  buttonVariant?: "default" | "destructive" | "outline" | "secondary" | "ghost" | "link";
  /** Whether the button is disabled */
  disabled?: boolean;
}

/**
 * A reusable settings row component with title, description and action button.
 * Used in GeneralSettings for consistent row-based layout.
 */
export const SettingsRow: React.FC<SettingsRowProps> = ({
  title,
  description,
  buttonLabel,
  onButtonClick,
  buttonVariant = "outline",
  disabled = false,
}: SettingsRowProps) => {
  return (
    <div className="flex items-center justify-between py-5">
      <div className="flex items-center gap-4">
        <div className="flex flex-col gap-1">
          <span className="text-base font-medium text-foreground">{title}</span>
          {description && (
            <span className="text-sm text-muted-foreground">{description}</span>
          )}
        </div>
      </div>
      <Button
        variant={buttonVariant}
        size="lg"
        onClick={onButtonClick}
        disabled={disabled}
        className="min-w-[200px] px-8 h-11 text-base font-medium rounded-lg"
      >
        {buttonLabel}
      </Button>
    </div>
  );
};

