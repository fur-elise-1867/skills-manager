import { workspaceApi } from './workspace-api';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';
import { CompanyApp } from '../CompanyApp';

test('employee can sign in and sign out without losing access to local installations', async () => {
  const api = { ...workspaceApi(),
    environment: vi.fn().mockResolvedValue({ mock: true, storeUrl: 'http://127.0.0.1:4318', profileRoot: 'C:/demo', dataRoot: 'C:/data', configured: true }),
    session: vi.fn().mockResolvedValue({ user: null, expiresAt: null, persistent: true }),
    login: vi.fn().mockResolvedValue({ user: { id: 'usr_001', username: 'employee01', displayName: 'Nhân viên 01' }, expiresAt: '2099-10-12T12:00:00Z', persistent: true }),
    logout: vi.fn().mockResolvedValue(null),
    storeSearch: vi.fn().mockResolvedValue({ items: [], pagination: { page: 1, pageSize: 20, total: 0, totalPages: 0 } }),
    installationsList: vi.fn().mockResolvedValue([]),
    storeDetail: vi.fn(),
    prepareInstallation: vi.fn(), confirmInstallation: vi.fn(), operationStatus: vi.fn(), cancelOperation: vi.fn(),
  };
  render(<CompanyApp api={api} />);
  const user = userEvent.setup();
  await user.type(await screen.findByLabelText('Tài khoản'), 'employee01');
  await user.type(screen.getByLabelText('Mật khẩu'), 'demo-password');
  await user.click(screen.getByRole('button', { name: 'Đăng nhập' }));
  expect(await screen.findByText('Nhân viên 01')).toBeInTheDocument();
  expect(screen.queryByLabelText('Mật khẩu')).not.toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: 'Đăng xuất' }));
  await waitFor(() => expect(screen.queryByText('Nhân viên 01')).not.toBeInTheDocument());
  await user.click(screen.getByRole('button', { name: 'Skill đã cài' }));
  expect(await screen.findByText('Chưa có skill được cài bởi ứng dụng.')).toBeInTheDocument();
  expect(screen.getByText('Dữ liệu giả lập')).toBeInTheDocument();
});

test('invalid credentials are shown and password is cleared', async () => {
  const api = { ...workspaceApi(),
    environment: vi.fn().mockResolvedValue({ mock: false, storeUrl: 'https://store.internal', profileRoot: 'C:/demo', dataRoot: 'C:/data', configured: true }),
    session: vi.fn().mockResolvedValue({ user: null, expiresAt: null, persistent: true }),
    login: vi.fn().mockRejectedValue({ code: 'INVALID_CREDENTIALS', message: 'Tài khoản hoặc mật khẩu không đúng.' }),
    logout: vi.fn(), storeSearch: vi.fn(), storeDetail: vi.fn(), installationsList: vi.fn().mockResolvedValue([]),
    prepareInstallation: vi.fn(), confirmInstallation: vi.fn(), operationStatus: vi.fn(), cancelOperation: vi.fn(),
  };
  render(<CompanyApp api={api} />);
  const user = userEvent.setup();
  await user.type(await screen.findByLabelText('Tài khoản'), 'bad');
  await user.type(screen.getByLabelText('Mật khẩu'), 'secret');
  await user.click(screen.getByRole('button', { name: 'Đăng nhập' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Tài khoản hoặc mật khẩu không đúng.');
  expect(screen.getByLabelText('Mật khẩu')).toHaveValue('');
});
