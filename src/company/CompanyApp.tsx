import { useCallback, useEffect, useState } from 'react';
import { nativeApi } from './api';
import type { CompanyApi } from './api';
import type { Environment, Installation, Project, SessionView } from './types';
import { errorMessage } from './types';
import './company.css';
import { StoreView } from './flows/StoreView';
import { ProjectsView } from './flows/ProjectsView';
import { InstalledView } from './flows/InstalledView';
import { StorageView } from './flows/StorageView';

const emptySession: SessionView = { user: null, expiresAt: null, persistent: false };
const tabs = ['Store', 'Skill đã cài', 'Dự án', 'Cài đặt'] as const;
type Tab = typeof tabs[number];

export function CompanyApp({ api = nativeApi }: { api?: CompanyApi }) {
  const [environment, setEnvironment] = useState<Environment>();
  const [session, setSession] = useState(emptySession);
  const [tab, setTab] = useState<Tab>('Store');
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [installations, setInstallations] = useState<Installation[]>([]);
  const [projects, setProjects] = useState<Project[]>([]);
  const [projectId, setProjectId] = useState('');
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const refreshProjects = useCallback(() => { api.projectsList().then(setProjects).catch(e => setError(errorMessage(e))); }, [api]);
  const [booting, setBooting] = useState(true);
  const [resumeSkillId, setResumeSkillId] = useState<string>();
  const expired = useCallback((message: string, skillId?: string) => { setResumeSkillId(skillId); setSession(emptySession); setError(message); }, []);
  const resumed = useCallback(() => setResumeSkillId(undefined), []);
  const refreshInstallations = useCallback(() => { Promise.all([api.installationsList(), api.projectsList()]).then(([items, knownProjects]) => { setInstallations(items); setProjects(knownProjects); }).catch(e => setError(errorMessage(e))); }, [api]);

  useEffect(() => {
    let active = true;
    Promise.all([api.environment(), api.session(), api.installationsList(), api.projectsList()]).then(([env, current, local, knownProjects]) => {
      if (active) { setEnvironment(env); setSession(current); setInstallations(local); setProjects(knownProjects); }
    }).catch(e => { if (active) setError(errorMessage(e)); }).finally(() => { if (active) setBooting(false); });
    return () => { active = false; };
  }, [api]);

  async function login(event: React.FormEvent) {
    event.preventDefault(); setBusy(true); setError('');
    try { const current = await api.login(username.trim(), password); setSession(current); if (!current.persistent) setError('Phiên chỉ được giữ trong lần mở app này; bạn sẽ cần đăng nhập lại khi mở app.'); }
    catch (e) { setError(errorMessage(e)); }
    finally { setPassword(''); setBusy(false); }
  }

  async function logout() {
    setBusy(true); setError('');
    try { await api.logout(); }
    catch (e) { setError(errorMessage(e)); }
    finally { setSession(emptySession); setResumeSkillId(undefined); setPassword(''); setBusy(false); }
  }

  const signIn = <section className="company-card sign-in">
    <div className="eyebrow">TÀI KHOẢN CÔNG TY</div><h2>Đăng nhập Skill Store</h2>
    <p>Dùng tài khoản được công ty cấp để tìm và tải skill.</p>
    <form onSubmit={login}>
      <label>Tài khoản<input autoComplete="username" value={username} onChange={e => setUsername(e.target.value)} required /></label>
      <label>Mật khẩu<input type="password" autoComplete="current-password" value={password} onChange={e => setPassword(e.target.value)} required /></label>
      <button className="primary" disabled={busy || !environment?.configured}>{busy ? 'Đang đăng nhập…' : 'Đăng nhập'}</button>
    </form>
  </section>;

  return <div className="company-app">
    <aside className="company-sidebar"><div className="company-brand"><span className="brand-icon">S</span><div>Company<span>Skills Manager</span></div></div>
      <nav aria-label="Điều hướng">{tabs.map(item => <button key={item} className={tab === item ? 'selected' : ''} onClick={() => { setTab(item); setError(''); if (item === 'Skill đã cài') refreshInstallations(); }}>{item}</button>)}</nav>
      <div className="sidebar-footer"><span className="aipro-dot" /> AIPRO<span>Quản lý skill trên máy của bạn</span></div>
    </aside>
    <main className="company-main"><header className="company-header"><div className="eyebrow">WORKSPACE / AIPRO</div><div className="account">{session.user ? <><span>{session.user.displayName}</span><button onClick={logout} disabled={busy}>Đăng xuất</button></> : <span>Chưa đăng nhập</span>}</div></header>
      {environment?.mock && <div className="mock-banner">Dữ liệu giả lập</div>}
      {error && <div role="alert" className="error-banner">{error}</div>}
      {environment?.recoveryWarnings?.map(message => <div className="error-banner" role="alert" key={message}>{message}</div>)}
      {environment && !environment.configured && <div role="status" className="error-banner">Chưa cấu hình địa chỉ Skill Store. Liên hệ quản trị ứng dụng.</div>}
      <div className="page-heading"><h1>{tab === 'Store' ? 'Skill Store' : tab}</h1><p>{tab === 'Store' ? 'Tìm skill phù hợp và đưa vào AIPRO của bạn.' : 'Các bản cài và dữ liệu trên máy Windows hiện tại.'}</p></div>
      {booting && <p role="status">Đang mở môi trường AIPRO…</p>}
      {tab === 'Store' && !booting && (!session.user ? signIn : <StoreView key={session.user.id} api={api} projects={projects} projectId={projectId} onProjectChange={setProjectId} onExpired={expired} onInstalled={refreshInstallations} resumeSkillId={resumeSkillId} onResumed={resumed} />)}
      {tab === 'Skill đã cài' && <InstalledView api={api} installations={installations} projects={projects} onChanged={refreshInstallations} signedIn={!!session.user} onExpired={expired} selectedIds={selectedIds} onSelectionChange={setSelectedIds} onReinstall={item => { if (!item.skillId || item.origin === 'local') return; setProjectId(item.projectId ?? ''); setResumeSkillId(item.skillId); setTab('Store'); }} />}
      {tab === 'Dự án' && <ProjectsView api={api} projects={projects} installations={installations} onChanged={refreshProjects} onBrowse={id => { setProjectId(id); setTab('Store'); }} />}
      {tab === 'Cài đặt' && <><section className="company-card"><h2>Môi trường AIPRO</h2><dl><dt>Thư mục Global</dt><dd>{environment?.profileRoot}\.aipro\skills</dd><dt>Cấu hình MCP</dt><dd>{environment?.profileRoot}\.aipro\mcp.json</dd><dt>Dữ liệu ứng dụng</dt><dd>{environment?.dataRoot}</dd><dt>Skill Store</dt><dd>{environment?.storeUrl || 'Chưa cấu hình'}</dd></dl></section>{!session.user ? signIn : <section className="company-card"><h2>Phiên đăng nhập</h2><p>{session.persistent ? 'Phiên được giữ trong Windows Credential Manager.' : 'Phiên chỉ được giữ trong lần mở app này.'}</p><p>Hết hạn: {session.expiresAt ? new Date(session.expiresAt).toLocaleString('vi-VN') : '—'}</p></section>}</>}
      {tab === 'Cài đặt' && <StorageView api={api} />}
    </main>
  </div>;
}
