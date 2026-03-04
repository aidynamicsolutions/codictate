import type {
  PreflightCompatibilityNoteCode,
  PreflightRestoreReport,
} from "@/bindings";

type Translator = (
  key: string,
  options?: Record<string, string | number | boolean | null | undefined>,
) => string;

const COMPATIBILITY_NOTE_LABEL_KEYS: Record<PreflightCompatibilityNoteCode, string> =
  {
    v1_macos_guaranteed_cross_platform_best_effort:
      "settings.backup.restore.compatibilityNotes.v1MacosGuaranteedCrossPlatformBestEffort",
  };

const DEFAULT_COMPATIBILITY_NOTE_LABEL_KEY =
  "settings.backup.restore.compatibilityNote";

export function resolvePreflightCompatibilityNote(
  report: Pick<
    PreflightRestoreReport,
    "compatibility_note_code" | "compatibility_note"
  >,
  t: Translator,
): string {
  const codeLabelKey = COMPATIBILITY_NOTE_LABEL_KEYS[report.compatibility_note_code];
  if (codeLabelKey) {
    return t(codeLabelKey);
  }

  if (report.compatibility_note.trim().length > 0) {
    return report.compatibility_note;
  }

  return t(DEFAULT_COMPATIBILITY_NOTE_LABEL_KEY);
}
