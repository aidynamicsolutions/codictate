## 1. OpenSpec
- [x] 1.1 Add proposal, design, tasks, and `custom-word-correction` spec delta
- [x] 1.2 Validate with `openspec validate update-dictionary-intent-first-entry-flow --strict`

## 2. Dictionary Modal Refactor
- [x] 2.1 Replace chip controls with intent selector (`Recognize this term` / `Replace spoken phrase`)
- [x] 2.2 Keep aliases visible by default in both intents
- [x] 2.3 Show fuzzy as a conditional single-row control only for eligible recognize input
- [x] 2.4 Enforce replace-intent output validation (non-empty and different from input)
- [x] 2.5 Preserve payload mapping contract for `CustomWordEntry`

## 3. Frontend Logic and Copy
- [x] 3.1 Add/update utility helpers for intent derivation and replacement validation
- [x] 3.2 Update Dictionary page quick tips and help modal copy
- [x] 3.3 Update i18n keys and remove obsolete wording (`Output different text`)

## 4. Documentation
- [x] 4.1 Update dictionary user guide to reflect intent-first flow and Dictionary vs Snippets boundary
- [x] 4.2 Update custom-word-correction doc terminology and guidance

## 5. Verification
- [x] 5.1 Add/update Vitest coverage for utility helpers
- [x] 5.2 Run targeted checks (`bunx eslint ...`, `bunx tsc --noEmit`, `bun run test src/utils/dictionaryUtils.test.ts`)
- [x] 5.3 Perform manual QA against S1-S5 state model and save rules
