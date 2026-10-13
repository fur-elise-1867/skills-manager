import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';
import { CompanyApp } from '../CompanyApp';
import { workspaceApi } from './workspace-api';

test('employee adds a project without creating skills and sees customized copies separately', async () => {
  const project = { id: 'project-a', name: 'Project A', path: 'C:/work/Project A' };
  const api = {
    ...workspaceApi(),
    environment: vi.fn().mockResolvedValue({ mock: true, configured: true, profileRoot: 'C:/profile', dataRoot: 'C:/data', storeUrl: 'http://127.0.0.1' }),
    session: vi.fn().mockResolvedValue({ user: { id: 'usr_001', displayName: 'Employee', username: 'employee01' }, persistent: true, expiresAt: '2099-01-01T00:00:00Z' }),
    login: vi.fn(), logout: vi.fn(),
    storeSearch: vi.fn().mockResolvedValue({ items: [], pagination: { page: 1, pageSize: 20, total: 0, totalPages: 0 } }),
    storeDetail: vi.fn(), prepareInstallation: vi.fn(), confirmInstallation: vi.fn(), operationStatus: vi.fn(), cancelOperation: vi.fn(),
    installationsList: vi.fn().mockResolvedValue([
      { id: 'global', skillId: 'skl_001', name: 'Jira skill', target: 'C:/profile/.aipro/skills/jira', packageHash: 'a', installedAt: '', mcpBypassed: false, contentState: 'original' },
      { id: 'project-copy', skillId: 'skl_001', name: 'Jira skill', target: 'C:/work/Project A/.aipro/skills/jira', projectId: 'project-a', packageHash: 'a', installedAt: '', mcpBypassed: false, contentState: 'customized' },
    ]),
  };
  api.addProject.mockImplementation(async () => { api.projectsList.mockResolvedValue([project]); return project; });
  const user = userEvent.setup();
  render(<CompanyApp api={api} />);
  await user.click(await screen.findByRole('button', { name: 'Dự án' }));
  await user.type(screen.getByLabelText('Thư mục dự án'), 'C:/work/Project A');
  await user.click(screen.getByRole('button', { name: 'Thêm dự án' }));
  expect(await screen.findByText('Project A', { exact: true })).toBeInTheDocument();
  expect(api.addProject).toHaveBeenCalledWith('C:/work/Project A');
  await user.click(screen.getByRole('button', { name: 'Skill đã cài' }));
  expect(await screen.findByText('Đã tùy chỉnh')).toBeInTheDocument();
  expect(screen.getByText('Nguyên bản')).toBeInTheDocument();
  await user.click(screen.getAllByRole('button', { name: 'Mở SKILL.md' })[1]);
  await waitFor(() => expect(api.openInstallation).toHaveBeenCalledWith('project-copy', 'skill_file'));
});
