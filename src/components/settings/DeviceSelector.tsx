import React from "react";
import { useTranslation } from "react-i18next";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/shared/ui/select";
import { Label } from "@/components/shared/ui/label";
import { Button } from "@/components/shared/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/shared/ui/tooltip";
import { InfoIcon, RotateCcw, RefreshCw } from "lucide-react";

interface DeviceOption {
  value: string;
  label: string;
}

interface DeviceSelectorProps {
  label: string;
  description: string;
  value: string;
  options: DeviceOption[];
  onSelect: (value: string) => void;
  onRefresh: () => void;
  onReset: () => void;
  isLoading?: boolean;
  isUpdating?: boolean;
  disabled?: boolean;
  placeholder?: string;
  descriptionMode?: "inline" | "tooltip";
  loadingLabel?: string;
  refreshLabel?: string;
  resetLabel?: string;
}

export const DeviceSelector: React.FC<DeviceSelectorProps> = ({
  label,
  description,
  value,
  options,
  onSelect,
  onRefresh,
  onReset,
  isLoading = false,
  isUpdating = false,
  disabled = false,
  placeholder = "Select...",
  descriptionMode = "tooltip",
  loadingLabel = "Loading...",
  refreshLabel = "Refresh",
  resetLabel = "Reset",
}) => {
  return (
    <div className={`flex items-center justify-between py-4 ${disabled ? "opacity-50" : ""}`}>
      <div className="flex items-center gap-2">
        <Label className="text-sm font-medium">
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
      <div className="flex items-center gap-2">
        <Select
          value={value}
          onValueChange={onSelect}
          disabled={disabled || isUpdating || isLoading || options.length === 0}
        >
          <SelectTrigger className="w-[280px]">
            <SelectValue placeholder={isLoading ? loadingLabel : placeholder} />
          </SelectTrigger>
          <SelectContent position="popper">
            {options.map((option) => (
              <SelectItem key={option.value} value={option.value}>
                {option.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Button variant="ghost" size="icon" onClick={onRefresh} disabled={disabled || isLoading} title={refreshLabel}>
          <RefreshCw className={`h-4 w-4 ${isLoading ? "animate-spin" : ""}`} />
          <span className="sr-only">{refreshLabel}</span>
        </Button>
        <Button variant="ghost" size="icon" onClick={onReset} disabled={disabled || isUpdating} title={resetLabel}>
          <RotateCcw className="h-4 w-4" />
          <span className="sr-only">{resetLabel}</span>
        </Button>
      </div>
      {descriptionMode === "inline" && (
        <p className="text-sm text-muted-foreground mt-1 col-span-2">
          {description}
        </p>
      )}
    </div>
  );
};
