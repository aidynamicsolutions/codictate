import { describe, expect, it, vi } from "vitest";

import { selectBackupArchivePath, selectBackupSavePath } from "./backupDialogGuards";

describe("backupDialogGuards", () => {
  it("returns selected save path when dialog succeeds", async () => {
    const onError = vi.fn();
    const selected = await selectBackupSavePath(
      async () => "/tmp/example.codictatebackup",
      onError,
    );

    expect(selected).toBe("/tmp/example.codictatebackup");
    expect(onError).not.toHaveBeenCalled();
  });

  it("handles save dialog throws and caller can skip command invocation", async () => {
    const onError = vi.fn();
    const createBackup = vi.fn();

    const selected = await selectBackupSavePath(async () => {
      throw new Error("save dialog failed");
    }, onError);

    if (selected) {
      await createBackup(selected);
    }

    expect(selected).toBeNull();
    expect(onError).toHaveBeenCalledTimes(1);
    expect(createBackup).not.toHaveBeenCalled();
  });

  it("returns first path when open dialog returns array", async () => {
    const onError = vi.fn();
    const selected = await selectBackupArchivePath(
      async () => ["/tmp/first.codictatebackup", "/tmp/second.codictatebackup"],
      onError,
    );

    expect(selected).toBe("/tmp/first.codictatebackup");
    expect(onError).not.toHaveBeenCalled();
  });

  it("handles open dialog throws and caller can skip preflight command invocation", async () => {
    const onError = vi.fn();
    const preflightRestore = vi.fn();

    const selected = await selectBackupArchivePath(async () => {
      throw new Error("open dialog failed");
    }, onError);

    if (selected) {
      await preflightRestore(selected);
    }

    expect(selected).toBeNull();
    expect(onError).toHaveBeenCalledTimes(1);
    expect(preflightRestore).not.toHaveBeenCalled();
  });
});
