import { invoke } from '@tauri-apps/api/core';
import type { Backup, Environment, Installation, OperationView, Project, SessionView, Skill, SkillPage } from './types';
import { open } from '@tauri-apps/plugin-dialog';

export interface CompanyApi {
  environment(): Promise<Environment>;
  session(): Promise<SessionView>;
  login(username: string, password: string): Promise<SessionView>;
  logout(): Promise<null>;
  storeSearch(query: string, page: number): Promise<SkillPage>;
  storeDetail(id: string): Promise<Skill>;
  installationsList(): Promise<Installation[]>;
  prepareInstallation(id: string): Promise<OperationView>;
  confirmInstallation(id: string, bypassMcp: boolean): Promise<OperationView>;
  operationStatus(id: string): Promise<OperationView>;
  cancelOperation(id: string): Promise<OperationView>;
  projectsList(): Promise<Project[]>;
  addProject(path: string): Promise<Project>;
  removeProject(id: string): Promise<null>;
  chooseProjectFolder(): Promise<string | null>;
  prepareProjectInstallation(id: string, projectId: string): Promise<OperationView>;
  refreshInstallation(id: string): Promise<Installation>;
  openInstallation(id: string, kind: 'skill_file' | 'folder'): Promise<null>;
  checkUpdates(ids: string[]): Promise<Installation[]>;
  prepareUpdate(id: string): Promise<OperationView>;
  applyUpdate(id: string, replaceCustomized: boolean): Promise<OperationView>;
  backupsList(id: string): Promise<Backup[]>;
  openPreview(id: string, side: 'current' | 'incoming', path: string): Promise<null>;
}

export const nativeApi: CompanyApi = {
  environment: () => invoke('company_environment'),
  session: () => invoke('company_session'),
  login: (username, password) => invoke('company_login', { username, password }),
  logout: () => invoke('company_logout'),
  storeSearch: (query, page) => invoke('company_store_search', { query, page }),
  storeDetail: id => invoke('company_store_detail', { id }),
  installationsList: () => invoke('company_installations_list'),
  prepareInstallation: id => invoke('company_operation_prepare', { id }),
  confirmInstallation: (id, bypassMcp) => invoke('company_operation_confirm', { id, bypassMcp }),
  operationStatus: id => invoke('company_operation_status', { id }),
  cancelOperation: id => invoke('company_operation_cancel', { id }),
  projectsList: () => invoke('company_projects_list'),
  addProject: path => invoke('company_project_add', { path }),
  removeProject: id => invoke('company_project_remove', { id }),
  chooseProjectFolder: async () => { const path = await open({ directory: true, multiple: false }); return typeof path === 'string' ? path : null; },
  prepareProjectInstallation: (id, projectId) => invoke('company_project_install_prepare', { id, projectId }),
  refreshInstallation: id => invoke('company_installation_refresh', { id }),
  openInstallation: (id, kind) => invoke('company_installation_open', { id, kind }),
  checkUpdates: ids => invoke('company_updates_check', { ids }),
  prepareUpdate: id => invoke('company_update_prepare', { id }),
  applyUpdate: (id, replaceCustomized) => invoke('company_update_apply', { id, replaceCustomized }),
  backupsList: id => invoke('company_backups_list', { id }),
  openPreview: (id, side, path) => invoke('company_operation_open_preview', { id, side, path }),
};
