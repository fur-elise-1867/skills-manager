import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';
import { ImportDialog } from '../flows/ImportDialog';
import { workspaceApi } from './workspace-api';

test('offline local import shows its origin and requires an explicit MCP bypass before installing', async () => {
  const op = { id: 'local', kind: 'local_import' as const, state: 'awaiting_decision' as const, target: 'C:/profile/.aipro/skills/jira', mcp: { state: 'unknown' as const, required: [], present: [], missing: [], reason: 'Gói local chưa có dữ liệu yêu cầu MCP từ Skill Store.', checkedAt: '' }, progressBytes: 0, totalBytes: 42, error: null,
    skill: { id: '', slug: 'jira', name: 'jira', updatedAt: '', package: { format: 'zip' as const, sizeBytes: 42, sha256: 'hash', etag: '' } } };
  const api = { ...workspaceApi(), environment: vi.fn(), session: vi.fn(), login: vi.fn(), logout: vi.fn(), storeSearch: vi.fn(), storeDetail: vi.fn(), installationsList: vi.fn(), prepareInstallation: vi.fn(), confirmInstallation: vi.fn().mockResolvedValue({ ...op, state: 'staging' }), operationStatus: vi.fn().mockResolvedValue({ ...op, state: 'succeeded' }), cancelOperation: vi.fn(), chooseArchive: vi.fn(), prepareLocalImport: vi.fn().mockResolvedValue(op) };
  const user = userEvent.setup();
  render(<ImportDialog api={api} projects={[]} onClose={vi.fn()} onInstalled={vi.fn()} />);
  await user.type(screen.getByLabelText('File ZIP/7z'), 'C:/input/jira.zip');
  await user.type(screen.getByLabelText('Tên thư mục skill'), 'jira');
  await user.click(screen.getByRole('button', { name: 'Kiểm tra gói local' }));
  expect(await screen.findByText('Gói local chưa có dữ liệu yêu cầu MCP từ Skill Store.')).toBeInTheDocument();
  expect(api.confirmInstallation).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button', { name: 'Bỏ qua và cài tiếp' }));
  await waitFor(() => expect(api.confirmInstallation).toHaveBeenCalledWith('local', true));
  expect(api.prepareLocalImport).toHaveBeenCalledWith('C:/input/jira.zip', 'jira', null);
  expect(api.storeDetail).not.toHaveBeenCalled();
});
