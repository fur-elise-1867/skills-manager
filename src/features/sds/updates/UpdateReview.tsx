import {useEffect,useRef,useState} from "react";
import Dialog from "../Dialog";
import {cancelUpdateSelection,executeUpdates,getUpdateSelectionStatus,prepareUpdates} from "../api";
import {storeErrorMessage} from "../errors";
import type {LibraryItem,PreparedUpdateSelection,ReleaseRef,UpdateRequest,UpdateSelectionResult} from "../contracts";

type Outcome=UpdateSelectionResult["items"][number];
const terminal=(item:Outcome)=>!!item.preflightError || !!item.result && ["completed","failed","cancelled","recoveryRequired"].includes(item.result.state);
const labels={prepared:"Waiting",downloading:"Downloading",validating:"Validating",staging:"Staging",committing:"Committing",completed:"Updated",failed:"Failed",cancelled:"Cancelled",recoveryRequired:"Recovery required"};

export default function UpdateReview({item,items=[item],latest,onClose,onCompleted}:{item:LibraryItem;items?:LibraryItem[];latest:ReleaseRef;onClose:()=>void;onCompleted:()=>void}) {
  const candidates=items.flatMap(release=>release.deployments.map(deployment=>({deployment,release:release.release})));
  const [selected,setSelected]=useState<string[]>([]),[phase,setPhase]=useState<"select"|"preparing"|"review"|"running"|"result">("select");
  const [selection,setSelection]=useState<PreparedUpdateSelection>(),[requests,setRequests]=useState<UpdateRequest[]>([]);
  const [acknowledged,setAcknowledged]=useState<string[]>([]),[outcomes,setOutcomes]=useState<Outcome[]>([]),[retained,setRetained]=useState<Outcome[]>([]);
  const [error,setError]=useState(""),[cancelling,setCancelling]=useState(false),[submitting,setSubmitting]=useState(false);
  const inFlight=useRef(false),active=useRef(true),notified=useRef(new Set<string>());
  const complete=useRef(onCompleted);complete.current=onCompleted;
  useEffect(()=>{active.current=true;return()=>{active.current=false;};},[]);
  const runningId=phase==="running"?selection?.selectionId:undefined;
  useEffect(()=>{
    if(!runningId || !selection)return;
    let mounted=true,busy=false;
    async function poll(){
      if(busy)return;busy=true;
      try{
        const result=await getUpdateSelectionStatus(runningId!);
        if(!mounted || result.selectionId!==runningId)return;
        setOutcomes(result.items);setError("");
        if(selection!.items.every(prepared=>{const entry=result.items.find(entry=>entry.deploymentId===prepared.deploymentId);return entry && terminal(entry);})){setPhase("result");if(!notified.current.has(runningId!)){notified.current.add(runningId!);complete.current();}}
      }catch(failure){if(mounted)setError(storeErrorMessage(failure));}finally{busy=false;}
    }
    void poll();const timer=setInterval(()=>{void poll();},500);
    return()=>{mounted=false;clearInterval(timer);};
  },[runningId,selection]);
  async function prepare(nextRequests:UpdateRequest[]){
    if(inFlight.current || !nextRequests.length)return;
    inFlight.current=true;setPhase("preparing");setError("");setAcknowledged([]);setRequests(nextRequests);
    try{
      const next=await prepareUpdates(nextRequests);
      if(!active.current)return;
      const expected=new Set(nextRequests.map(request=>request.deploymentId));
      if(!next.selectionId || next.items.length!==expected.size || new Set(next.items.map(entry=>entry.deploymentId)).size!==expected.size || next.items.some(entry=>!expected.has(entry.deploymentId)))throw {code:"INVALID_RESPONSE"};
      setSelection(next);setOutcomes([]);setPhase("review");
    }catch(failure){if(active.current){setError(storeErrorMessage(failure));setPhase("select");}}finally{inFlight.current=false;}
  }
  function close(){if(phase==="preparing" || phase==="running" || submitting)return;if(selection)void cancelUpdateSelection(selection.selectionId).catch(()=>{});onClose();}
  const preparedItems=selection?.items.filter(entry=>entry.operationId && entry.operation)??[];
  const ready=!!selection && preparedItems.length>0 && selection.items.every(entry=>!!entry.error || !!entry.operation && entry.operation.id===entry.operationId && entry.operation.kind==="update" && entry.operation.canExecute && (!entry.operation.requiresMcpAck || acknowledged.includes(entry.operation.id)));
  async function execute(){
    if(!selection || !ready || inFlight.current)return;
    inFlight.current=true;setSubmitting(true);setError("");
    try{await executeUpdates(selection.selectionId,preparedItems.filter(entry=>entry.operation!.requiresMcpAck).map(entry=>entry.operationId!));if(active.current)setPhase("running");}
    catch(failure){if(active.current){setError(storeErrorMessage(failure));setPhase("running");}}
    finally{inFlight.current=false;if(active.current)setSubmitting(false);}
  }
  async function cancel(){if(!selection || cancelling)return;setCancelling(true);try{const result=await cancelUpdateSelection(selection.selectionId);if(active.current){setOutcomes(result.items);if(selection.items.every(entry=>{const status=result.items.find(status=>status.deploymentId===entry.deploymentId);return status && terminal(status);})){setPhase("result");complete.current();}}}catch(failure){if(active.current)setError(storeErrorMessage(failure));}finally{if(active.current)setCancelling(false);}}
  const allOutcomes=[...retained,...outcomes];
  const failed=outcomes.filter(entry=>entry.preflightError || entry.result?.state==="failed").map(entry=>entry.deploymentId);
  const preflightFailures=selection?.items.filter(entry=>entry.error).map(entry=>entry.deploymentId)??[];
  function retry(ids:string[]){
    const retryIds=new Set(ids);
    setSelected(ids);
    setRetained([...retained,...outcomes.filter(entry=>!retryIds.has(entry.deploymentId))]);
    void prepare(requests.filter(request=>retryIds.has(request.deploymentId)));
  }
  const busy=phase==="preparing" || phase==="running" || submitting;
  return <Dialog title={`Update ${item.release.installName}`} onClose={close} busy={busy} wide>
    <p className="sds-review-subtitle">{item.release.version} → {latest.version}</p>
    {error && <p role="alert" className="sds-error">{error}</p>}
    {phase==="select" && <>
      <p>Choose the locations to update. Each target keeps its own version. Only selected locations will change.</p>
      <div className="sds-table-scroll"><table className="sds-lifecycle-table"><thead><tr><th>Update</th><th>Target</th><th>Current</th><th>Local state</th></tr></thead><tbody>{candidates.map(({deployment,release})=><tr key={deployment.id}>
        <td><input type="checkbox" aria-label={`Update ${deployment.id}`} disabled={retained.some(entry=>entry.deploymentId===deployment.id && entry.result?.state==="completed") || !["installed","modified"].includes(deployment.status)} checked={selected.includes(deployment.id)} onChange={event=>setSelected(values=>event.target.checked?[...values,deployment.id]:values.filter(id=>id!==deployment.id))}/></td><td><strong>AIPro · {deployment.target.workspaceId==="global"?"Global":"Project"}</strong><p className="sds-target-path">{deployment.target.path}</p></td><td>{release.version}</td><td>{retained.some(entry=>entry.deploymentId===deployment.id && entry.result?.state==="completed")?"Updated":deployment.status==="modified"?"Local changes":deployment.status==="installed"?"Unchanged":"Needs review"}</td>
      </tr>)}</tbody></table></div>
      {candidates.some(({deployment})=>selected.includes(deployment.id) && deployment.status==="modified") && <div className="sds-lifecycle-notice"><strong>Protect your changes</strong><p>A verified snapshot of each selected modified copy is saved before replacement. If backup fails, this copy will not be updated.</p></div>}
      <div className="sds-review-actions"><button className="sds-secondary-button" onClick={close}>Keep current</button><button className="sds-primary-button" disabled={!selected.length} onClick={()=>{void prepare(selected.map(deploymentId=>({deploymentId,releaseId:latest.id,localEdits:"backupAndReplace"})));}}>Review selected targets</button></div>
    </>}
    {phase==="preparing" && <p role="status">Preparing frozen release and MCP reviews for selected targets…</p>}
    {phase==="review" && <>
      <p>No packages have been installed. Review each target and the new release’s requirements before starting.</p>
      {selection?.items.map(entry=><section className="sds-update-item" key={entry.deploymentId} aria-label={`Review ${entry.deploymentId}`}>
        {entry.error?<><h3>{entry.deploymentId} · Cannot prepare</h3><p className="sds-error" role="alert">{storeErrorMessage(entry.error)}</p></>:entry.operation?<>
          <h3>AIPro · {entry.operation.target.workspaceId==="global"?"Global":"Project"} · {entry.operation.release.version}</h3><p className="sds-target-path">{entry.operation.target.path}</p>
          <h4>Release changes</h4><p className="sds-changelog">{entry.operation.release.changelog || "No release notes provided."}</p>
          <h4>MCP check · {entry.operation.mcp.state}</h4>{entry.operation.mcp.missing.length>0 && <ul>{entry.operation.mcp.missing.map(name=><li key={name}>{name}</li>)}</ul>}
          {entry.operation.mcp.reason && <p>{entry.operation.mcp.reason}</p>}
          <p>Configuration detection does not verify that MCP servers are running.</p>
          {entry.operation.conflict && <p role="alert" className="sds-error">{entry.operation.conflict}</p>}
          {entry.operation.requiresMcpAck && <label className="sds-check-row"><input type="checkbox" aria-label={`Acknowledge MCP warnings for ${entry.deploymentId}`} disabled={submitting} checked={acknowledged.includes(entry.operation.id)} onChange={event=>setAcknowledged(values=>event.target.checked?[...values,entry.operation!.id]:values.filter(id=>id!==entry.operation!.id))}/>I acknowledge the new release’s MCP warnings for this operation</label>}
        </>:<p className="sds-error" role="alert">The frozen operation review is unavailable. Keep the current files and prepare this selection again.</p>}
      </section>)}
      <p>Modified copies require verified snapshots. A snapshot failure stops that target. Unselected destinations stay unchanged.</p>
      <div className="sds-review-actions"><button className="sds-secondary-button" disabled={submitting} onClick={close}>Keep current</button>{preflightFailures.length>0 && !preparedItems.length && <button className="sds-secondary-button" onClick={()=>retry(preflightFailures)}>Review failed targets again</button>}<button className="sds-primary-button" disabled={!ready || submitting} onClick={()=>{void execute();}}>Back up and update selected targets</button></div>
    </>}
    {(phase==="running" || phase==="result") && <>
      <p role="status">{phase==="running"?"Updating selected targets serially…":allOutcomes.length>0 && allOutcomes.every(entry=>entry.result?.state==="completed")?"All selected updates completed.":"Selected updates finished with individual results."}</p>
      <div className="sds-table-scroll"><table className="sds-lifecycle-table"><thead><tr><th>Target</th><th>Result</th><th>Details</th></tr></thead><tbody>{[...retained,...(selection?.items??[]).map(entry=>outcomes.find(outcome=>outcome.deploymentId===entry.deploymentId)??{deploymentId:entry.deploymentId,operationId:entry.operationId,preflightError:entry.error})].map(entry=><tr key={`${entry.deploymentId}:${entry.operationId??"preflight"}`}>
        <td>{entry.deploymentId}<p className="sds-target-path">{candidates.find(candidate=>candidate.deployment.id===entry.deploymentId)?.deployment.target.path}</p></td><td>{entry.preflightError?"Failed preflight":entry.result?labels[entry.result.state]:"Waiting"}</td><td>{entry.preflightError || entry.result?.error?<p className="sds-error">{storeErrorMessage(entry.preflightError??entry.result?.error)}</p>:entry.operationId}
          {entry.result?.state==="recoveryRequired" && <p>Restart SDS to reconcile this operation before retrying. Its files are retained.</p>}</td>
      </tr>)}</tbody></table></div>
      <div className="sds-review-actions">{phase==="running"?<button className="sds-secondary-button" disabled={cancelling} onClick={()=>{void cancel();}}>{cancelling?"Cancellation requested…":"Cancel remaining updates"}</button>:<><button className="sds-secondary-button" onClick={close}>Done</button>{failed.length>0 && <button className="sds-primary-button" onClick={()=>retry(failed)}>Review failed targets again</button>}</>}</div>
    </>}
  </Dialog>;
}
