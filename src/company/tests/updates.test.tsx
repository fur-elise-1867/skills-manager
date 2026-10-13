import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';
import { CompanyApp } from '../CompanyApp';
import { workspaceApi } from './workspace-api';
import { UpdateDialog } from '../flows/UpdateDialog';

test('employee previews a customized update and explicitly replaces only that copy after backup', async () => {
  const mcp = { state: 'not_required', required: [], present: [], missing: [], reason: null, checkedAt: '' };
  const initial = { id: 'update-op', kind: 'update', state: 'awaiting_decision', target: 'C:/project/.aipro/skills/jira', mcp, progressBytes: 0, totalBytes: 100, error: null, customized: true, previewReady: false, changes: [] };
  const ready = { ...initial, previewReady: true, changes: [{ path: 'SKILL.md', kind: 'modified' }] };
  const api = {
    ...workspaceApi(), environment: vi.fn().mockResolvedValue({ mock: true, configured: true, profileRoot: 'C:/profile', dataRoot: 'C:/data', storeUrl: 'http://localhost' }),
    session: vi.fn().mockResolvedValue({ user: { id: 'usr_001', displayName: 'Employee', username: 'employee01' }, persistent: true, expiresAt: '2099-01-01T00:00:00Z' }),
    login: vi.fn(), logout: vi.fn(), storeSearch: vi.fn().mockResolvedValue({ items: [], pagination: { page: 1, pageSize: 20, total: 0, totalPages: 0 } }),
    storeDetail: vi.fn(), prepareInstallation: vi.fn(), confirmInstallation: vi.fn().mockResolvedValue({ ...initial, state: 'downloading' }),
    operationStatus: vi.fn().mockResolvedValue(ready), cancelOperation: vi.fn(),
    installationsList: vi.fn().mockResolvedValue([{ id: 'project-copy', skillId: 'skl_001', name: 'Jira skill', target: initial.target, packageHash: 'old', installedAt: '', mcpBypassed: false, contentState: 'customized', storeState: 'new_package' }]),
  };
  api.prepareUpdate.mockResolvedValue(initial);
  api.applyUpdate.mockImplementation(async () => { api.operationStatus.mockResolvedValue({ ...ready, state: 'succeeded' }); return { ...ready, state: 'committing' }; });
  const user = userEvent.setup(); render(<CompanyApp api={api} />);
  await user.click(await screen.findByRole('button', { name: 'Skill đã cài' }));
  await user.click(screen.getByRole('button', { name: 'Xem cập nhật' }));
  await user.click(await screen.findByRole('button', { name: 'Tải để xem thay đổi' }));
  expect(await screen.findByText('SKILL.md', { exact: true })).toBeInTheDocument();
  expect(api.applyUpdate).not.toHaveBeenCalled();
  expect(screen.getByRole('button', { name: 'Backup và cập nhật' })).toBeDisabled();
  await user.click(screen.getByLabelText('Tôi đồng ý thay bản đã tùy chỉnh bằng gói Store'));
  await user.click(screen.getByRole('button', { name: 'Backup và cập nhật' }));
  await waitFor(() => expect(api.applyUpdate).toHaveBeenCalledWith('update-op', true));
  expect(await screen.findByText('Đã cập nhật đích này; bản trước đã được lưu.')).toBeInTheDocument();
});

test('cancelling after commit has started keeps reporting the actual result instead of claiming the old copy was kept', async () => {
  const op = { id: 'op', state: 'staging' as const, target: 'C:/project', mcp: { state: 'not_required' as const, required: [], present: [], missing: [], reason: null, checkedAt: '' }, progressBytes: 100, totalBytes: 100, error: null, previewReady: true };
  const api = {
    ...workspaceApi(), environment: vi.fn(), session: vi.fn(), login: vi.fn(), logout: vi.fn(), storeSearch: vi.fn(), storeDetail: vi.fn(), installationsList: vi.fn(), prepareInstallation: vi.fn(), confirmInstallation: vi.fn(),
    operationStatus: vi.fn().mockResolvedValue({ ...op, state: 'committing' }), cancelOperation: vi.fn().mockResolvedValue({ ...op, state: 'committing' }),
  };
  const onClose = vi.fn(); const onApplied = vi.fn();
  render(<UpdateDialog api={api} initial={op} onClose={onClose} onApplied={onApplied} onExpired={vi.fn()} />);
  await userEvent.setup().click(screen.getByRole('button', { name: 'Hủy' }));
  expect(api.cancelOperation).toHaveBeenCalledWith('op');
  expect(onClose).not.toHaveBeenCalled();
  expect(await screen.findByRole('button', { name: 'Hủy' })).toBeDisabled();
});

test('binary preview shows information and folder actions while text can open in an editor', async () => {
  const op = { id: 'preview-op', state: 'awaiting_decision' as const, target: 'C:/project', mcp: { state: 'not_required' as const, required: [], present: [], missing: [], reason: null, checkedAt: '' }, progressBytes: 100, totalBytes: 100, error: null, previewReady: true,
    changes: [
      { path: 'tool.exe', kind: 'added' as const, incoming: { kind: 'binary' as const, sizeBytes: 4096 } },
      { path: 'notes.txt', kind: 'added' as const, incoming: { kind: 'text' as const, sizeBytes: 20 } },
    ] };
  const api = { ...workspaceApi(), environment: vi.fn(), session: vi.fn(), login: vi.fn(), logout: vi.fn(), storeSearch: vi.fn(), storeDetail: vi.fn(), installationsList: vi.fn(), prepareInstallation: vi.fn(), confirmInstallation: vi.fn(), operationStatus: vi.fn(), cancelOperation: vi.fn() };
  render(<UpdateDialog api={api} initial={op} onClose={vi.fn()} onApplied={vi.fn()} onExpired={vi.fn()} />);
  const binaryRow = screen.getByText('tool.exe').closest('li')!;
  expect(within(binaryRow).queryByRole('button')).not.toBeInTheDocument();
  expect(within(binaryRow).getByText(/4096 bytes/)).toBeInTheDocument();
  const user = userEvent.setup();
  await user.click(within(screen.getByText('notes.txt').closest('li')!).getByRole('button', { name: 'Mở bản Store' }));
  expect(api.openPreview).toHaveBeenCalledWith('preview-op', 'incoming', 'notes.txt');
  await user.click(screen.getByRole('button', { name: 'Thư mục bản Store' }));
  expect(api.openPreview).toHaveBeenCalledWith('preview-op', 'incoming', '');
});
