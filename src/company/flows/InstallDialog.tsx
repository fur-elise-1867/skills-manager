import { useEffect, useRef, useState } from 'react';
import type { CompanyApi } from '../api';
import type { CompanyError, OperationView, Skill } from '../types';
import { errorMessage } from '../types';

const labels: Record<string, string> = { downloading: 'Đang tải gói…', validating: 'Đang kiểm tra gói…', staging: 'Đang chuẩn bị bản cài…', committing: 'Đang lưu bản cài…' };
export function InstallDialog({ api, skill, initial, onClose, onInstalled, onExpired }: { api: CompanyApi; skill: Skill; initial: OperationView; onClose: () => void; onInstalled: () => void; onExpired: (message: string) => void }) {
  const [operation, setOperation] = useState(initial);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [bypassed, setBypassed] = useState(false);
  const notified = useRef(false);
  const terminal = ['succeeded', 'failed', 'cancelled', 'recovery_required'].includes(operation.state);

  useEffect(() => {
    if (operation.state === 'awaiting_decision' || terminal) return;
    let active = true;
    const timer = setTimeout(() => api.operationStatus(operation.id).then(next => { if (active) setOperation(next); }).catch(e => { if (active) setError(errorMessage(e)); }), 300);
    return () => { active = false; clearTimeout(timer); };
  }, [api, operation, terminal]);

  useEffect(() => {
    if (operation.state === 'succeeded' && !notified.current) { notified.current = true; onInstalled(); }
    if (operation.error?.code === 'SESSION_EXPIRED' || operation.error?.code === 'UNAUTHENTICATED') onExpired(errorMessage(operation.error));
  }, [operation, onInstalled, onExpired]);

  async function proceed(bypass: boolean) {
    setBusy(true); setError(''); setBypassed(bypass);
    try { setOperation(await api.confirmInstallation(operation.id, bypass)); }
    catch (e) {
      if (['SESSION_EXPIRED', 'UNAUTHENTICATED'].includes((e as CompanyError)?.code)) onExpired(errorMessage(e));
      else setError(errorMessage(e));
    } finally { setBusy(false); }
  }
  async function stop() {
    setBusy(true);
    try { await api.cancelOperation(operation.id); onClose(); }
    catch (e) { setError(errorMessage(e)); }
    finally { setBusy(false); }
  }
  const warning = operation.mcp.state === 'missing' || operation.mcp.state === 'unknown';
  return <div className="modal-overlay"><section role="dialog" aria-label="Cài skill" aria-modal="true" className="company-modal">
    <div className="eyebrow">CÀI ĐẶT / AIPRO GLOBAL</div><h2>{skill.name}</h2><p className="target-path">{operation.target}</p>
    {operation.state === 'awaiting_decision' && <><h3>{operation.mcp.state === 'missing' ? 'Chưa có MCP trong cấu hình' : operation.mcp.state === 'unknown' ? 'Chưa kiểm tra được MCP' : operation.mcp.state === 'undeclared' ? 'Skill chưa khai báo yêu cầu MCP' : operation.mcp.state === 'not_required' ? 'Không yêu cầu MCP' : 'Đã có MCP trong cấu hình'}</h3>
      {operation.mcp.missing.length > 0 && <ul>{operation.mcp.missing.map(name => <li key={name}>{name}</li>)}</ul>}{operation.mcp.reason && <p>{operation.mcp.reason}</p>}
      {operation.mcpPath && <p className="target-path">{operation.mcpPath}</p>}
      {warning && <p>Bạn có thể cài MCP thủ công, sau đó thử cài skill lại; hoặc bỏ qua cảnh báo cho lần cài này.</p>}
      <div className="dialog-actions"><button className="secondary" onClick={stop} disabled={busy}>Dừng cài đặt</button><button className="primary" onClick={() => proceed(warning)} disabled={busy}>{warning ? 'Bỏ qua và cài tiếp' : 'Cài skill'}</button></div></>}
    {!terminal && operation.state !== 'awaiting_decision' && <><p role="status">{labels[operation.state] || 'Đang cài…'}</p><progress max={Math.max(operation.totalBytes, 1)} value={operation.progressBytes} /><button className="secondary" onClick={stop} disabled={operation.state === 'committing'}>Hủy</button></>}
    {operation.state === 'succeeded' && <><div className="success-banner">{bypassed ? 'Cài xong; đã bỏ qua cảnh báo MCP của lần này.' : 'Đã cài skill vào AIPRO Global.'}</div><p>Bản cài được quản lý tại đường dẫn trên.</p><button className="primary" onClick={onClose}>Hoàn tất</button></>}
    {(error || operation.error) && <div role="alert" className="error-banner">{error || operation.error?.message}</div>}
    {operation.state === 'failed' && <><p>Thao tác chưa hoàn tất. Đóng hộp thoại rồi bấm cài lại để kiểm tra thông tin và MCP mới.</p><button className="secondary" onClick={onClose}>Đóng</button></>}
    {operation.state === 'recovery_required' && <><p>Dữ liệu thao tác đã được giữ để khôi phục. Kiểm tra đường dẫn đích và thử mở lại ứng dụng.</p><button className="secondary" onClick={onClose}>Đóng</button></>}
  </section></div>;
}
