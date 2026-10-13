import { useState } from 'react';
import type { CompanyApi } from '../api';
import type { OperationView } from '../types';
import { errorMessage, isSessionError } from '../types';
import { useCompanyOperation } from './useCompanyOperation';

export type UpdateResult = 'applied' | 'kept' | 'failed';
const changeLabels = { added: 'Thêm', modified: 'Sửa', deleted: 'Xóa' };
export function UpdateDialog({ api, initial, onClose, onApplied, onExpired }: { api: CompanyApi; initial: OperationView; onClose: (result: UpdateResult) => void; onApplied: () => void; onExpired: (message: string) => void }) {
  const { operation: op, setOperation: setOp, error, setError, terminal } = useCompanyOperation(api, initial, onApplied, onExpired);
  const [busy, setBusy] = useState(false);
  const [replace, setReplace] = useState(false);
  function failed(e: unknown) { if (isSessionError(e)) onExpired(errorMessage(e)); else setError(errorMessage(e)); }
  async function preview() { setBusy(true); setError(''); try { setOp(await api.confirmInstallation(op.id, op.mcp.state === 'missing' || op.mcp.state === 'unknown')); } catch (e) { failed(e); } finally { setBusy(false); } }
  async function apply() { setBusy(true); setError(''); try { setOp(await api.applyUpdate(op.id, replace)); } catch (e) { failed(e); } finally { setBusy(false); } }
  async function keep() { setBusy(true); try { const next = await api.cancelOperation(op.id); if (next.state === 'cancelled') onClose('kept'); else setOp(next); } catch (e) { failed(e); } finally { setBusy(false); } }
  async function open(side: 'current' | 'incoming', path = '') { try { await api.openPreview(op.id, side, path); } catch (e) { failed(e); } }
  const warning = op.mcp.state === 'missing' || op.mcp.state === 'unknown';
  return <div className="modal-overlay"><section className="company-modal" role="dialog" aria-modal="true" aria-label="Cập nhật skill"><div className="eyebrow">CẬP NHẬT MỘT ĐÍCH / AIPRO</div><h2>{op.skill?.name || 'Cập nhật skill'}</h2><p className="target-path">{op.target}</p>
    {op.state === 'awaiting_decision' && !op.previewReady && <>
      <h3>{warning ? 'Cần xử lý cảnh báo MCP' : 'Kiểm tra MCP trước khi xem thay đổi'}</h3><p>MCP yêu cầu: {op.mcp.required.join(', ') || (op.mcp.state === 'not_required' ? 'Không yêu cầu' : 'Chưa khai báo')}</p><p>Đã có trong cấu hình: {op.mcp.present.join(', ') || 'Chưa có tên nào'}</p>{op.mcp.missing.length > 0 && <p>Thiếu: {op.mcp.missing.join(', ')}</p>}{op.mcp.reason && <p>{op.mcp.reason}</p>}
      {warning && <p>Bổ sung MCP thủ công rồi thử lại, hoặc bỏ qua cho lần cập nhật này.</p>}
      <div className="dialog-actions"><button className="secondary" disabled={busy} onClick={keep}>Dừng cập nhật</button><button className="primary" disabled={busy} onClick={preview}>{warning ? 'Bỏ qua và xem thay đổi' : 'Tải để xem thay đổi'}</button></div>
    </>}
    {op.state === 'awaiting_decision' && op.previewReady && <><h3>Thay đổi so với bản đang dùng</h3>
      {op.changes?.length ? <ul className="file-changes">{op.changes.map(change => <li key={change.path}><span>{changeLabels[change.kind]}: <strong>{change.path}</strong></span><div>{change.current && <span>Bản đang dùng: {change.current.kind === 'directory' ? 'Thư mục' : `${change.current.kind === 'text' ? 'Text' : 'Nhị phân'} · ${change.current.sizeBytes} bytes`}</span>}{change.incoming && <span>Bản Store: {change.incoming.kind === 'directory' ? 'Thư mục' : `${change.incoming.kind === 'text' ? 'Text' : 'Nhị phân'} · ${change.incoming.sizeBytes} bytes`}</span>}{change.current?.kind === 'text' && <button onClick={() => open('current', change.path)}>Mở bản đang dùng</button>}{change.incoming?.kind === 'text' && <button onClick={() => open('incoming', change.path)}>Mở bản Store</button>}</div></li>)}</ul> : <p>Gói tải thay đổi nhưng nội dung so sánh giống nhau.</p>}
      <div className="row-actions"><button className="secondary" onClick={() => open('current')}>Thư mục bản đang dùng</button><button className="secondary" onClick={() => open('incoming')}>Thư mục bản Store</button></div>
      <p>Bản hiện tại được lưu đầy đủ trước khi thay. Các đích khác giữ nguyên.</p><p>Các bản mở để xem là bản sao; chỉnh sửa ở đây không được áp dụng vào bản cài.</p>
      {op.customized && <label className="checkbox-label"><input type="checkbox" checked={replace} onChange={e => setReplace(e.target.checked)} />Tôi đồng ý thay bản đã tùy chỉnh bằng gói Store</label>}
      <div className="dialog-actions"><button className="secondary" disabled={busy} onClick={keep}>Giữ bản đang dùng</button><button className="primary" disabled={busy || (!!op.customized && !replace)} onClick={apply}>Backup và cập nhật</button></div>
    </>}
    {!terminal && op.state !== 'awaiting_decision' && <><p role="status">{op.state === 'committing' ? 'Đang thay bản cài…' : op.state === 'staging' ? 'Đang lưu bản trước và chuẩn bị cập nhật…' : 'Đang tải và kiểm tra gói…'}</p><progress value={op.progressBytes} max={Math.max(op.totalBytes, 1)} /><button className="secondary" disabled={busy || op.state === 'committing'} onClick={keep}>Hủy</button></>}
    {op.state === 'succeeded' && <><div className="success-banner">Đã cập nhật đích này; bản trước đã được lưu.</div><p>Kết quả cài file không xác nhận AIPRO hoặc MCP đã chạy.</p><button className="primary" onClick={() => onClose('applied')}>Hoàn tất</button></>}
    {(error || op.error) && <div className="error-banner" role="alert">{error || op.error?.message}</div>}
    {['failed', 'cancelled', 'recovery_required'].includes(op.state) && <><p>{op.state === 'recovery_required' ? 'Dữ liệu đã được giữ để kiểm tra. Mở lại app để phục hồi.' : op.state === 'cancelled' ? 'Đã hủy trước khi thay bản cài.' : 'Giữ dữ liệu hiện tại. Thử lại sẽ lấy detail và kiểm tra MCP mới.'}</p><button className="secondary" onClick={() => onClose(op.state === 'cancelled' ? 'kept' : 'failed')}>Đóng</button></>}
  </section></div>;
}
