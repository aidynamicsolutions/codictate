import React from "react";
import { Button } from "@/components/shared/ui/button";
import { cva } from "class-variance-authority";
import { cn } from "@/lib/utils";

const settingsButtonVariants = cva(
  "h-9 px-6 min-w-[12rem] text-sm font-medium shadow-sm rounded-md",
  {
    variants: {
      variant: {
        default: "",
        destructive: "bg-destructive hover:bg-destructive/90 text-destructive-foreground",
        outline: "bg-secondary hover:bg-secondary/80 text-secondary-foreground",
        secondary: "bg-secondary hover:bg-secondary/80 text-secondary-foreground",
        ghost: "",
        link: "",
      },
    },
    defaultVariants: {
      variant: "outline",
    },
  }
);

interface SettingsRowProps {
  /** Title of the setting row */
  title: React.ReactNode;
  /** Description text (can include formatted content) */
  description?: React.ReactNode;
  /** Label for the action button */
  buttonLabel?: string;
  /** Handler for the action button click */
  onButtonClick?: () => void;
  buttonVariant?: "default" | "destructive" | "outline" | "secondary" | "ghost" | "link";
  /** Whether the button is disabled */
  disabled?: boolean;
  children?: React.ReactNode;
  className?: string;
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
  children,
  className,
}: SettingsRowProps) => {
  return (
    <div className={cn("flex items-center justify-between py-4", className)}>
      <div className="flex flex-col gap-1.5 flex-1 mr-6">
        <div className="text-sm font-medium text-foreground/90 flex items-center gap-2">
          {title}
        </div>
        {description && (
          <div className="text-[13px] text-muted-foreground/80 leading-relaxed font-normal">
            {description}
          </div>
        )}
      </div>
      <div className="flex items-center shrink-0">
        {children}
        {buttonLabel && (
          <Button
            variant={buttonVariant === "outline" ? "secondary" : buttonVariant}
            size="sm"
            onClick={onButtonClick}
            disabled={disabled}
            className={settingsButtonVariants({ variant: buttonVariant })}
          >
            {buttonLabel}
          </Button>
        )}
      </div>
    </div>
  );
};

