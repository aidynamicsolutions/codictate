import type { AudioDevice } from "@/bindings";

/**
 * Check if a microphone setting means "use system default".
 * The frontend may store "default", "Default", or undefined/null.
 */
export function isDefaultMicSetting(value: string | null | undefined): boolean {
  return !value || value.toLowerCase() === "default";
}

/**
 * Resolve the display name of the microphone that will actually be used
 * when "Default" is selected. Mirrors the backend's BT-avoidance priority:
 *   1. Built-in (non-BT system default)
 *   2. Any non-BT device
 *   3. null (couldn't determine)
 *
 * This is for DISPLAY ONLY — the backend makes the authoritative device choice.
 */
export function resolveDefaultMicName(
  audioDevices: AudioDevice[],
): string | null {
  const nonBtDevices = audioDevices.filter(
    (d) => d.name !== "Default" && !d.is_bluetooth,
  );
  const systemDefault = nonBtDevices.find((d) => d.is_default);
  return systemDefault?.name || nonBtDevices[0]?.name || null;
}
