export type OverlayState =
  | "recording"
  | "transcribing"
  | "processing"
  | "connecting"
  | "cancelling"
  | "correcting";

export type OverlayVisualVariant =
  | "bars"
  | "status_message"
  | "pre_ready_shell"
  | "correction";

export function resolveOverlayVisualVariant(
  state: OverlayState,
  hasUndoCard: boolean,
  hasCorrectionData: boolean,
): OverlayVisualVariant {
  if (hasUndoCard) {
    return "status_message";
  }

  if (state === "recording") {
    return "bars";
  }

  if (state === "connecting") {
    return "pre_ready_shell";
  }

  if (state === "correcting" && hasCorrectionData) {
    return "correction";
  }

  return "status_message";
}
