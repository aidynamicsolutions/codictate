import React from "react";
import { useTranslation } from "react-i18next";
import { X } from "lucide-react";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/shared/ui/select";
import type { HistoryFilter } from "@/hooks/useHistory";

interface HistoryFilterDropdownProps {
  filter: HistoryFilter;
  onFilterChange: (value: HistoryFilter) => void;
  hasActiveFilters: boolean;
  onClearFilters: () => void;
}

export const HistoryFilterDropdown: React.FC<HistoryFilterDropdownProps> = ({
  filter,
  onFilterChange,
  hasActiveFilters,
  onClearFilters,
}) => {
  const { t } = useTranslation();

  return (
    <div className="flex items-center gap-1.5">
      <Select
        value={filter}
        onValueChange={(value) => onFilterChange(value as HistoryFilter)}
      >
        <SelectTrigger
          id="history-filter"
          size="sm"
          className={`h-8 text-xs rounded-full shrink-0 ${
            hasActiveFilters
              ? "bg-primary/10 border-primary/40 text-primary"
              : ""
          }`}
        >
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="all">{t("settings.history.filter.allTime")}</SelectItem>
          <SelectItem value="starred">⭐ {t("settings.history.filter.starred")}</SelectItem>
          <SelectItem value="today">{t("settings.history.filter.today")}</SelectItem>
          <SelectItem value="this_week">{t("settings.history.filter.thisWeek")}</SelectItem>
          <SelectItem value="this_month">{t("settings.history.filter.thisMonth")}</SelectItem>
          <SelectItem value="this_year">{t("settings.history.filter.thisYear")}</SelectItem>
        </SelectContent>
      </Select>
      {hasActiveFilters && (
        <button
          onClick={onClearFilters}
          className="shrink-0 p-1 rounded-full text-muted-foreground hover:text-foreground hover:bg-muted/50 transition-colors"
          title={t("settings.history.filter.clearFilters")}
        >
          <X className="w-3.5 h-3.5" />
        </button>
      )}
    </div>
  );
};
