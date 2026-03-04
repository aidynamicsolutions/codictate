# Backup/Restore Pre-Merge Manual Test Checklist

## Context
- App: Codictate (`com.pais.codictate`)
- Date: 2026-03-02
- Existing app-data backup already prepared at:
`/Users/tiger/Desktop/codictate-data-backup-20260302-122837`

---

## 1. Setup

- [ ] Launch app from staged changes (`bun run tauri:dev`).
- [ ] Confirm feature is tested on macOS.
- [ ] Open Settings -> General -> Backup/Restore.
- [ ] Create a notes file to record observed counts and any failures.
- [ ] Keep this checklist open during execution.

---

## 2. Pass A: Existing Real Data

### A1. Baseline snapshot

- [ ] Record current history count (and roughly saved/unsaved mix).
- [ ] Record dictionary entry count.
- [ ] Record Home stats baseline values: `Total Words`, `WPM`, `Time Saved`, and `Daily Streak`.
- [ ] Record one visible user-profile/growth indicator in UI (if available).
- [ ] Record one app setting value (example: language or paste method).

### A2. Backup creation

- [ ] Click `Create Backup`, keep `Include recordings` ON, and save as `passA-complete.codictatebackup`.
- [ ] Click `Create Backup`, turn `Include recordings` OFF, and save as `passA-smaller.codictatebackup`.
- [ ] Confirm both operations complete with success feedback.

### A3. Preflight checks

- [ ] Run restore preflight on `passA-complete.codictatebackup`.
- [ ] Confirm summary is shown (created time, app version, platform, counts).
- [ ] Confirm compatibility note is visible in preflight modal (`macOS guaranteed in v1; cross-platform best-effort`).
- [ ] Confirm no blocking errors for this valid archive.
- [ ] Confirm no recoverable warning about `history/user_stats.json` for newly-created backups.

### A4. Mutate current state after backup

- [ ] Add at least 1 new transcription.
- [ ] Add at least 1 dictionary entry.
- [ ] Delete 1 unsaved history entry.
- [ ] Change 1 app setting value.
- [ ] Record these post-backup changes in notes.

### A5. Restore + verify

- [ ] Apply restore from `passA-complete.codictatebackup`.
- [ ] Confirm history/dictionary/user data return to baseline snapshot.
- [ ] Confirm Home stats (`Total Words`, `WPM`, `Time Saved`, `Daily Streak`) match the A1 baseline snapshot.
- [ ] Confirm app settings are NOT replaced by backup (settings should remain current-install values).
- [ ] Confirm no crashes and UI remains responsive.

### A6. Undo restore + verify

- [ ] Click `Undo Last Restore`.
- [ ] Confirm undo immediately shows in-progress UI in the Backup/Restore card (progress row + phase text + percent).
- [ ] Confirm `Undo Last Restore` button shows a loading spinner while undo is running.
- [ ] Confirm state returns to the post-backup mutated state (A4).
- [ ] Confirm undo availability updates afterward.

### A7. Restart persistence

- [ ] Fully quit and relaunch app.
- [ ] Confirm data state after undo remains correct after restart.

### A8. Negative preflight safety

- [ ] Try restore preflight with an invalid/corrupted `.codictatebackup` file.
- [ ] Confirm restore is blocked safely with error feedback.

### A9. Legacy stats fallback warning

- [ ] Create a legacy-like archive variant by removing `history/user_stats.json` and its checksum line.
- [ ] Run preflight and confirm restore remains allowed with a recoverable warning about stats recompute.
- [ ] Apply restore and confirm success warning count includes the stats-recompute warning.
- [ ] Confirm restored stats are plausible (no extreme WPM spike/regression).

---

## 3. Pass B: Clean Slate

### B1. Swap to clean app data (no deletion)

- [ ] Quit app completely.
- [ ] Run:
```bash
APP_DATA="$HOME/Library/Application Support/com.pais.codictate"
ORIG_DATA="$HOME/Library/Application Support/com.pais.codictate.premerge-orig-$(date +%Y%m%d-%H%M%S)"
mv "$APP_DATA" "$ORIG_DATA"
echo "$ORIG_DATA"
```
- [ ] Save printed `ORIG_DATA` path in notes (needed for restoration later).
- [ ] Relaunch app and confirm it starts with empty/fresh data.

### B2. Minimal seed data

- [ ] Create 1 transcription entry.
- [ ] Add 1 dictionary entry.
- [ ] Click `Create Backup`, keep `Include recordings` ON, and save as `passB-complete.codictatebackup`.

### B3. Mutate then restore

- [ ] Add another transcription entry.
- [ ] Modify dictionary entries.
- [ ] Apply restore from `passB-complete.codictatebackup`.
- [ ] Confirm state matches B2 snapshot exactly.

### B4. Undo restore on clean slate

- [ ] Run `Undo Last Restore`.
- [ ] Confirm undo progress row appears immediately and phase label updates until completion.
- [ ] Confirm undo button switches to spinner state during operation and returns to normal after completion.
- [ ] Confirm state returns to the post-backup mutated state from B3.

### B5. Cancellation and write-gate behavior

- [ ] Start a backup/restore large enough to show progress.
- [ ] Click `Cancel` while operation is active.
- [ ] Confirm operation exits safely (no crash, app responsive).
- [ ] During active operation, attempt to start transcription.
- [ ] Confirm transcription is blocked until operation completes.

### B6. Restart persistence

- [ ] Restart app.
- [ ] Confirm state is preserved correctly after restart.

### B7. Restore your original dev data

- [ ] Quit app completely.
- [ ] Run:
```bash
APP_DATA="$HOME/Library/Application Support/com.pais.codictate"
CLEAN_DATA="$HOME/Library/Application Support/com.pais.codictate.clean-pass-$(date +%Y%m%d-%H%M%S)"
mv "$APP_DATA" "$CLEAN_DATA"
mv "<PASTE_ORIG_DATA_PATH_FROM_B1>" "$APP_DATA"
```
- [ ] Relaunch app and confirm original dev data is back.

---

## 4. Merge Gate (Pass/Fail)

- [ ] PASS only if all checklist items succeed in both Pass A and Pass B.
- [ ] PASS only if there are no crashes, no stuck maintenance mode, and no data corruption symptoms.
- [ ] If FAIL, record failing step ID, observed behavior, expected behavior, and timestamp.
