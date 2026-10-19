import {useEffect,useRef,useState} from "react";
import {executeCacheDelete,getOperationStatus,prepareCacheDelete} from "../api";
import type {CacheDeletePlan,LibraryItem,OperationResult} from "../contracts";
import {storeErrorMessage} from "../errors";
import Dialog from "../Dialog";

const terminal=(result:OperationResult)=>["completed","failed","cancelled","recoveryRequired"].includes(result.state);
export default function DeleteCacheReview({item,onClose,onCompleted}:{item:LibraryItem;onClose:()=>void;onCompleted:()=>void}) {
  const [plan,setPlan]=useState<CacheDeletePlan>(),[error,setError]=useState(""),[busy,setBusy]=useState(false),[result,setResult]=useState<OperationResult>();
  const executed=useRef(false),complete=useRef(onCompleted);complete.current=onCompleted;
  useEffect(()=>{let active=true;prepareCacheDelete(item.id).then(value=>{if(active)setPlan(value);}).catch(failure=>{if(active)setError(storeErrorMessage(failure));});return()=>{active=false;};},[item.id]);
  const running=result && !terminal(result)?result.operationId:null;
  useEffect(()=>{if(!running)return;let active=true,inFlight=false;async function poll(){if(inFlight)return;inFlight=true;try{const next=await getOperationStatus(running!);if(active){setResult(next);setError("");}}catch(failure){if(active)setError(storeErrorMessage(failure));}finally{inFlight=false;}}void poll();const timer=setInterval(()=>{void poll();},500);return()=>{active=false;clearInterval(timer);};},[running]);
  useEffect(()=>{if(result?.state==="completed")complete.current();},[result]);
  async function confirm(){if(!plan || executed.current)return;executed.current=true;setBusy(true);try{setResult(await executeCacheDelete(plan.id));}catch(failure){executed.current=false;setError(storeErrorMessage(failure));}finally{setBusy(false);}}
  const locked=busy || !!running;
  return <Dialog title="Delete downloaded copy" onClose={onClose} busy={locked}>
    <p>{item.release.installName} · {item.release.version}</p><p>This deletes only the downloaded Library cache. Installed destinations and self-contained snapshots are kept. Native reference checks block deletion only while a pending operation uses this release.</p>
    {!plan && !error && <p role="status">Checking release references…</p>}
    {error && <p className="sds-error" role="alert">{error}</p>}
    {running && <p role="status">Deleting cached files…</p>}
    {result && terminal(result) && <p role="status">{result.state==="completed"?"Downloaded copy deleted.":result.state==="recoveryRequired"?"Recovery needs attention. Restart SDS to reconcile the journal before retrying.":"Cache deletion did not complete. Review Library before trying again."}</p>}
    {result?.error && <p className="sds-error" role="alert">{storeErrorMessage(result.error)}</p>}
    <div className="sds-review-actions"><button className="sds-secondary-button" disabled={locked} onClick={onClose}>{result?"Done":"Cancel"}</button>{!result && <button className="sds-danger-button" disabled={!plan || locked || executed.current} onClick={()=>{void confirm();}}>Delete cached files</button>}</div>
  </Dialog>;
}
