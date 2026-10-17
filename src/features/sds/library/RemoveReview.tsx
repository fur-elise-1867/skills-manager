import {useEffect,useRef} from "react";
import type {PreparedOperation} from "../contracts";
export default function RemoveReview({operation,pending,onConfirm,onCancel}:{operation:PreparedOperation;pending:boolean;onConfirm:(id:string,ack:boolean)=>void;onCancel:()=>void}){
  const dialog=useRef<HTMLDivElement>(null),cancel=useRef<HTMLButtonElement>(null);
  useEffect(()=>{const previous=document.activeElement;cancel.current?.focus();return()=>{if(previous instanceof HTMLElement && previous.isConnected)previous.focus();};},[]);
  const missing=operation.expectedFingerprint===null;
  return <div className="sds-modal-backdrop"><div className="sds-install-review" ref={dialog} role="dialog" aria-modal="true" aria-labelledby="sds-remove-title" onKeyDown={event=>{
    if(event.key==="Escape" && !pending){event.preventDefault();onCancel();}
    if(event.key!=="Tab")return;
    const controls=Array.from(dialog.current?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)")??[]);const first=controls[0],last=controls.at(-1);
    if(event.shiftKey && document.activeElement===first){event.preventDefault();last?.focus();}else if(!event.shiftKey && document.activeElement===last){event.preventDefault();first?.focus();}
  }}>
    <h2 id="sds-remove-title">Remove skill</h2><p className="sds-review-subtitle">{operation.release.installName} · Release {operation.release.version}</p>
    <h3>AIPro · Global</h3><p className="sds-target-path">{operation.target.path}</p>
    <p>{missing ? "This destination is missing. Only its installation record will be removed." : "Only this skill folder will be removed from the selected destination."}</p>
    <p>Your Library copy and other destinations will be kept. AIPro configuration will stay unchanged.</p>
    {operation.conflict && <p role="alert" className="sds-error">{operation.conflict}</p>}
    {!operation.canExecute && <p>Keep your current files. Copy them to a safe location before resolving local changes; force removal is unavailable in this pilot.</p>}
    <div className="sds-review-actions"><button ref={cancel} className="sds-secondary-button" disabled={pending} onClick={onCancel}>Cancel</button><button className="sds-danger-button" disabled={pending || !operation.canExecute || operation.kind!=="remove"} onClick={()=>onConfirm(operation.id,false)}>{pending ? "Removing…" : missing ? "Remove missing record" : "Remove from target"}</button></div>
  </div></div>;
}
