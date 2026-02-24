import { commands } from "@/bindings";
import { logError } from "@/utils/logging";

export type UiAnalyticsEvent =
  | "settings_opened"
  | "onboarding_completed"
  | "analytics_toggle_changed"
  | "upgrade_prompt_shown"
  | "upgrade_prompt_action"
  | "upgrade_checkout_result";

export async function trackUiAnalyticsEvent(
  event: UiAnalyticsEvent,
  props?: Record<string, string>,
): Promise<void> {
  try {
    const result = await commands.trackUiAnalyticsEvent(event, props ?? null);
    if (result.status === "error") {
      logError(
        `event=analytics_track_failed scope=ui event_name=${event} error=${result.error}`,
        "fe-analytics",
      );
    }
  } catch (error) {
    logError(
      `event=analytics_track_failed scope=ui event_name=${event} error=${error}`,
      "fe-analytics",
    );
  }
}
