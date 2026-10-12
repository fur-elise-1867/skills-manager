import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';
import { CompanyApp } from '../CompanyApp';

const skill = { id: 'skl_001', slug: 'jira-project-summary', name: 'Tổng hợp tiến độ Jira', updatedAt: '2026-10-12T00:00:00Z', descriptionExcerpt: 'Tổng hợp công việc dự án.', package: { format: 'zip' as const, sizeBytes: 2048, sha256: 'a'.repeat(64), etag: '"r1"' } };
const page = { items: [skill], pagination: { page: 1, pageSize: 20, total: 1, totalPages: 1 } };
function api() {
  return {
    environment: vi.fn().mockResolvedValue({ mock: true, storeUrl: 'http://127.0.0.1:4318', profileRoot: 'C:/demo', dataRoot: 'C:/data', configured: true }),
    session: vi.fn().mockResolvedValue({ user: { id: 'usr_001', username: 'employee01', displayName: 'Nhân viên 01' }, expiresAt: '2099-10-12T12:00:00Z', persistent: true }),
    login: vi.fn(), logout: vi.fn(), installationsList: vi.fn().mockResolvedValue([]),
    storeSearch: vi.fn().mockResolvedValue(page),
    storeDetail: vi.fn().mockResolvedValue({ ...skill, description: 'Mô tả đầy đủ.\n\nMCP bắt buộc: jira-mcp, confluence-mcp\n\nHướng dẫn nội bộ.' }),
    prepareInstallation: vi.fn(), confirmInstallation: vi.fn(), operationStatus: vi.fn(), cancelOperation: vi.fn(),
  };
}

test('employee searches the Store and opens full description with MCP declaration', async () => {
  const bridge = api();
  render(<CompanyApp api={bridge} />);
  const user = userEvent.setup();
  await user.type(await screen.findByPlaceholderText('Tìm theo tên hoặc công dụng skill…'), 'jira');
  await waitFor(() => expect(bridge.storeSearch).toHaveBeenLastCalledWith('jira', 1));
  await user.click(await screen.findByRole('button', { name: 'Xem chi tiết Tổng hợp tiến độ Jira' }));
  expect(await screen.findByText(/MCP bắt buộc: jira-mcp, confluence-mcp/)).toBeInTheDocument();
  expect(screen.getByText('Hướng dẫn nội bộ.')).toBeInTheDocument();
});

test('expired session asks for login while local installations stay available', async () => {
  const bridge = api();
  bridge.storeSearch.mockRejectedValue({ code: 'SESSION_EXPIRED', message: 'Phiên đăng nhập đã hết hạn.' });
  render(<CompanyApp api={bridge} />);
  expect(await screen.findByLabelText('Tài khoản')).toBeInTheDocument();
  await userEvent.click(screen.getByRole('button', { name: 'Skill đã cài' }));
  expect(await screen.findByText('Chưa có skill được cài bởi ứng dụng.')).toBeInTheDocument();
});

test('offline cached catalog shows its timestamp and disables Store installation', async () => {
  const bridge = api();
  bridge.storeSearch.mockResolvedValue({ ...page, cache: { fetchedAt: '2026-10-12T00:00:00Z', stale: true } });
  render(<CompanyApp api={bridge} />);
  expect(await screen.findByText(/Dữ liệu đã lưu lúc/)).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Xem chi tiết Tổng hợp tiến độ Jira' })).toBeDisabled();
  expect(bridge.prepareInstallation).not.toHaveBeenCalled();
});

test('a late response from the old query cannot replace the new results', async () => {
  const bridge = api();
  let finishOld: (value: typeof page) => void = () => {};
  bridge.storeSearch.mockImplementation(query => query === 'old' ? new Promise(resolve => { finishOld = resolve; }) : Promise.resolve({ ...page, items: [{ ...skill, name: query === 'new' ? 'Kết quả mới' : skill.name }] }));
  render(<CompanyApp api={bridge} />);
  const user = userEvent.setup();
  const input = await screen.findByLabelText('Tìm kiếm skill');
  await user.type(input, 'old');
  await waitFor(() => expect(bridge.storeSearch).toHaveBeenCalledWith('old', 1));
  await user.clear(input); await user.type(input, 'new');
  expect(await screen.findByRole('heading', { name: 'Kết quả mới' })).toBeInTheDocument();
  finishOld(page);
  await waitFor(() => expect(screen.queryByRole('heading', { name: skill.name })).not.toBeInTheDocument());
});
