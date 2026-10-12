import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';
import { CompanyApp } from '../CompanyApp';

const skill = { id: 'skl_001', slug: 'jira-project-summary', name: 'Jira Summary', updatedAt: '2026-10-12T00:00:00Z', description: 'MCP bắt buộc: jira-mcp', package: { format: 'zip' as const, sizeBytes: 100, sha256: 'a'.repeat(64), etag: '"r1"' } };
function bridge() {
  const op = { id: 'op1', state: 'awaiting_decision', target: 'C:/demo/.aipro/skills/jira-project-summary', mcp: { state: 'missing', required: ['jira-mcp'], present: [], missing: ['jira-mcp'], checkedAt: '2026-10-12T00:00:00Z', reason: null }, progressBytes: 0, totalBytes: 100, error: null };
  return {
    environment: vi.fn().mockResolvedValue({ mock: true, storeUrl: 'http://127.0.0.1:4318', profileRoot: 'C:/demo', dataRoot: 'C:/data', configured: true }),
    session: vi.fn().mockResolvedValue({ user: { id: 'u1', username: 'employee01', displayName: 'Nhân viên' }, expiresAt: '2099-01-01T00:00:00Z', persistent: true }),
    login: vi.fn(), logout: vi.fn(), storeSearch: vi.fn().mockResolvedValue({ items: [skill], pagination: { page: 1, pageSize: 20, total: 1, totalPages: 1 } }),
    storeDetail: vi.fn().mockResolvedValue(skill), installationsList: vi.fn().mockResolvedValue([]),
    prepareInstallation: vi.fn().mockResolvedValue(op),
    confirmInstallation: vi.fn().mockResolvedValue({ ...op, state: 'downloading' }),
    operationStatus: vi.fn().mockResolvedValue({ ...op, state: 'succeeded' }),
    cancelOperation: vi.fn().mockResolvedValue({ ...op, state: 'cancelled' }),
  };
}
async function openInstall(api: ReturnType<typeof bridge>) {
  render(<CompanyApp api={api} />);
  await userEvent.click(await screen.findByRole('button', { name: 'Xem chi tiết Jira Summary' }));
  await userEvent.click(await screen.findByRole('button', { name: 'Cài vào AIPRO Global' }));
  expect(await screen.findByText('Chưa có MCP trong cấu hình')).toBeInTheDocument();
}

test('employee stops installation to configure missing MCP manually', async () => {
  const api = bridge();
  await openInstall(api);
  await userEvent.click(screen.getByRole('button', { name: 'Dừng cài đặt' }));
  await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Cài skill' })).not.toBeInTheDocument());
  expect(api.confirmInstallation).not.toHaveBeenCalled();
});

test('employee bypasses MCP warning and sees successful installation result', async () => {
  const api = bridge();
  await openInstall(api);
  await userEvent.click(screen.getByRole('button', { name: 'Bỏ qua và cài tiếp' }));
  expect(await screen.findByText('Cài xong; đã bỏ qua cảnh báo MCP của lần này.')).toBeInTheDocument();
  expect(api.confirmInstallation).toHaveBeenCalledWith('op1', true);
});

test('installation shows the fresh Store snapshot returned by prepare', async () => {
  const api = bridge();
  const prepared = await api.prepareInstallation();
  api.prepareInstallation.mockResolvedValue({ ...prepared, skill: { ...skill, name: 'Jira Summary mới' }, mcpPath: 'C:/demo/.aipro/mcp.json' });
  await openInstall(api);
  expect(screen.getByRole('heading', { name: 'Jira Summary mới' })).toBeInTheDocument();
  expect(screen.getByText('C:/demo/.aipro/mcp.json')).toBeInTheDocument();
});
