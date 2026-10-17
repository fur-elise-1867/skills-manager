import type {SessionView} from "../contracts";

export default function OfflineBanner({session,onRetry}:{session:SessionView;onRetry?:()=>void}) {
  if(session.status==="authenticated")return null;
  const offline=session.status==="offline";
  return <aside className="sds-lifecycle-notice sds-offline-banner" aria-label="Store connection status"><div><strong>{offline?"Skills Store is unavailable":session.status==="expired"?"Your session has expired":"Signed out of Skills Store"}</strong><p>{offline?"You can manage downloaded skills and local snapshots. New downloads and update checks need a Store connection.":"Sign in to deploy downloaded releases or access Skills Store. Existing files, snapshots and local maintenance remain available."}</p>{offline && <p>Current Store permissions are unknown. Use cached copy requires a verified release and a prior grant for this profile and account. Agent and MCP services may still need a network connection.</p>}</div>{offline && onRetry && <button className="sds-secondary-button" onClick={onRetry}>Retry connection</button>}</aside>;
}
