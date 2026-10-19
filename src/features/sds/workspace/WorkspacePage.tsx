import {useEffect,useRef,useState} from "react";
import {open} from "@tauri-apps/plugin-dialog";
import {addWorkspace,listWorkspaces,removeWorkspace} from "../api";
import type {TargetSelection,WorkspaceRef,OperationRequest,SessionView,UpdateCheck} from "../contracts";
import LibraryPage from "../library/LibraryPage";
import {storeErrorMessage} from "../errors";

export default function WorkspacePage({onSelect,onCancel,onOperation,onLibrary,session,profileId,updates}:{onOperation?:(request:OperationRequest)=>void;onLibrary?:()=>void;session?:SessionView;profileId?:string;updates?:UpdateCheck;onSelect?:(selection:TargetSelection)=>void;onCancel?:()=>void}) {
  const [projects,setProjects]=useState<WorkspaceRef[]>(),[error,setError]=useState(""),[busy,setBusy]=useState(false);
  const [scope,setScope]=useState("global"),[selected,setSelected]=useState(""),[revision,setRevision]=useState(0);
  const listingVersion=useRef(0);

  useEffect(()=>{
    let active=true;
    const version=++listingVersion.current;
    listWorkspaces().then(items=>{
      if(active && listingVersion.current===version){
        setProjects(items);
        setSelected(prev=>{
          if(prev && items.some(item=>item.id===prev))return prev;
          return onSelect ? "" : (items[0]?.id ?? "");
        });
        setError("");
      }
    }).catch(failure=>{
      if(active && listingVersion.current===version)setError(storeErrorMessage(failure));
    });
    return()=>{active=false;};
  },[revision,onSelect]);

  async function register(){
    setBusy(true);setError("");
    try{
      const root=await open({directory:true,multiple:false,title:"Choose project folder"});
      if(typeof root!=="string")return;
      const project=await addWorkspace(root);
      listingVersion.current++;
      setProjects(items=>[...(items??[]).filter(item=>item.id!==project.id),project]);
      setSelected(project.id);
      setScope("project");
    }
    catch(failure){setError(storeErrorMessage(failure));}
    finally{setBusy(false);}
  }

  async function remove(id:string){
    setBusy(true);
    try{
      await removeWorkspace(id);
      listingVersion.current++;
      setProjects(items=>items?.filter(item=>item.id!==id));
      setSelected(prev=>{
        if(prev===id){
          const remaining=(projects??[]).filter(item=>item.id!==id);
          return remaining[0]?.id ?? "";
        }
        return prev;
      });
      setError("");
    }
    catch(failure){setError(storeErrorMessage(failure));}
    finally{setBusy(false);}
  }

  const validProjectSelected=!onSelect && Boolean(selected && selected!=="global" && projects?.some(project=>project.id===selected));

  return <section className="sds-workspace" aria-label="AIPro workspace">
    <p className="sds-eyebrow">AIPRO · CHOOSE LOCATION</p><p>Each project and the global location keep independent copies and release versions. MCP configuration is checked in your global AIPro configuration.</p>
    {onSelect && <fieldset disabled={busy} className="sds-scope-options"><legend>Where should this skill be available?</legend><label><input type="radio" aria-label="Global" name="sds-scope" checked={scope==="global"} onChange={()=>setScope("global")}/>Global<span>Available across your projects</span></label><label><input type="radio" aria-label="Project" name="sds-scope" checked={scope==="project"} onChange={()=>setScope("project")}/>Project<span>Available in one registered project</span></label></fieldset>}
    {(scope==="project" || !onSelect) && <><label className="sds-field-label" htmlFor="sds-project">Registered project</label><select id="sds-project" value={selected} disabled={busy || !projects} onChange={event=>setSelected(event.target.value)}>{onSelect?<option value="">Choose a project…</option>:(!projects?.length?<option value="">No registered projects</option>:null)}{projects?.map(project=><option key={project.id} value={project.id}>{project.root}</option>)}</select></>}
    {!projects && !error && <p role="status">Reading registered projects…</p>}
    {projects?.length===0 && <p>No registered projects yet. Choose an existing project folder to add one.</p>}
    {error && <p role="alert" className="sds-error">{error} <button className="sds-text-button" onClick={()=>setRevision(value=>value+1)}>Retry</button></p>}
    {(!onSelect || scope==="project") && <button className="sds-secondary-button" disabled={busy} onClick={()=>{void register();}}>Add project folder</button>}
    {!onSelect && projects?.map(project=><div className="sds-workspace-row" key={project.id}><p className="sds-target-path">{project.root}</p><button disabled={busy} className="sds-text-button" onClick={()=>{void remove(project.id);}}>Remove registration</button></div>)}
    {!onSelect && <p className="sds-field-help">Removing a registration keeps the project folder. Registrations still used by deployments or pending operations cannot be removed.</p>}
    {validProjectSelected && onOperation && <section className="sds-workspace-skills"><div className="sds-library-toolbar"><h2>Installed AIPro skills</h2>{onLibrary && <button className="sds-secondary-button" onClick={onLibrary}>Add from Library</button>}</div><LibraryPage key={selected} targetWorkspaceId={selected} session={session} profileId={profileId} updates={updates} onOperation={onOperation} onRemove={(deploymentId,options)=>onOperation({kind:"remove",deploymentId,...options})}/></section>}
    {onSelect && <div className="sds-review-actions"><button className="sds-secondary-button" disabled={busy} onClick={onCancel}>Cancel</button><button className="sds-primary-button" disabled={busy || (scope==="project" && !projects?.some(project=>project.id===selected))} onClick={()=>onSelect({agentKey:"aipro",workspaceId:scope==="global"?"global":selected})}>Review requirements</button></div>}
  </section>;
}
