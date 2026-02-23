# Merge History Archive

This directory records each upstream sync merge into our custom branch.

## Why This Exists

- Gives future developers a clear audit trail of what was taken from upstream.
- Documents conflict decisions and why they were made.
- Makes future syncs faster by reusing previous conflict patterns.

## Historical Note

Existing files such as `*-merge-origin-main.md` were created during the earlier workflow when upstream changes were merged into `llm`/`main`.

Current workflow uses:

- Source branch: `upstream-main` (tracking `upstream/main`)
- Target branch: `main`

## File Naming Convention

Use:

- `YYYY-MM-DD-merge-upstream-main.md`

Example:

- `2026-03-01-merge-upstream-main.md`

## Required Sections

Each merge record should include:

1. Source and target branches plus before/after SHAs.
2. Commit review table from upstream with take/skip/partial decisions.
3. Conflict resolution notes per file.
4. Verification commands and results.
5. Final status and follow-up tasks (if any).

## Template

Copy:

- `doc/mergehistory/TEMPLATE.md`

and fill it out for each upstream sync merge.
