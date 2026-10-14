import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';
import { InstalledView } from '../flows/InstalledView';
import { workspaceApi } from './workspace-api';

test('offline employee reviews target and explicitly restores a backup instead of calling Store', async () => {
  const item = { id: 'installation', skillId: 'skill', name: 'Jira skill', target: 'C:/work/.aipro/skills/jira', packageHash: 'old-hash', installedAt: '', mcpBypassed: true, contentState: 'customized' as const };
  const op = { id: 'restore', kind: 'restore' as const, state: 'awaiting_decision' as const, target: item.target, mcp: { state: 'undeclared' as const, required: [], present: [], missing: [], reason: null, checkedAt: '' }, progressBytes: 0, totalBytes: 0, error: null };
  const api = { ...workspaceApi(), environment: vi.fn(), session: vi.fn(), login: vi.fn(), logout: vi.fn(), storeSearch: vi.fn(), storeDetail: vi.fn(), prepareInstallation: vi.fn(), confirmInstallation: vi.fn(), installationsList: vi.fn(), operationStatus: vi.fn().mockResolvedValue({ ...op, state: 'succeeded' }), cancelOperation: vi.fn(), prepareRestore: vi.fn().mockResolvedValue(op), prepareRemove: vi.fn(), applyLifecycle: vi.fn().mockResolvedValue({ ...op, state: 'staging' }) };
  api.backupsList.mockResolvedValue([{ id: 'backup', installationId: item.id, createdAt: '2026-10-14T00:00:00Z', reason: 'update', path: 'C:/data/backups/backup/content', fullHash: 'hash', installation: item }]);
  const user = userEvent.setup();
  render(<InstalledView api={api} installations={[item]} projects={[]} onChanged={vi.fn()} signedIn={false} />);
  await user.click(screen.getByRole('button', { name: 'Bản lưu' }));
  await user.click(await screen.findByRole('button', { name: 'Khôi phục bản lưu' }));
  expect(api.applyLifecycle).not.toHaveBeenCalled();
  expect(screen.getByRole('dialog')).toHaveTextContent(item.target);
  await user.click(screen.getByRole('button', { name: 'Backup và khôi phục' }));
  await waitFor(() => expect(api.applyLifecycle).toHaveBeenCalledWith('restore'));
  expect(await screen.findByText('Đã khôi phục nội dung và baseline của bản lưu.')).toBeInTheDocument();
  expect(api.storeDetail).not.toHaveBeenCalled();
});

test('offline remove is explicitly confirmed and a removed row keeps backup access without reopening foreign content', async () => {
  const item = { id: 'installation', skillId: 'skill', name: 'Jira skill', target: 'C:/profile/.aipro/skills/jira', packageHash: 'hash', installedAt: '', mcpBypassed: false, state: 'active' as const };
  const op = { id: 'remove', kind: 'remove' as const, state: 'awaiting_decision' as const, target: item.target, mcp: { state: 'undeclared' as const, required: [], present: [], missing: [], reason: null, checkedAt: '' }, progressBytes: 0, totalBytes: 0, error: null };
  const api = { ...workspaceApi(), environment: vi.fn(), session: vi.fn(), login: vi.fn(), logout: vi.fn(), storeSearch: vi.fn(), storeDetail: vi.fn(), prepareInstallation: vi.fn(), confirmInstallation: vi.fn(), installationsList: vi.fn(), operationStatus: vi.fn().mockResolvedValue({ ...op, state: 'succeeded' }), cancelOperation: vi.fn().mockResolvedValue({ ...op, state: 'cancelled' }) };
  api.prepareRemove.mockResolvedValue(op); api.applyLifecycle.mockResolvedValue({ ...op, state: 'staging' });
  const user = userEvent.setup();
  const props = { api, installations: [item], projects: [], onChanged: vi.fn(), signedIn: false };
  const { rerender } = render(<InstalledView {...props} />);
  await user.click(screen.getByRole('button', { name: 'Gỡ bản cài' }));
  await screen.findByRole('button', { name: 'Backup và gỡ' });
  expect(api.applyLifecycle).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button', { name: 'Hủy' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  expect(api.applyLifecycle).not.toHaveBeenCalled();
  rerender(<InstalledView {...props} installations={[{ ...item, state: 'removed' }]} />);
  expect(screen.getByText('Đã gỡ')).toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'Mở SKILL.md' })).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Bản lưu' })).toBeEnabled();
});
