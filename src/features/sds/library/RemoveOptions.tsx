import {useState} from "react";
import Dialog from "../Dialog";
import type {LibraryDeployment} from "../contracts";

export default function RemoveOptions({deployment,onReview,onCancel}:{deployment:LibraryDeployment;onReview:(options:{backup:boolean;acknowledgeLocalEdits:boolean})=>void;onCancel:()=>void}) {
  const [backup,setBackup]=useState(true),[ack,setAck]=useState(false);
  return <Dialog title="Protect local changes" onClose={onCancel}>
    <p>This copy has local changes. Review the exact destination before removing it.</p><p className="sds-target-path">{deployment.target.path}</p>
    <label className="sds-check-row"><input type="checkbox" checked={backup} onChange={event=>setBackup(event.target.checked)}/>Save a local backup before removing</label>
    <p>{backup?"If the requested backup fails, removal stops and your current copy stays in place.":"The local changes will be discarded without creating a new snapshot."}</p>
    <label className="sds-check-row"><input type="checkbox" checked={ack} onChange={event=>setAck(event.target.checked)}/>I understand this removes the local changes from this target</label>
    <p>Your Library copy and installations in other projects are kept.</p>
    <div className="sds-review-actions"><button className="sds-secondary-button" onClick={onCancel}>Cancel</button><button className="sds-danger-button" disabled={!ack} onClick={()=>onReview({backup,acknowledgeLocalEdits:ack})}>Review removal</button></div>
  </Dialog>;
}
