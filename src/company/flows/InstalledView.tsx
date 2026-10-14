import { useState } from 'react';
import type { CompanyApi } from '../api';
import type { Backup, Installation, OperationView, Project } from '../types';
import { displayPath, errorMessage, isSessionError } from '../types';
import { UpdateDialog } from './UpdateDialog';
import type { UpdateResult } from './UpdateDialog';
import { BackupsView } from './BackupsView';
import { LifecycleDialog } from './LifecycleDialog';

const contentLabels = { original: 'Nguyên bản', customized: 'Đã tùy chỉnh', missing: 'Không tìm thấy bản cài', unknown: 'Chưa kiểm tra được nội dung' };
const storeLabels = { current: 'Gói hiện tại trên Store', new_package: 'Có gói mới', unknown: 'Chưa kiểm tra được Store', unavailable: 'Skill không còn khả dụng trên Store' };
export function InstalledView({ api, installations, projects, onChanged, signedIn = true, onExpired, selectedIds, onSelectionChange, onReinstall }: { api: CompanyApi; installations: Installation[]; projects: Project[]; onChanged: () => void; signedIn?: boolean; onExpired?: (message: string) => void; selectedIds?: string[]; onSelectionChange?: (ids: string[]) => void; onReinstall?: (item: Installation) => void }) {
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const [localSelected, setLocalSelected] = useState<string[]>([]);
  const [operation, setOperation] = useState<OperationView>();
  const [currentId, setCurrentId] = useState('');
  const [queue, setQueue] = useState<string[]>([]);
  const [results, setResults] = useState<Record<string, { status: UpdateResult; message: string }>>({});
  const [backups, setBackups] = useState<Backup[]>();
  const [lifecycle, setLifecycle] = useState<OperationView>();
  const selected = (selectedIds ?? localSelected).filter(id => installations.find(i => i.id === id)?.state !== 'removed');
  function toggle(id: string) { const ids = selected.includes(id) ? selected.filter(i => i !== id) : [...selected, id]; setLocalSelected(ids); onSelectionChange?.(ids); }
  function failure(e: unknown) { setError(errorMessage(e)); if (isSessionError(e)) { setOperation(undefined); setQueue([]); onExpired?.(errorMessage(e)); } }
  async function open(id: string, kind: 'skill_file' | 'folder') { setError(''); try { await api.openInstallation(id, kind); } catch (e) { setError(errorMessage(e)); } }
  async function check() { setBusy(true); setError(''); try { await api.checkUpdates((selected.length ? selected : installations.map(i => i.id)).filter(id => installations.find(i => i.id === id)?.state !== 'removed')); onChanged(); } catch (e) { failure(e); } finally { setBusy(false); } }
  async function prepare(id: string) {
    setBusy(true); setError(''); setCurrentId(id); setBackups(undefined);
    try { setOperation(await api.prepareUpdate(id)); }
    catch (e) { setResults(r => ({ ...r, [id]: { status: 'failed', message: errorMessage(e) } })); failure(e); }
    finally { setBusy(false); }
  }
  async function start(ids: string[]) { if (!ids.length) return; setQueue(ids.slice(1)); await prepare(ids[0]); }
  async function next(result: UpdateResult) {
    setResults(r => ({ ...r, [currentId]: { status: result, message: result === 'applied' ? 'Đã cập nhật' : result === 'kept' ? 'Giữ bản đang dùng' : 'Chưa hoàn tất' } }));
    setOperation(undefined); onChanged();
    if (queue.length) { const [id, ...rest] = queue; setQueue(rest); await prepare(id); }
  }
  async function showBackups(id: string) { setError(''); try { setBackups(await api.backupsList(id)); } catch (e) { failure(e); } }
  async function remove(id: string) { setBusy(true); setError(''); try { setLifecycle(await api.prepareRemove(id)); } catch (e) { setError(errorMessage(e)); } finally { setBusy(false); } }
  return <>
    <div className="row-actions"><button className="secondary" onClick={onChanged} disabled={busy}>Làm mới nội dung</button><button className="secondary" onClick={check} disabled={busy || !signedIn || !installations.length}>Kiểm tra gói mới</button><button className="primary" onClick={() => start(selected)} disabled={busy || !signedIn || !selected.length}>Xem cập nhật các đích đã chọn ({selected.length})</button></div>
    {!signedIn && <p>Đăng nhập Store để kiểm tra hoặc cập nhật; vẫn có thể mở các bản cài trên máy.</p>}
    {error && <div className="error-banner" role="alert">{error}</div>}
    {!operation && queue.length > 0 && <button className="secondary" onClick={() => { const [id, ...rest] = queue; setQueue(rest); prepare(id); }}>Tiếp tục với đích còn lại</button>}
    {installations.length === 0 && <section className="company-card"><p>Chưa có skill được cài bởi ứng dụng.</p></section>}
    {installations.map(item => { const removed = item.state === 'removed'; const scope = item.projectId ? `Dự án: ${projects.find(p => p.id === item.projectId)?.name ?? item.projectId}` : 'AIPRO Global'; return <article className="company-card installation-card" key={item.id}>
      {!removed && <label className="checkbox-label"><input type="checkbox" checked={selected.includes(item.id)} onChange={() => toggle(item.id)} />Chọn {item.name} tại {scope}</label>}<h2>{item.name}</h2><span className="status-tag">{scope}</span><span className={`status-tag ${item.contentState ?? 'unknown'}`}>{removed ? 'Đã gỡ' : contentLabels[item.contentState ?? 'unknown']}</span>{!removed && <span className="status-tag">{storeLabels[item.storeState ?? 'unknown']}</span>}<p className="target-path">{displayPath(item.target)}</p>{!removed && item.checkedAt && <p>Store kiểm tra lúc {new Date(item.checkedAt).toLocaleString('vi-VN')}</p>}{!removed && item.storeError && <p>{item.storeError}</p>}
      <div className="row-actions">{!removed && <><button className="secondary" onClick={() => open(item.id, 'skill_file')}>Mở SKILL.md</button><button className="secondary" onClick={() => open(item.id, 'folder')}>Mở thư mục</button><button className="primary" onClick={() => start([item.id])} disabled={busy || !signedIn || item.contentState === 'missing' || item.contentState === 'unknown'}>Xem cập nhật</button><button className="secondary" disabled={busy} onClick={() => remove(item.id)}>Gỡ bản cài</button></>}<button className="secondary" onClick={() => showBackups(item.id)}>Bản lưu</button></div>
      {!removed && item.contentState === 'missing' && onReinstall && <button className="secondary" disabled={!signedIn || busy} onClick={() => onReinstall(item)}>Cài lại từ Store</button>}
      {results[item.id] && <p role="status">{results[item.id].message}{results[item.id].status === 'failed' && <button className="secondary" disabled={busy || !signedIn} onClick={() => start([item.id])}>Thử lại đích này</button>}</p>}
    </article>; })}
    {backups && <BackupsView api={api} backups={backups} onPrepared={setLifecycle} />}
    {lifecycle && <LifecycleDialog key={lifecycle.id} api={api} initial={lifecycle} onChanged={onChanged} onClose={() => { setLifecycle(undefined); setBackups(undefined); onChanged(); }} />}
    {operation && <UpdateDialog key={operation.id} api={api} initial={operation} onClose={next} onApplied={onChanged} onExpired={message => failure({ code: 'SESSION_EXPIRED', message })} />}
  </>;
}
