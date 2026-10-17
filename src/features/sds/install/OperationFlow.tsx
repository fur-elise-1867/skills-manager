import {useEffect,useRef,useState} from "react";
import {listen} from "@tauri-apps/api/event";
import type {OperationRequest,OperationResult,PreparedOperation} from "../contracts";
import {prepareOperation,executeOperation,cancelOperation,getOperationStatus} from "../api";
import {storeErrorMessage} from "../errors";
import InstallReview from "./InstallReview";
import RemoveReview from "../library/RemoveReview";

const terminal=(result:OperationResult)=>result.stage!=="removal-cleanup" && ["completed","failed","cancelled","recoveryRequired"].includes(result.state);
const stageLabels:Record<string,string>={prepared:"Waiting for review",downloading:"Downloading package",validating:"Checking package and permissions",staging:"Preparing destination",committing:"Committing changes",completed:"Completed",failed:"Could not finish",cancelled:"Cancelled",recoveryRequired:"Recovery needs attention"};

export default function OperationFlow({request,onClose,onCompleted}:{request:OperationRequest;onClose:()=>void;onCompleted:()=>void}){
  const key=JSON.stringify(request);
  const [revision,setRevision]=useState(0),[prepared,setPrepared]=useState<{key:string;revision:number;plan?:PreparedOperation;error?:string}>({key:"",revision:-1});
  const [running,setRunning]=useState<string|null>(null),[result,setResult]=useState<OperationResult|null>(null),[error,setError]=useState("");
  const [submitting,setSubmitting]=useState(false),[cancelling,setCancelling]=useState(false);
  const currentRef=useRef(key);currentRef.current=key;
  const completeRef=useRef(onCompleted);completeRef.current=onCompleted;
  const executed=useRef<string|null>(null),notified=useRef<string|null>(null);
  const dialog=useRef<HTMLDivElement>(null);
  useEffect(()=>{
    let active=true;
    const parsed=JSON.parse(key) as OperationRequest;
    prepareOperation(parsed).then(plan=>{if(active)setPrepared({key,revision,plan});}).catch(failure=>{if(active)setPrepared({key,revision,error:storeErrorMessage(failure)});});
    return()=>{active=false;};
  },[key,revision]);
  const plan=prepared.key===key && prepared.revision===revision ? prepared.plan : undefined;
  const prepError=prepared.key===key && prepared.revision===revision ? prepared.error : undefined;
  useEffect(()=>{
    if(!running)return;
    let active=true,busy=false;let unsubscribe:(()=>void)|undefined;
    function accept(next:OperationResult){
      if(!active || next.operationId!==running)return;
      setResult(next);
      if(next.state==="completed" && terminal(next) && notified.current!==next.operationId){notified.current=next.operationId;completeRef.current();}
    }
    // Events improve responsiveness; status remains authoritative if an event is missed.
    listen<OperationResult>("sds:operation",event=>{if(event.payload.operationId===running)void poll();}).then(stop=>{if(active)unsubscribe=stop;else stop();}).catch(()=>{});
    async function poll(){
      if(busy || !active)return;busy=true;
      try{const status=await getOperationStatus(running!);accept(status);if(active)setError("");}catch(failure){if(active)setError(storeErrorMessage(failure));}finally{busy=false;}
    }
    void poll();const timer=setInterval(()=>{void poll();},500);
    return()=>{active=false;clearInterval(timer);unsubscribe?.();};
  },[running]);
  useEffect(()=>{if(running)dialog.current?.focus();},[running]);
  async function confirm(id:string,ack:boolean){
    if(executed.current===id || !plan || plan.id!==id)return;
    executed.current=id;setSubmitting(true);setError("");
    try{
      const started=await executeOperation(id,ack);
      if(currentRef.current!==key)return;
      setRunning(started.operationId);setResult(null);
    }catch(failure){executed.current=null;setError(storeErrorMessage(failure));}finally{setSubmitting(false);}
  }
  async function cancel(){
    if(cancelling || !running)return;setCancelling(true);
    try{const status=await cancelOperation(running);setResult(status);}catch(failure){setError(storeErrorMessage(failure));}finally{setCancelling(false);}
  }
  function closeReview(){if(submitting)return;if(plan)void cancelOperation(plan.id).catch(()=>{});onClose();}
  if(plan && !running)return <>{error && <p className="sds-flow-error" role="alert">{error}</p>}{plan.kind==="remove" ? <RemoveReview operation={plan} pending={submitting} onConfirm={(id,ack)=>{void confirm(id,ack);}} onCancel={closeReview}/> : <InstallReview operation={plan} pending={submitting} onConfirm={(id,ack)=>{void confirm(id,ack);}} onCancel={closeReview} onCheckAgain={()=>{void cancelOperation(plan.id).catch(()=>{});setRevision(value=>value+1);}}/>}</>;
  const finished=result && terminal(result);
  const success=result?.state==="completed" && finished;
  return <div className="sds-modal-backdrop"><div className="sds-install-review sds-operation" ref={dialog} tabIndex={-1} role="dialog" aria-modal="true" aria-labelledby="sds-operation-title" onKeyDown={event=>{
    if(event.key==="Escape" && !running){event.preventDefault();onClose();}
    if(event.key!=="Tab")return;const controls=Array.from(dialog.current?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)")??[]),first=controls[0],last=controls.at(-1);
    if(event.shiftKey && document.activeElement===first){event.preventDefault();last?.focus();}else if(!event.shiftKey && document.activeElement===last){event.preventDefault();first?.focus();}
  }}>
    <h2 id="sds-operation-title">{!running ? "Preparing review" : success ? request.kind==="remove" ? "Removal completed" : "Installation completed" : finished ? "Operation result" : request.kind==="remove" ? "Removing skill" : "Installing skill"}</h2>
    {plan && <><p className="sds-review-subtitle">{plan.release.installName} · {plan.release.version}</p><p className="sds-target-path">{plan.target.path}</p></>}
    {!running && !prepError && <p role="status">Checking the release and destination…</p>}
    {running && !finished && <><div className="sds-operation-bar" aria-hidden="true"/><p role="status">{result?.stage==="removal-cleanup" ? "Finishing removal cleanup" : result ? stageLabels[result.state] : "Starting operation…"}</p></>}
    {success && <p className="sds-operation-success">{request.kind==="remove" ? "Removed from target" : "Installed successfully"}</p>}
    {result?.state==="cancelled" && <p>Cancelled. The operation stopped before commit.</p>}
    {(prepError || error || result?.error) && <p role="alert" className="sds-error">{prepError || error || storeErrorMessage(result?.error)}</p>}
    {result?.operationId && <p className="sds-operation-id">Operation {result.operationId}</p>}
    {result?.error?.code==="CLEANUP_REVIEW_REQUIRED" && <p>Review the retained staging files beside your skills folder before cleaning them up.</p>}
    <div className="sds-review-actions">{!running ? <><button className="sds-secondary-button" onClick={onClose}>Cancel</button>{prepError && <button className="sds-primary-button" onClick={()=>setRevision(value=>value+1)}>Review again</button>}</> : finished ? <button className="sds-primary-button" onClick={onClose}>Done</button> : <button className="sds-secondary-button" disabled={cancelling || result?.state==="committing"} onClick={()=>{void cancel();}}>{cancelling ? "Cancellation requested…" : "Cancel operation"}</button>}</div>
  </div></div>;
}
