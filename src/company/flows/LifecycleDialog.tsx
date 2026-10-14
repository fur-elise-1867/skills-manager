import type { CompanyApi } from '../api';
import type { OperationView } from '../types';
import { displayPath, errorMessage } from '../types';
import { useCompanyOperation } from './useCompanyOperation';

export function LifecycleDialog({ api, initial, onChanged, onClose }: { api: CompanyApi; initial: OperationView; onChanged: () => void; onClose: () => void }) {
  const { operation, setOperation, error, setError, terminal } = useCompanyOperation(api, initial, onChanged, () => {});
  const removing = operation.kind === 'remove';
  async function apply() {
    setError('');
    try { setOperation(await api.applyLifecycle(operation.id)); }
    catch (e) { setError(errorMessage(e)); }
  }
  async function close() {
    if (terminal) { onClose(); return; }
    try { const next = await api.cancelOperation(operation.id); setOperation(next); if (next.state === 'cancelled') onClose(); }
    catch (e) { setError(errorMessage(e)); }
  }
  return <div className="modal-overlay"><section role="dialog" aria-modal="true" aria-label={removing ? 'Gỡ bản cài' : 'Khôi phục bản lưu'} className="company-modal">
    <h2>{removing ? 'Gỡ bản cài này' : 'Khôi phục bản lưu'}</h2><p className="target-path">{displayPath(operation.target)}</p>
    <p>{removing ? 'App lưu đầy đủ nội dung hiện tại trước khi gỡ. Các đích khác và thư mục dự án được giữ.' : 'Khôi phục nội dung cùng baseline của snapshot. App lưu bản hiện tại trước khi thay; đích trống không tạo bản lưu rỗng.'}</p>
    {operation.customized && <p className="warning-banner">Bản hiện tại đã tùy chỉnh. Nội dung này sẽ được giữ trong bản lưu trước thao tác.</p>}
    <p>Thao tác cục bộ, dùng được offline. MCP trong snapshot là thông tin lịch sử.</p>
    {(error || operation.error) && <p role="alert" className="error-banner">{error || operation.error?.message}</p>}
    {operation.state === 'succeeded' && <p role="status">{removing ? 'Đã gỡ bản cài; bản lưu vẫn có thể khôi phục.' : 'Đã khôi phục nội dung và baseline của bản lưu.'}</p>}
    {!terminal && operation.state !== 'awaiting_decision' && <p role="status">Đang lưu và xử lý dữ liệu…</p>}
    <div className="row-actions"><button className="secondary" onClick={close} disabled={operation.state === 'committing'}>{terminal ? 'Đóng' : 'Hủy'}</button>
      {operation.state === 'awaiting_decision' && <button className="primary" onClick={apply}>{removing ? 'Backup và gỡ' : 'Backup và khôi phục'}</button>}
    </div>
  </section></div>;
}
