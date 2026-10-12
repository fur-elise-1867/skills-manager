import { invoke } from '@tauri-apps/api/core';
import type { Environment, Installation, OperationView, SessionView, Skill, SkillPage } from './types';

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
};
