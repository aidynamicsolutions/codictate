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

  return (
    <div className={`flex items-center justify-between py-4 ${className}`}>
      <div className="flex items-center gap-2">
        <Label htmlFor={switchId} className="text-sm font-medium">
          {label}
        </Label>
        {descriptionMode === "tooltip" && (
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <InfoIcon className="h-4 w-4 text-muted-foreground cursor-help" />
              </TooltipTrigger>
              <TooltipContent>
                <p className="max-w-xs">{description}</p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        )}
      </div>
      <Switch
        id={switchId}
        checked={checked}
        onCheckedChange={onChange}
        disabled={disabled || isUpdating}
      />
      {descriptionMode === "inline" && (
        <p className="text-sm text-muted-foreground mt-1 col-span-2">
          {description}
        </p>
      )}
    </div>
  );
};
