import {useEffect,useRef,useState} from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import {getLibrary,getLibraryPreview,revealDeployment,captureSnapshot} from "../api";
import {writeText} from "@tauri-apps/plugin-clipboard-manager";
import type {CacheStatus,DeploymentStatus,LibraryItem,Release,LibraryDeployment,SessionView,UpdateCheck,OperationRequest} from "../contracts";

import RemoveOptions from "./RemoveOptions";
import DeleteCacheReview from "./DeleteCacheReview";
import UpdateReview from "../updates/UpdateReview";
import OfflineBanner from "../offline/OfflineBanner";

const cacheLabels:Record<CacheStatus,string>={verified:"Downloaded",corrupt:"Cache needs attention",missing:"Cache missing",unverifiable:"Cache unavailable"};
const deploymentLabels:Record<DeploymentStatus,string>={installed:"Installed",modified:"Local changes",missing:"Missing folder",unverifiable:"Needs review"};
const mcpLabels={notRequired:"Not required",configured:"Configured",missing:"Missing",unknown:"Not declared",unverifiable:"Unverifiable"};
function localError(error:unknown,preview=false){
  const code=typeof error==="object" && error!==null && "code" in error ? error.code : "";
  if(code==="CACHE_CORRUPT")return "The cached files have changed. Keep existing installations and download a verified copy before previewing.";
  if(code==="PREVIEW_LIMIT")return "The skill document is too large to preview (maximum 1 MiB).";
  return preview ? "The skill document could not be read safely. Keep the cached files and check the release." : "Library could not be read. Check disk space and permissions, then try again.";
}

