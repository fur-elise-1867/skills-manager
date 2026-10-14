import { useState } from 'react';
import type { CompanyApi } from '../api';
import type { OperationView, Project } from '../types';
import { errorMessage } from '../types';
import { InstallDialog } from './InstallDialog';

export function ImportDialog({ api, projects, onClose, onInstalled }: { api: CompanyApi; projects: Project[]; onClose: () => void; onInstalled: () => void }) {
  const [path, setPath] = useState(''); const [slug, setSlug] = useState(''); const [projectId, setProjectId] = useState('');
  const [op, setOp] = useState<OperationView>(); const [busy, setBusy] = useState(false); const [error, setError] = useState('');
  async function choose() { try { const file = await api.chooseArchive(); if (file) setPath(file); } catch (e) { setError(errorMessage(e)); } }
  async function prepare(event: React.FormEvent) {
    event.preventDefault(); setBusy(true); setError('');
    try { setOp(await api.prepareLocalImport(path.trim(), slug.trim(), projectId || null)); }
    catch (e) { setError(errorMessage(e)); } finally { setBusy(false); }
  }
  if (op?.skill) return <InstallDialog api={api} skill={op.skill} initial={op} onClose={onClose} onInstalled={onInstalled} onExpired={setError} onRefresh={() => setOp(undefined)} scopeLabel={projectId ? 'Dự án' : 'Global'} />;
  return <div className="modal-overlay"><section className="company-modal" role="dialog" aria-modal="true" aria-label="Nhập gói local"><h2>Nhập ZIP/7z từ máy</h2>
    <p>Gói local do bạn cung cấp. Chưa xác nhận phê duyệt hoặc nguồn cập nhật từ Skill Store.</p>
    {error && <p role="alert" className="error-banner">{error}</p>}
    <form onSubmit={prepare}><label>File ZIP/7z<input value={path} onChange={e => setPath(e.target.value)} required /></label><button type="button" className="secondary" onClick={choose} disabled={busy}>Chọn file…</button>
      <label>Tên thư mục skill<input value={slug} onChange={e => setSlug(e.target.value)} required placeholder="jira-project-summary" /></label><p>Dùng chữ thường, số, dấu - hoặc _. Tên phải khớp thư mục gốc nếu gói có wrapper.</p>
      <label>Phạm vi nhập<select value={projectId} onChange={e => setProjectId(e.target.value)}><option value="">AIPRO Global</option>{projects.map(p => <option key={p.id} value={p.id}>Dự án: {p.name}</option>)}</select></label>
      <div className="dialog-actions"><button type="button" className="secondary" disabled={busy} onClick={onClose}>Đóng</button><button className="primary" disabled={busy || !path.trim() || !slug.trim()}>Kiểm tra gói local</button></div>
    </form>
  </section></div>;
}
