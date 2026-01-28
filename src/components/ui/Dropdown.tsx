import React from "react";
import { useTranslation } from "react-i18next";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/shared/ui/select";

export interface DropdownOption {
  value: string;
  label: string;
  disabled?: boolean;
}

interface DropdownProps {
  options: DropdownOption[];
  className?: string;
  selectedValue: string | null;
  onSelect: (value: string) => void;
  placeholder?: string;
  disabled?: boolean;
  onRefresh?: () => void;
}

export const Dropdown: React.FC<DropdownProps> = ({
  options,
  selectedValue,
  onSelect,
  className = "",
  placeholder,
  disabled = false,
  onRefresh,
}) => {
  const { t } = useTranslation();

  // If onRefresh is provided, we might want to trigger it when the dropdown opens.
  // However, Shadcn Select doesn't have a direct "onOpen" prop exposed easily without controlling open state.
  // For now, we'll assume onRefresh is compatible with just rendering the options, 
  // or we can wrap it if strictly needed. 
  // Given previous usage, onRefresh was likely for dynamic lists. 
  // If strict parity is needed, we can use onOpenChange.

  const handleOpenChange = (open: boolean) => {
    if (open && onRefresh) {
      onRefresh();
    }
  };

  return (
    <div className={className}>
      <Select
        value={selectedValue || undefined}
        onValueChange={onSelect}
        disabled={disabled}
        onOpenChange={handleOpenChange}
      >
        <SelectTrigger className="w-full">
          <SelectValue placeholder={placeholder || t("common.select")} />
        </SelectTrigger>
        <SelectContent position="popper">
          {options.length === 0 ? (
            <div className="px-2 py-2 text-sm text-muted-foreground text-center">
              {t("common.noOptionsFound")}
            </div>
          ) : (
            options.map((option) => (
              <SelectItem
                key={option.value}
                value={option.value}
                disabled={option.disabled}
              >
                {option.label}
              </SelectItem>
            ))
          )}
        </SelectContent>
      </Select>
    </div>
  );
};
