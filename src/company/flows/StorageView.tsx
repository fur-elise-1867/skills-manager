import { useCallback, useEffect, useState } from 'react';
import type { CompanyApi } from '../api';
import type { StorageSummary } from '../types';
import { displayPath, errorMessage } from '../types';

type DeleteChoice = { kind: 'backup'; id: string } | { kind: 'packages'; hashes: string[] };
function size(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  const unit = bytes < 1048576 ? 'KiB' : 'MiB';
  return `${(bytes / (unit === 'KiB' ? 1024 : 1048576)).toLocaleString('vi-VN', { maximumFractionDigits: 2 })} ${unit}`;
}
export function StorageView({ api }: { api: CompanyApi }) {
  const [summary, setSummary] = useState<StorageSummary>(); const [selected, setSelected] = useState<string[]>([]);
  const [choice, setChoice] = useState<DeleteChoice>(); const [busy, setBusy] = useState(false); const [error, setError] = useState('');
  const refresh = useCallback(async () => {
    try { const value = await api.storageStatus(); setSummary(value); setSelected(ids => ids.filter(id => value.packages.some(p => p.hash === id && !p.referenced))); }
    catch (e) { setError(errorMessage(e)); }
  }, [api]);
  useEffect(() => { refresh(); }, [refresh]);
  async function remove() {
    if (!choice) return; setBusy(true); setError('');
    try { if (choice.kind === 'backup') await api.deleteBackup(choice.id); else await api.cleanupPackages(choice.hashes); setChoice(undefined); }
    catch (e) { setError(errorMessage(e)); }
    finally { await refresh(); setBusy(false); }
  }
  return <section className="company-card"><h2>Dung lượng và bản lưu</h2><p>Chỉ xóa khi bạn chọn và xác nhận. Gói đang được bản cài, backup hoặc thao tác tham chiếu được giữ.</p>
    {error && <p role="alert" className="error-banner">{error}</p>}
    <button className="secondary" onClick={() => { setError(''); refresh(); }} disabled={busy}>Làm mới dung lượng</button>
    {!summary ? <p role="status">Đang kiểm tra dung lượng…</p> : <><dl><dt>Package</dt><dd>{size(summary.packageBytes)}</dd><dt>Backup</dt><dd>{size(summary.backupBytes)}</dd><dt>Gói không còn dùng</dt><dd>{size(summary.unreferencedPackageBytes)}</dd></dl>
      <h3>Gói cục bộ</h3>{summary.packages.length === 0 && <p>Chưa có gói lưu trên máy.</p>}
      {summary.packages.map(p => <label className="checkbox-label" key={p.hash}><input type="checkbox" aria-label={`Chọn gói ${p.hash}`} disabled={p.referenced || busy} checked={selected.includes(p.hash)} onChange={() => setSelected(ids => ids.includes(p.hash) ? ids.filter(id => id !== p.hash) : [...ids, p.hash])} /><span>{p.hash.slice(0, 12)} · {size(p.bytes)} · {p.referenced ? 'Đang được tham chiếu' : 'Có thể dọn'}</span></label>)}
      <button className="secondary" disabled={busy || !selected.length} onClick={() => setChoice({ kind: 'packages', hashes: [...selected] })}>Dọn các gói đã chọn ({selected.length})</button>
      <h3>Bản lưu</h3>{summary.backups.length === 0 && <p>Chưa có bản lưu.</p>}
      {summary.backups.map(({ backup: b, bytes, inUse }) => <article key={b.id}><p>{b.installation.name} · {new Date(b.createdAt).toLocaleString('vi-VN')} · {size(bytes)}</p><p className="target-path">{displayPath(b.installation.target)}</p>{inUse && <p>Đang dùng cho thao tác hoặc recovery.</p>}<button className="secondary" disabled={busy || inUse} onClick={() => setChoice({ kind: 'backup', id: b.id })}>Xóa bản lưu</button></article>)}
    </>}
    {choice && <div className="modal-overlay"><section className="company-modal" role="alertdialog" aria-modal="true" aria-label="Xác nhận xóa dữ liệu"><h2>Xóa dữ liệu đã chọn?</h2><p>{choice.kind === 'backup' ? 'Sau khi xóa, snapshot này không còn dùng để khôi phục được.' : `${choice.hashes.length} gói không còn dùng sẽ được xóa. App kiểm tra lại reference trước khi dọn.`}</p><p className="target-path">{choice.kind === 'backup' ? summary?.backups.find(b => b.backup.id === choice.id)?.backup.path : choice.hashes.join(', ')}</p><div className="dialog-actions"><button className="secondary" disabled={busy} onClick={() => setChoice(undefined)}>Hủy</button><button className="primary" disabled={busy} onClick={remove}>Xác nhận xóa</button></div></section></div>}
  </section>;
}