export default function LibraryPage({onInstall,onRemove,onCachedInstall,session,profileId,onRetryConnection,updates,onOperation,onCheckUpdates,updatesBusy,updatesError,targetWorkspaceId}:{targetWorkspaceId?:string;updates?:UpdateCheck;onOperation?:(request:OperationRequest)=>void;onCheckUpdates?:()=>void;updatesBusy?:boolean;updatesError?:string;onCachedInstall?:(releaseKey:string)=>void;session?:SessionView;profileId?:string;onRetryConnection?:()=>void;onInstall?:(release:Release)=>void;onRemove?:(deploymentId:string,options?:{backup:boolean;acknowledgeLocalEdits:boolean})=>void}){
  const [revision,setRevision]=useState(0);
  const [deleting,setDeleting]=useState<LibraryItem|null>(null),[updating,setUpdating]=useState<LibraryItem|null>(null);
  const [removing,setRemoving]=useState<LibraryDeployment|null>(null),[snapshotBusy,setSnapshotBusy]=useState(false);
  const [result,setResult]=useState<{revision:number;items?:LibraryItem[];error?:string}>({revision:-1});
  const [query,setQuery]=useState("");
  const [actionMessage,setActionMessage]=useState("");
  const [filter,setFilter]=useState<"all"|"installed"|"available">("all");
  const [locationsId,setLocationsId]=useState<string|null>(null);
  const [selected,setSelected]=useState<LibraryItem|null>(null);
  const [preview,setPreview]=useState<{id:string;text?:string;error?:string}>({id:""});
  const previewTrigger=useRef<HTMLButtonElement|null>(null);
  useEffect(()=>{
    let active=true;
    getLibrary().then(items=>{if(active)setResult({revision,items});}).catch(error=>{if(active)setResult({revision,error:localError(error)});});
    return()=>{active=false;};
  },[revision]);
  const selectedId=selected?.id;
  useEffect(()=>{
    if(!selectedId)return;
    let active=true;
    getLibraryPreview(selectedId).then(text=>{if(active)setPreview({id:selectedId,text});}).catch(error=>{if(active)setPreview({id:selectedId,error:localError(error,true)});});
    return()=>{active=false;};
  },[selectedId]);
  const current=result.revision===revision ? result : null;
  const keyword=query.trim().toLocaleLowerCase();
  const scoped=current?.items?.map(item=>targetWorkspaceId?{...item,deployments:item.deployments.filter(deployment=>deployment.target.workspaceId===targetWorkspaceId)}:item).filter(item=>!targetWorkspaceId || item.deployments.length>0);
  const items=scoped?.filter(item=>(filter==="all" || (filter==="installed" ? item.deployments.length>0 : item.deployments.length===0)) && [item.release.installName,item.release.version,item.release.skillId,...item.deployments.map(deployment=>deployment.target.path)].some(value=>value.toLocaleLowerCase().includes(keyword)));
  const locations=items?.find(item=>item.id===locationsId)??items?.[0];
  function refresh(){setSelected(null);setRevision(value=>value+1);}
  async function copyPath(path:string){try{await writeText(path);setActionMessage("Destination path copied.");}catch{setActionMessage("The path could not be copied. Select the displayed path to copy it manually.");}}
  async function reveal(id:string){try{await revealDeployment(id);setActionMessage("");}catch{setActionMessage("The recorded destination could not be revealed safely. Refresh Library and check its location.");}}
  async function snapshot(deployment:LibraryDeployment){if(!deployment.expectedFingerprint || snapshotBusy)return;setSnapshotBusy(true);try{await captureSnapshot(deployment.id,deployment.expectedFingerprint);setActionMessage("Local snapshot saved.");}catch{setActionMessage("Snapshot could not be saved. Your current files have been kept. Refresh Library and try again.");}finally{setSnapshotBusy(false);}}
  return <section className="sds-library" aria-label="Downloaded releases">
    {session && <OfflineBanner session={session} onRetry={onRetryConnection}/>}
    {onCheckUpdates && <div className="sds-update-toolbar"><button className="sds-secondary-button" disabled={updatesBusy || session?.status!=="authenticated"} onClick={onCheckUpdates}>{updatesBusy?"Checking updates…":"Check for updates"}</button>{updates && <span>Metadata checked {updates.checkedAt}</span>}</div>}
    {updatesError && <p role="alert" className="sds-error">{updatesError}</p>}
    {actionMessage && <p role="status">{actionMessage}</p>}
    <div className="sds-library-toolbar"><label className="sds-library-search"><span className="sds-visually-hidden">Search Library</span><input type="search" value={query} onChange={event=>setQuery(event.target.value)} placeholder="Search skills in your Library…"/></label><div className="sds-library-filters" aria-label="Library status"><button aria-pressed={filter==="all"} onClick={()=>setFilter("all")}>All</button><button aria-pressed={filter==="installed"} onClick={()=>setFilter("installed")}>Installed</button><button aria-pressed={filter==="available"} onClick={()=>setFilter("available")}>Available</button><button className="sds-secondary-button" onClick={refresh} disabled={!current}>Refresh Library</button></div></div>
    {!current ? <p role="status">Checking local releases and destinations…</p>
      : current.error ? <div className="sds-error" role="alert"><p>{current.error}</p><button className="sds-secondary-button" onClick={refresh}>Try again</button></div>
      : !scoped?.length ? <p className="sds-empty">{targetWorkspaceId?"No installed skills in this workspace.":"No downloaded releases yet."}</p>
      : !items?.length ? <p className="sds-empty">No matching releases.</p>
      : <div className="sds-skill-grid">{items.map(item=><article className="sds-skill-card sds-library-card" data-selected={locations?.id===item.id} key={item.id}>
        <div className="sds-card-heading"><h3>{item.release.installName}</h3><span>{item.release.version}</span></div>
        {onOperation && item.profileId===profileId && updates?.items.some(update=>update.skillId===item.release.skillId && update.currentReleaseId===item.release.id && update.status==="update_available" && update.latestRelease && update.latestRelease.sequence>item.release.sequence) && <button className="sds-text-button" onClick={()=>setUpdating(item)}>Review update</button>}
        {updates && item.profileId!==profileId && <p className="sds-field-help">Not checked · another Store profile</p>}
        <span className="sds-badge" data-local-status={item.cacheStatus}>{cacheLabels[item.cacheStatus]}</span>
        <p className="sds-library-release">Release {item.release.id} · {item.release.package.format.toUpperCase()}</p>
        <p className="sds-library-release">{item.deployments.length ? `${item.deployments.length} recorded ${item.deployments.length===1?"destination":"destinations"}` : "No installed destinations"}</p>
        <div className="sds-library-actions"><button className="sds-secondary-button" aria-label={`Preview ${item.release.installName} ${item.release.version}`} disabled={item.cacheStatus!=="verified"} onClick={event=>{previewTrigger.current=event.currentTarget;setSelected(item);}}>Preview</button>
          {onInstall && (!session || session.status==="authenticated") && <button className="sds-primary-button" disabled={item.cacheStatus!=="verified"} onClick={()=>onInstall(item.release)}>Install skill</button>}
          {onCachedInstall && session?.status!=="authenticated" && <button className="sds-primary-button" disabled={item.cacheStatus!=="verified" || session?.status!=="offline" || !session.user || item.profileId!==profileId} onClick={()=>onCachedInstall(item.id)}>Use cached copy</button>}
          <button className="sds-text-button" disabled={item.deployments.length>0} title={item.deployments.length?"Installed destinations still reference this release":undefined} onClick={()=>setDeleting(item)}>Delete from Library</button>
        </div>
        <p className="sds-library-release">Cached {item.cachedAt} · Source profile {item.profileId}</p>
        <button className="sds-text-button" onClick={()=>setLocationsId(item.id)} aria-pressed={locations?.id===item.id}>View destinations →</button>
      </article>)}</div>}
    {locations && <section className="sds-locations" aria-label="Installed locations"><div className="sds-locations-heading"><h3>{locations.release.installName} · Installed locations</h3><span>{locations.deployments.length} recorded {locations.deployments.length===1?"target":"targets"}</span></div><div className="sds-table-scroll"><table><thead><tr><th>Agent</th><th>Scope / Path</th><th>Version</th><th>Local status</th><th>Action</th></tr></thead><tbody>
      {locations.deployments.map(deployment=><tr key={deployment.id}><td><span className="sds-agent-dot"/>AIPro</td><td><strong>{deployment.target.workspaceId==="global"?"Global":"Project"}</strong><p className="sds-target-path">{deployment.target.path}</p><div className="sds-library-path-actions"><button className="sds-text-button" onClick={()=>{void copyPath(deployment.target.path);}}>Copy path</button><button className="sds-text-button" disabled={deployment.status==="missing" || deployment.status==="unverifiable"} onClick={()=>{void reveal(deployment.id);}}>Reveal folder</button></div><p>MCP at installation: {mcpLabels[deployment.mcp.state]}{deployment.mcp.missing.length ? ` · ${deployment.mcp.missing.join(", ")}` : ""}</p></td><td>{locations.release.version}</td><td><span className="sds-badge" data-local-status={deployment.status}>{deploymentLabels[deployment.status]}</span>{deployment.reason && <p className="sds-library-reason">{deployment.reason}</p>}</td><td>{onRemove && <button className="sds-text-button" disabled={deployment.status==="unverifiable"} onClick={()=>{if(deployment.status==="modified")setRemoving(deployment);else onRemove(deployment.id);}}>Remove from target</button>}<button className="sds-text-button" disabled={snapshotBusy || !deployment.expectedFingerprint || !["installed","modified"].includes(deployment.status)} onClick={()=>{void snapshot(deployment);}}>Create snapshot</button></td></tr>)}
      {!locations.deployments.length && <tr><td colSpan={5}>No installed destinations. Your downloaded copy remains in Library.</td></tr>}
    </tbody></table></div><p className="sds-locations-note">Each target keeps its own content. Changing Library files does not update other locations.</p></section>}
    {updating && onOperation && (()=>{const latest=updates?.items.find(update=>update.skillId===updating.release.skillId && update.currentReleaseId===updating.release.id)?.latestRelease;return latest?<UpdateReview item={updating} items={current?.items?.filter(item=>item.profileId===updating.profileId && item.release.skillId===updating.release.skillId && updates?.items.some(check=>check.skillId===item.release.skillId && check.currentReleaseId===item.release.id && check.status==="update_available" && check.latestRelease?.id===latest.id))} latest={latest} onClose={()=>setUpdating(null)} onCompleted={()=>setRevision(value=>value+1)}/>:null;})()}
    {deleting && <DeleteCacheReview item={deleting} onClose={()=>setDeleting(null)} onCompleted={()=>{setRevision(value=>value+1);}}/>}
    {removing && <RemoveOptions deployment={removing} onCancel={()=>setRemoving(null)} onReview={options=>{onRemove?.(removing.id,options);setRemoving(null);}}/>}
    {selected && <section className="sds-library-preview" aria-label="Local skill preview"><div className="sds-library-toolbar"><h2>Skill document · {selected.release.installName}</h2><button className="sds-secondary-button" onClick={()=>{setSelected(null);previewTrigger.current?.focus();}}>Close preview</button></div>
      {preview.id!==selected.id ? <p role="status">Reading verified skill document…</p> : preview.error ? <p role="alert" className="sds-error">{preview.error}</p> : <article className="sds-markdown"><ReactMarkdown skipHtml remarkPlugins={[remarkGfm]} components={{img:()=>null,a:({children})=><span>{children}</span>}}>{preview.text}</ReactMarkdown></article>}
    </section>}
  </section>;
}
