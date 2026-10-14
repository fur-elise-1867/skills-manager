import { useState } from 'react';
import type { CompanyApi } from '../api';
import type { Backup, OperationView } from '../types';
import { displayPath, errorMessage } from '../types';

export function BackupsView({ api, backups, onPrepared }: { api: CompanyApi; backups: Backup[]; onPrepared: (op: OperationView) => void }) {
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  async function restore(backup: Backup) {
    setBusy(true); setError('');
    try { onPrepared(await api.prepareRestore(backup.id)); }
    catch (e) { setError(errorMessage(e)); }
    finally { setBusy(false); }
  }
  return <section className="company-card"><h2>Bản lưu ({backups.length})</h2>
    {error && <p role="alert" className="error-banner">{error}</p>}
    {backups.length === 0 && <p>Chưa có bản lưu cho đích này.</p>}
    {backups.map(b => <article key={b.id}><p>{new Date(b.createdAt).toLocaleString('vi-VN')} · {b.installation.name} · {b.reason === 'remove' ? 'Trước khi gỡ' : b.reason === 'restore' ? 'Trước khi khôi phục' : 'Trước cập nhật'}</p>
      <p className="target-path">Đích: {displayPath(b.installation.target)}</p><p className="target-path">Bản lưu: {displayPath(b.path)}</p><p>Baseline: {b.installation.packageHash.slice(0, 12)}</p>
      <button className="secondary" onClick={() => restore(b)} disabled={busy}>Khôi phục bản lưu</button>
    </article>)}
  </section>;
}
