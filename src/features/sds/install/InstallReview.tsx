import { useEffect, useRef, useState } from "react";
import type { PreparedOperation } from "../contracts";

export default function InstallReview({operation,pending,onConfirm,onCheckAgain,onCancel}:{operation:PreparedOperation;pending:boolean;onConfirm:(id:string,ackMcp:boolean)=>void;onCheckAgain:()=>void;onCancel:()=>void}) {
  const dialog = useRef<HTMLDivElement>(null);
  const cancel = useRef<HTMLButtonElement>(null);
  const [acknowledgedId,setAcknowledgedId]=useState<string|null>(null);
  useEffect(()=>{
    const previous = document.activeElement;
    cancel.current?.focus();
    return ()=>{if(previous instanceof HTMLElement && previous.isConnected) previous.focus();};
  },[]);
  const warning = operation.mcp.state !== "configured" && operation.mcp.state !== "notRequired";
  const restore=operation.kind==="restore",update=operation.kind==="update";
  const title=restore?"Restore snapshot":update?"Review update":"Install skill";
  const action=restore?"Back up current and restore":update?"Back up and update":operation.requiresMcpAck?"Install anyway":"Install skill";
  return <div className="sds-modal-backdrop"><div ref={dialog} className="sds-install-review" role="dialog" aria-modal="true" aria-labelledby="sds-install-title" aria-describedby="sds-install-path" onKeyDown={(event)=>{
    if(event.key === "Escape" && !pending) { event.preventDefault(); onCancel(); }
    if(event.key !== "Tab") return;
    const controls = Array.from(dialog.current?.querySelectorAll<HTMLElement>('button:not(:disabled),a[href],input:not(:disabled),[tabindex="0"]') ?? []);
    const first = controls[0]; const last = controls[controls.length-1];
    if(event.shiftKey && document.activeElement === first) { event.preventDefault(); last?.focus(); }
    else if(!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus(); }
  }}>
    <h2 id="sds-install-title">{title}</h2><p className="sds-review-subtitle">{operation.release.version} · {operation.release.installName}</p>
    <h3>AIPro · {operation.target.workspaceId==="global"?"Global":"Project"}</h3><p id="sds-install-path" className="sds-target-path">{operation.target.path}</p>
    <p>The skill and its resources will be copied to this destination.</p>
    {restore && <div className="sds-lifecycle-notice"><p>Saved local edits are restored as local edits, with the original release baseline.</p><p>The recorded target will be replaced. Any modified current contents must be saved in a verified snapshot first. A snapshot or replacement failure stops restore.</p></div>}
    {update && <div className="sds-lifecycle-notice"><p>Only this selected destination will be updated. Local edits are saved in a verified snapshot before replacement. If backup fails, your current copy stays in place.</p><h3>Release changes</h3><p className="sds-changelog">{operation.release.changelog || "No release notes provided."}</p></div>}
    {operation.conflict && <p role="alert" className="sds-error">{operation.conflict}</p>}
    <section className={`sds-mcp-report ${warning ? "sds-mcp-warning" : ""}`} aria-label="MCP check">
      <h3>{operation.mcp.state === "configured" ? "Configuration found" : operation.mcp.state === "notRequired" ? "No MCP servers required" : "MCP configuration needs attention"}</h3>
      {operation.mcp.state === "unknown" && <p>This release has not declared its MCP requirements.</p>}
      {operation.mcp.state === "unverifiable" && <p>The MCP configuration could not be verified.</p>}
      {operation.mcp.missing.length > 0 && <><p>Missing or incomplete configuration:</p><ul>{operation.mcp.missing.map(name=><li key={name}>{name}</li>)}</ul></>}
      {operation.mcp.reason && <p>{operation.mcp.reason}</p>}
      <p>Checking configuration does not verify that MCP servers are running.</p>
      {warning && <p>You can configure MCP in AIPro, check again, or continue installing this skill.</p>}
      <button className="sds-secondary-button" disabled={pending} onClick={onCheckAgain}>{pending ? "Checking…" : "Check again"}</button>
      {(update || restore) && operation.requiresMcpAck && <label className="sds-check-row"><input type="checkbox" disabled={pending} checked={acknowledgedId===operation.id} onChange={event=>setAcknowledgedId(event.target.checked?operation.id:null)}/>I acknowledge the MCP warnings for this operation</label>}
    </section>
    <div className="sds-review-actions"><button ref={cancel} className="sds-secondary-button" disabled={pending} onClick={onCancel}>Cancel</button><button className="sds-primary-button" disabled={pending || !operation.canExecute || operation.kind==="remove" || ((update || restore) && operation.requiresMcpAck && acknowledgedId!==operation.id)} onClick={()=>onConfirm(operation.id,operation.requiresMcpAck)}>{action}</button></div>
  </div></div>;
}
