import { vi } from 'vitest';

export function workspaceApi() {
  return {
    projectsList: vi.fn().mockResolvedValue([]),
    addProject: vi.fn(), removeProject: vi.fn(),
    chooseProjectFolder: vi.fn().mockResolvedValue(null),
    prepareProjectInstallation: vi.fn(),
    refreshInstallation: vi.fn(), openInstallation: vi.fn(),
    checkUpdates: vi.fn(), prepareUpdate: vi.fn(), applyUpdate: vi.fn(),
    backupsList: vi.fn().mockResolvedValue([]), openPreview: vi.fn(),
    prepareRestore: vi.fn(), prepareRemove: vi.fn(), applyLifecycle: vi.fn(),
    chooseArchive: vi.fn(), prepareLocalImport: vi.fn(),
    storageStatus: vi.fn().mockResolvedValue({ packageBytes: 0, backupBytes: 0, unreferencedPackageBytes: 0, packages: [], backups: [] }), deleteBackup: vi.fn(), cleanupPackages: vi.fn(),
  };
}
