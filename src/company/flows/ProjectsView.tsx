import { useState } from 'react';
import type { CompanyApi } from '../api';
import type { Installation, Project } from '../types';
import { displayPath, errorMessage } from '../types';

export function ProjectsView({ api, projects, installations, onChanged, onBrowse }: { api: CompanyApi; projects: Project[]; installations: Installation[]; onChanged: () => void; onBrowse: (projectId: string) => void }) {
  const [path, setPath] = useState('');
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  async function add(event: React.FormEvent) {
    event.preventDefault(); setBusy(true); setError('');
    try { await api.addProject(path.trim()); setPath(''); onChanged(); }
    catch (e) { setError(errorMessage(e)); }
    finally { setBusy(false); }
  }
  async function choose() {
    try { const selected = await api.chooseProjectFolder(); if (selected) setPath(selected); }
    catch (e) { setError(errorMessage(e)); }
  }
  async function remove(id: string) {
    setBusy(true); setError('');
    try { await api.removeProject(id); onChanged(); }
    catch (e) { setError(errorMessage(e)); }
    finally { setBusy(false); }
  }
  return <>
    <section className="company-card"><h2>Thêm thư mục công việc</h2><p>Dự án không cần Git. Thư mục AIPRO được tạo khi bạn cài skill.</p>
      <form onSubmit={add} className="project-form"><label>Thư mục dự án<input value={path} onChange={e => setPath(e.target.value)} required placeholder={'D:\\Projects\\My Project'} /></label><button type="button" className="secondary" onClick={choose} disabled={busy}>Chọn thư mục…</button><button className="primary" disabled={busy || !path.trim()}>Thêm dự án</button></form>
    </section>
    {error && <div role="alert" className="error-banner">{error}</div>}
    {projects.length === 0 && <p>Chưa có dự án được đăng ký.</p>}
    {projects.map(project => { const count = installations.filter(i => i.projectId === project.id && i.state !== 'removed').length; return <article className="company-card" key={project.id}><h2>{project.name}</h2><p className="target-path">{displayPath(project.path)}</p><p>{count} bản cài được quản lý</p><div className="row-actions"><button className="primary" onClick={() => onBrowse(project.id)}>Cài skill vào {project.name}</button><button className="secondary" disabled={busy || count > 0} title={count ? 'Dự án còn bản cài được quản lý' : undefined} onClick={() => remove(project.id)}>Bỏ khỏi danh sách</button></div></article>; })}
  </>;
}
