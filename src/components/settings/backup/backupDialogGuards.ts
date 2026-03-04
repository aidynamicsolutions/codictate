type DialogErrorHandler = (error: unknown) => void;

export async function selectBackupSavePath(
  selectPath: () => Promise<string | null>,
  onError: DialogErrorHandler,
): Promise<string | null> {
  try {
    return await selectPath();
  } catch (error) {
    onError(error);
    return null;
  }
}

export async function selectBackupArchivePath(
  selectPath: () => Promise<string | string[] | null>,
  onError: DialogErrorHandler,
): Promise<string | null> {
  try {
    const selected = await selectPath();
    return Array.isArray(selected) ? selected[0] ?? null : selected;
  } catch (error) {
    onError(error);
    return null;
  }
}
