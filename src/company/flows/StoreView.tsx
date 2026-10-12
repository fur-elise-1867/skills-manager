import { useEffect, useState } from 'react';
import ReactMarkdown from 'react-markdown';
import type { CompanyApi } from '../api';
import type { CompanyError, OperationView, Skill, SkillPage } from '../types';
import { errorMessage } from '../types';
import { InstallDialog } from './InstallDialog';

export function StoreView({ api, onExpired, onInstalled }: { api: CompanyApi; onExpired: (message: string) => void; onInstalled: () => void }) {
  const [query, setQuery] = useState('');
  const [pageNumber, setPageNumber] = useState(1);
  const [page, setPage] = useState<SkillPage>();
  const [detail, setDetail] = useState<Skill>();
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [operation, setOperation] = useState<OperationView>();
  const [preparing, setPreparing] = useState(false);
  const [revision, setRevision] = useState(0);
  const [retryUntil, setRetryUntil] = useState(0);
  useEffect(() => {
    if (!retryUntil) return;
    const timer = setTimeout(() => setRetryUntil(0), Math.max(0, retryUntil - Date.now()));
    return () => clearTimeout(timer);
  }, [retryUntil]);

  useEffect(() => {
    let active = true;
    const timer = setTimeout(() => {
      setLoading(true); setError('');
      api.storeSearch(query, pageNumber).then(value => { if (active) setPage(value); }).catch((e: CompanyError) => {
        if (!active) return;
        if (e.code === 'SESSION_EXPIRED' || e.code === 'UNAUTHENTICATED') onExpired(errorMessage(e));
        else { setError(errorMessage(e)); if (e.code === 'RATE_LIMITED' && e.retryAfterSeconds) setRetryUntil(Date.now() + e.retryAfterSeconds * 1000); }
      }).finally(() => { if (active) setLoading(false); });
    }, 300);
    return () => { active = false; clearTimeout(timer); };
  }, [api, query, pageNumber, onExpired, revision]);

  async function openDetail(id: string) {
    setError('');
    try { setDetail(await api.storeDetail(id)); }
    catch (e) {
      const code = (e as CompanyError)?.code;
      if (code === 'SESSION_EXPIRED' || code === 'UNAUTHENTICATED') onExpired(errorMessage(e));
      else setError(errorMessage(e));
    }
  }
  async function prepare() {
    if (!detail) return;
    setPreparing(true); setError('');
    try { const prepared = await api.prepareInstallation(detail.id); if (prepared.skill) setDetail(prepared.skill); setOperation(prepared); }
    catch (e) { setError(errorMessage(e)); }
    finally { setPreparing(false); }
  }

  return <>
    <div className="store-toolbar"><input aria-label="Tìm kiếm skill" placeholder="Tìm theo tên hoặc công dụng skill…" disabled={retryUntil > 0} value={query} onChange={e => { setQuery(e.target.value); setPageNumber(1); setDetail(undefined); }} /><span>{page?.pagination.total ?? 0} skill</span></div>
    {error && <div role="alert" className="error-banner">{error}{page && ' · Đang hiển thị dữ liệu lần tải trước.'}{retryUntil > 0 && ' Vui lòng chờ trước khi thử lại.'}<button disabled={retryUntil > 0} onClick={() => setRevision(value => value + 1)}>Thử lại</button></div>}
    {loading && <p role="status">Đang tìm skill…</p>}
    {!loading && page?.items.length === 0 && <section className="company-card"><h2>Không tìm thấy skill</h2><p>Thử một từ khóa khác để tìm skill phù hợp.</p></section>}
    <div className="skill-grid">{page?.items.map(skill => <article className="company-card skill-card" key={skill.id}>
      <div className="skill-icon">↗</div><span className="format-tag">{skill.package.format.toUpperCase()}</span><h2>{skill.name}</h2><p>{skill.descriptionExcerpt}</p>
      <button aria-label={`Xem chi tiết ${skill.name}`} className="secondary" onClick={() => openDetail(skill.id)} disabled={loading || !!error}>Xem chi tiết →</button>
    </article>)}</div>
    {page && page.pagination.totalPages > 1 && <div className="pagination"><button disabled={pageNumber <= 1 || loading} onClick={() => setPageNumber(n => n - 1)}>Trang trước</button><span>{pageNumber} / {page.pagination.totalPages}</span><button disabled={pageNumber >= page.pagination.totalPages || loading} onClick={() => setPageNumber(n => n + 1)}>Trang sau</button></div>}
    {detail && !operation && <div className="modal-overlay"><section role="dialog" aria-label="Chi tiết skill" aria-modal="true" className="company-modal"><button className="close-modal" onClick={() => setDetail(undefined)} aria-label="Đóng chi tiết">×</button><div className="eyebrow">SKILL STORE / AIPRO</div><h2>{detail.name}</h2>
      <div className="skill-description"><ReactMarkdown skipHtml components={{ img: () => null, a: ({ children }) => <span>{children}</span> }}>{detail.description}</ReactMarkdown></div><div className="package-line">{detail.package.format.toUpperCase()} · {(detail.package.sizeBytes / 1024).toFixed(1)} KiB</div>
      <div className="dialog-actions"><button className="primary" disabled={preparing || !!error} onClick={prepare}>{preparing ? 'Đang kiểm tra…' : 'Cài vào AIPRO Global'}</button></div>
    </section></div>}
    {operation && detail && <InstallDialog api={api} skill={detail} initial={operation} onClose={() => { setOperation(undefined); setDetail(undefined); }} onExpired={onExpired} onInstalled={onInstalled} />}
  </>;
}
