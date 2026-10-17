import {useState} from "react";
import {revealLogs} from "../api";

export default function ActivitySettings(){
  const [error,setError]=useState("");
  async function open(){try{await revealLogs();setError("");}catch{setError("The local logs folder could not be shown. Check folder permissions.");}}
  return <section className="sds-welcome" aria-labelledby="sds-activity-title"><h2 id="sds-activity-title">Activity logs</h2><p>Local operation logs record the skill, release, destination, stages and result. Passwords and Store tokens are excluded.</p><button className="sds-secondary-button" onClick={()=>{void open();}}>Reveal logs folder</button>{error && <p role="alert" className="sds-error">{error}</p>}</section>;
}
