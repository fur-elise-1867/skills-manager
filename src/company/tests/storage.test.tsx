import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';
import { StorageView } from '../flows/StorageView';
import { workspaceApi } from './workspace-api';

test('storage cleanup is manual, cancellation preserves data and only selected unused packages are requested', async () => {
  const api = { ...workspaceApi(), environment: vi.fn(), session: vi.fn(), login: vi.fn(), logout: vi.fn(), storeSearch: vi.fn(), storeDetail: vi.fn(), installationsList: vi.fn(), prepareInstallation: vi.fn(), confirmInstallation: vi.fn(), operationStatus: vi.fn(), cancelOperation: vi.fn(),
    storageStatus: vi.fn().mockResolvedValue({ packageBytes: 100, backupBytes: 200, unreferencedPackageBytes: 40, packages: [{ hash: 'active', bytes: 60, referenced: true }, { hash: 'unused', bytes: 40, referenced: false }], backups: [] }),
    deleteBackup: vi.fn(), cleanupPackages: vi.fn().mockResolvedValue(['unused']) };
  const user = userEvent.setup(); render(<StorageView api={api} />);
  const active = await screen.findByLabelText('Chọn gói active'); expect(active).toBeDisabled();
  await user.click(screen.getByLabelText('Chọn gói unused'));
  await user.click(screen.getByRole('button', { name: 'Dọn các gói đã chọn (1)' }));
  expect(api.cleanupPackages).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button', { name: 'Hủy' })); expect(api.cleanupPackages).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button', { name: 'Dọn các gói đã chọn (1)' }));
  await user.click(screen.getByRole('button', { name: 'Xác nhận xóa' }));
  await waitFor(() => expect(api.cleanupPackages).toHaveBeenCalledWith(['unused']));
  expect(api.deleteBackup).not.toHaveBeenCalled();
});
