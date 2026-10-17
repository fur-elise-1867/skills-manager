import { useEffect, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { getSkillDetail } from "../api";
import type { Release, SkillDetail } from "../contracts";
import { storeErrorMessage } from "../errors";
import { useSdsSession } from "../session/context";

export function Compatibility({ agents }: {agents:string[] | null | undefined}) {
  return <span className="sds-compatibility">{agents == null ? "Compatibility not declared" : agents.length ? `Declared compatibility: ${agents.map((agent) => ({aipro:"AIPro",claude_code:"Claude Code",gemini_cli:"Gemini CLI",codex:"Codex"})[agent] ?? agent).join(", ")}` : "No compatible agents declared"}</span>;
}
export default function SkillDetailPanel({skillId,onBack,onInstall}:{skillId:string;onBack:()=>void;onInstall?:(release:Release)=>void}) {
  const { generation, refreshStatus, session } = useSdsSession();
  const [revision,setRevision] = useState(0);
  const key = JSON.stringify([skillId,generation,revision]);
  const [result,setResult] = useState<{key:string;detail?:SkillDetail;error?:string}>({key:""});
  useEffect(() => {
    let active = true;
    getSkillDetail(skillId).then((detail) => { if (active) setResult({key,detail}); }).catch((failure) => {
      if (!active) return;
      setResult({key,error:storeErrorMessage(failure)});
      if (failure?.code === "SESSION_EXPIRED" || failure?.code === "STORE_UNAVAILABLE") void refreshStatus();
    });
    return () => { active = false; };
  },[key,skillId,refreshStatus]);
  useEffect(()=>{
    if(result.key === key && result.detail && session?.status === "offline") void refreshStatus();
  },[result,key,session?.status,refreshStatus]);
  const current = result.key === key ? result : undefined;
  const detail = current?.detail;
  const release = detail?.latestRelease;
  return <section className="sds-detail">
    <button className="sds-text-button" aria-label="Back to results" onClick={onBack}>← Back to results</button>
    {!current || !current.detail && !current.error ? <p role="status">Loading skill details…</p>
      : current.error ? <div role="alert"><p>{current.error}</p><button onClick={()=>setRevision(revision+1)}>Try again</button></div>
      : detail && <>
        <div className="sds-detail-heading"><div><h2>{detail.summary.name}</h2><p>{detail.summary.creator.displayName}</p></div><span className="sds-badge">{release ? release.version : "No published release"}</span></div>
        <Compatibility agents={release ? release.compatibleAgents : detail.summary.compatibleAgents} />
        <div className="sds-detail-columns"><article className="sds-markdown"><ReactMarkdown skipHtml remarkPlugins={[remarkGfm]} components={{img:()=>null,a:({children})=><span>{children}</span>}}>{detail.summary.description}</ReactMarkdown></article>
          <aside className="sds-release-info" aria-label="Release information">
            <h3>Release information</h3>
            {release ? <>
              <dl><dt>Package</dt><dd>{release.package.format.toUpperCase()} · {(release.package.sizeBytes / 1024).toLocaleString(undefined,{maximumFractionDigits:1})} KB</dd><dt>Install folder</dt><dd>{release.installName}</dd></dl>
              <h3>MCP requirements</h3><p>{release.requiredMcps == null ? "Requirements not declared" : release.requiredMcps.length ? release.requiredMcps.join(", ") : "No MCP servers required"}</p>
              <h3>Release notes</h3><p>{release.changelog || "No release notes provided."}</p>
              <button className="sds-primary-button" disabled={!detail.summary.canDownload || !onInstall} onClick={()=>onInstall?.(release)}>Install skill</button>
              {!detail.summary.canDownload && <p>You do not have download permission for this skill.</p>}
            </> : <p>This skill does not have a published installable release.</p>}
          </aside>
        </div>
      </>}
  </section>;
}
