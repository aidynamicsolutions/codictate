import React from "react";
import { Switch } from "@/components/shared/ui/switch";
import { Label } from "@/components/shared/ui/label";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/shared/ui/tooltip";
import { InfoIcon } from "lucide-react";
import { SettingsRow } from "./SettingsRow";

interface ToggleSwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  isUpdating?: boolean;
  label: string;
  description: string;
  descriptionMode?: "inline" | "tooltip";
  id?: string;
  className?: string;
  grouped?: boolean; // Kept for compatibility with legacy calls
}

export const ToggleSwitch: React.FC<ToggleSwitchProps> = ({
  checked,
  onChange,
  disabled = false,
  isUpdating = false,
  label,
  description,
  descriptionMode = "tooltip",
  id,
  className = "",
}) => {
  const switchId = id || label.toLowerCase().replace(/\s+/g, "-");

  const titleNode = (
    <div className="flex items-center gap-2">
      <span>{label}</span>
      {descriptionMode === "tooltip" && (
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <InfoIcon className="h-3.5 w-3.5 text-muted-foreground/70 cursor-help" />
            </TooltipTrigger>
            <TooltipContent>
              <p className="max-w-xs">{description}</p>
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      )}
    </div>
  );

  return (
    <SettingsRow
      title={titleNode}
      description={descriptionMode === "inline" ? description : undefined}
      className={className}
      disabled={disabled}
    >
      <Switch
        id={switchId}
        checked={checked}
        onCheckedChange={onChange}
        disabled={disabled || isUpdating}
      />
    </SettingsRow>
  );
};
