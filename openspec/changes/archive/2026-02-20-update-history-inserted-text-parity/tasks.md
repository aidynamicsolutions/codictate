## 1. Backend Data Model
- [x] 1.1 Add additive migration for `inserted_text` in `transcription_history`
- [x] 1.2 Extend `HistoryEntry` with `inserted_text`, `effective_text`, and `raw_text`
- [x] 1.3 Implement effective fallback order (`inserted -> post_processed -> raw`)
- [x] 1.4 Add update-by-id helpers for inserted text and refine output

## 2. Transcribe / Refine / Paste Flows
- [x] 2.1 Return saved row id from history save contract
- [x] 2.2 Persist transformed inserted text (`PasteResult.pasted_text`) by exact row id on transcribe paste success
- [x] 2.3 Use effective text for `paste_last_transcript`
- [x] 2.4 Keep refine input sourced from raw ASR and update same latest row with refine output
- [x] 2.5 Update inserted text on refine paste success only

## 3. Search / Tray Consistency
- [x] 3.1 Search history by both effective and raw text
- [x] 3.2 Use effective text in tray last-transcript copy helper

## 4. Frontend History UX
- [x] 4.1 Render effective text as primary row text
- [x] 4.2 Add inline `Original transcript` disclosure when raw differs
- [x] 4.3 Keep copy action bound to primary/effective text
- [x] 4.4 Show raw-only match hint for search results

## 5. Localization and Docs
- [x] 5.1 Add history disclosure/search-hint keys across locale files
- [x] 5.2 Update smart insertion notes with history inserted-text parity behavior
- [x] 5.3 Add manual checklist cases for history parity and raw disclosure

## 6. Testing
- [x] 6.1 Add Rust tests for migration compatibility and effective fallback behavior
- [x] 6.2 Add Rust tests for update-by-id and search matching behavior
- [x] 6.3 Add Rust regression tests for effective/raw action selection helpers
- [x] 6.4 Add Vitest coverage for history display/copy/search matching utility behavior
- [x] 6.5 Run `cargo test`
- [x] 6.6 Run `bun run test`

## 7. Validation
- [x] 7.1 Run `openspec validate update-history-inserted-text-parity --strict`
- [x] 7.2 Run `openspec validate --all --strict`
