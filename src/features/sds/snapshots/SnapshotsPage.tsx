import {useEffect,useState} from "react";
import {listSnapshots,verifySnapshot} from "../api";
import type {OperationRequest,SnapshotRecord} from "../contracts";
import {storeErrorMessage} from "../errors";

const reasons={manual:"Manual snapshot",beforeUpdate:"Before update",beforeRestore:"Before restore",beforeRemove:"Before removal"};
export default function SnapshotsPage({onRestore}:{onRestore:(request:OperationRequest)=>void}) {
  const [result,setResult]=useState<{revision:number;items?:SnapshotRecord[];error?:string}>({revision:-1});
  const [revision,setRevision]=useState(0),[query,setQuery]=useState("");
  const [integrity,setIntegrity]=useState<Record<string,{state:"checking"|"verified"|"corrupt"|"error";checkedAt?:string}>>({});
  async function verify(id:string){
    if(integrity[id]?.state==="checking")return;
    setIntegrity(value=>({...value,[id]:{state:"checking"}}));
    try{await verifySnapshot(id);setIntegrity(value=>({...value,[id]:{state:"verified",checkedAt:new Date().toLocaleString("en-GB")}}));}
    catch(failure){const corrupt=typeof failure==="object" && failure!==null && "code" in failure && failure.code==="SNAPSHOT_CORRUPT";setIntegrity(value=>({...value,[id]:{state:corrupt?"corrupt":"error"}}));}
  }
  useEffect(()=>{let active=true;listSnapshots().then(items=>{if(active)setResult({revision,items});}).catch(failure=>{if(active)setResult({revision,error:storeErrorMessage(failure)});});return()=>{active=false;};},[revision]);
  const current=result.revision===revision?result:null;
  const items=current?.items?.filter(item=>[item.release.installName,item.release.version,item.target.path].join(" ").toLowerCase().includes(query.toLowerCase().trim()));
  return <section className="sds-snapshots" aria-label="Local snapshots">
    <div className="sds-lifecycle-notice"><strong>Local only</strong><p>Snapshots preserve one selected skill folder, including its local edits. They are independent of Library cache and are not a backup of your entire computer or AIPro configuration.</p></div>
    <div className="sds-library-toolbar"><label className="sds-library-search"><span className="sds-visually-hidden">Search snapshots</span><input type="search" value={query} onChange={event=>setQuery(event.target.value)} placeholder="Search skill or destination…"/></label><button className="sds-secondary-button" disabled={!current} onClick={()=>setRevision(value=>value+1)}>Refresh snapshots</button></div>
    {!current?<p role="status">Reading local snapshots…</p>:current.error?<p className="sds-error" role="alert">{current.error}</p>:!current.items?.length?<p className="sds-empty">No local snapshots yet. Create a snapshot from a deployment in Library.</p>:!items?.length?<p>No matching snapshots.</p>:<div className="sds-table-scroll"><table className="sds-lifecycle-table"><thead><tr><th>Skill / target</th><th>Saved</th><th>Reason</th><th>Size</th><th>Action</th></tr></thead><tbody>{items.map(snapshot=><tr key={snapshot.id}>
      <td><strong>{snapshot.release.installName}</strong><p>{snapshot.release.version} · AIPro · {snapshot.target.workspaceId==="global"?"Global":"Project"}{snapshot.contentFingerprint!==snapshot.baselineFingerprint?" · Saved local edits":""}</p><p className="sds-target-path">{snapshot.target.path}</p></td>
      <td><time dateTime={snapshot.capturedAt}>{new Date(snapshot.capturedAt).toLocaleString("en-GB")}</time></td><td>{reasons[snapshot.reason]}</td><td>{(snapshot.contentManifest.entries.reduce((sum,entry)=>sum+entry.sizeBytes,0)/1024).toFixed(1)} KiB</td>
      <td><p role="status">{integrity[snapshot.id]?.state==="checking"?"Checking bytes…":integrity[snapshot.id]?.state==="verified"?`Verified ${integrity[snapshot.id].checkedAt}`:integrity[snapshot.id]?.state==="corrupt"?"Corrupt or missing":integrity[snapshot.id]?.state==="error"?"Could not verify · try again":"Not checked"}</p><button className="sds-text-button" disabled={integrity[snapshot.id]?.state==="checking"} onClick={()=>{void verify(snapshot.id);}}>Verify snapshot</button><button className="sds-text-button" disabled={["checking","corrupt","error"].includes(integrity[snapshot.id]?.state)} onClick={()=>onRestore({kind:"restore",snapshotId:snapshot.id,deploymentId:snapshot.deploymentId,protectCurrent:true})}>Restore…</button></td>
    </tr>)}</tbody></table></div>}
    <p className="sds-field-help">Snapshots are verified before restore. Unreadable or changed snapshots cannot replace your current target. No automatic retention policy is applied.</p>
  </section>;
}
