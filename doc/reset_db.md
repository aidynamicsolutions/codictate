# Resetting the Database

For testing purposes (e.g., verifying fresh install behavior or clearing corrupted data), you may need to reset the application database.

**⚠️ WARNING: This will permanently delete all your transcription history and stats.**

## macOS

Run the following command in your terminal:

```bash
rm ~/Library/Application\ Support/com.pais.codictate/history.db
```

## Windows

Navigate to `%APPDATA%\com.pais.codictate\` and delete `history.db`.

## Linux

Navigate to `~/.config/com.pais.codictate/` (or standard XDG data location) and delete `history.db`.

## What happens next?

When you restart Codictate:
1. The app detects the missing database.
2. It creates a fresh `history.db`.
3. It initializes the `user_stats` table with zero values (Total Words: 0, etc.).
