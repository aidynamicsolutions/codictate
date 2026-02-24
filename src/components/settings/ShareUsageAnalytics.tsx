import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";
import { trackUiAnalyticsEvent } from "@/utils/analytics";

interface ShareUsageAnalyticsProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const ShareUsageAnalytics: React.FC<ShareUsageAnalyticsProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const shareUsageAnalytics = getSetting("share_usage_analytics") ?? true;

    const handleToggle = async (enabled: boolean) => {
      // When disabling analytics, emit the transition while analytics is still enabled.
      // Backend policy intentionally blocks all events once share_usage_analytics=false.
      // Keep this call ordered before updateSetting so disable transitions remain observable.
      if (!enabled) {
        await trackUiAnalyticsEvent("analytics_toggle_changed", {
          enabled: "disabled",
          source: "settings",
        });
      }

      await updateSetting("share_usage_analytics", enabled);

      if (enabled) {
        await trackUiAnalyticsEvent("analytics_toggle_changed", {
          enabled: "enabled",
          source: "settings",
        });
      }
    };

    return (
      <ToggleSwitch
        checked={shareUsageAnalytics}
        onChange={handleToggle}
        isUpdating={isUpdating("share_usage_analytics")}
        label={t("settings.application.analytics.label")}
        description={t("settings.application.analytics.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
    );
  },
);

ShareUsageAnalytics.displayName = "ShareUsageAnalytics";
