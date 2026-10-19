import { useState } from "react";
import { Box, Library, Settings, Folder } from "lucide-react";
import type { ProfileInfo, OperationRequest, Release } from "./contracts";
import { useSdsSession } from "./session/context";
import SignIn from "./session/SignIn";
import ConnectionSettings from "./settings/ConnectionSettings";
import TargetSettings from "./settings/TargetSettings";
import AppearanceSettings from "./settings/AppearanceSettings";
import CatalogPage from "./catalog/CatalogPage";
import LibraryPage from "./library/LibraryPage";
import WorkspacePage from "./workspace/WorkspacePage";
import useUpdateCheck from "./updates/useUpdateCheck";
import OperationFlow from "./install/OperationFlow";

export default function SdsShell({ profile }: { profile: ProfileInfo }) {
  const account = useSdsSession();
  const [page, setPage] = useState<"store" | "settings" | "library" | "workspace">("store");
  const updates=useUpdateCheck();
  const reviewContext=JSON.stringify([account.connection?.id,account.session?.user?.id,account.generation,account.session?.status]);
  const [operation,setOperation]=useState<{request:OperationRequest;context:string}|null>(null);
  const beginOperation=(request:OperationRequest)=>setOperation({request,context:reviewContext});
  const [libraryRevision,setLibraryRevision]=useState(0);
  const install=(release:Release)=>beginOperation({kind:"install",skillId:release.skillId,releaseId:release.id});
  const library = page === "library";
  const workspace=page==="workspace";
  const settings = page === "settings" || page === "store" && account.connection === null;
  return <div className="sds-app">
    <a className="sds-skip" href="#sds-main">Skip to content</a>
    <aside className="sds-sidebar">
      <div className="sds-brand"><span className="sds-brand-mark"><img src="/ax-portal/icon.png" alt="" onError={(e)=>{e.currentTarget.style.display="none";}} /><span>AX</span></span><div><strong>AX Portal</strong><small>Your internal workspace</small></div></div>
      <nav aria-label="Main navigation">
        <p className="sds-nav-group">SKILLS</p>
        <button disabled={!account.connection} onClick={() => setPage("store")} aria-current={page==="store" && !settings ? "page" : undefined} title={!account.connection ? "Configure your Store connection first" : undefined}><Box size={16} aria-hidden="true" />Skills Store</button>
        <button onClick={() => setPage("library")} aria-current={library ? "page" : undefined}><Library size={16} aria-hidden="true" />Library</button>
        <button onClick={()=>setPage("workspace")} aria-current={workspace?"page":undefined}><Folder size={16} aria-hidden="true"/>Workspaces</button>
        <p className="sds-nav-group">PREFERENCES</p>
        <button onClick={() => setPage("settings")} aria-current={settings ? "page" : undefined}><Settings size={16} aria-hidden="true" />Settings</button>
      </nav>
      <div className="sds-sidebar-footer">
        <div className="sds-sidebar-footer-user"><span className="sds-status-dot" />{account.session?.user?.displayName ?? "AIPro workspace"}</div>
        <span className="sds-environment">{profile.environment === "development" ? "Local development" : "Internal"}{profile.fixtureMode ? " · Fixtures" : ""}</span>
      </div>
      {account.session?.user && <button onClick={() => { void account.logout(); }}>Sign out</button>}
    </aside>
    <main id="sds-main" className="sds-main" tabIndex={-1}>
      {profile.fixtureMode && <p role="status" className="sds-fixture">Development fixtures — sample data and isolated targets. Sample account: employee@example.invalid · Password: synthetic-password. Search __empty, __slow or __unavailable to inspect states.</p>}
      <div className="sds-page-heading"><div><p className="sds-eyebrow">INTERNAL WORKSPACE</p><h1>{workspace?"Workspaces":library ? "Library" : settings ? "Settings" : "Skills Store"}</h1><p>{workspace?"Register projects for independent AIPro skill copies.":library ? "Downloaded releases and their destinations on this computer." : "Find and manage skills from your internal Skills Store."}</p></div><span className="sds-badge" data-status={account.session?.status}>{library ? "Local Library" : account.session?.status === "authenticated" ? "Signed in" : account.session?.status === "offline" ? "Offline" : account.session?.status === "expired" ? "Session expired" : account.connection ? "Connection configured" : "Setup required"}</span></div>
      {account.error && <p role="alert" className="sds-error">{account.error}</p>}
      {workspace?<WorkspacePage key={libraryRevision} onOperation={beginOperation} onLibrary={()=>setPage("library")} session={account.session} profileId={account.connection?.id} updates={updates.data}/>:library ? <LibraryPage key={`${libraryRevision}:${reviewContext}`} onInstall={install} onRemove={(deploymentId,options)=>beginOperation({kind:"remove",deploymentId,...options})} onOperation={beginOperation} onCachedInstall={releaseKey=>beginOperation({kind:"cached",releaseKey})} session={account.session} profileId={account.connection?.id} onRetryConnection={()=>{void (account.retryConnection??account.refreshStatus)();}} updates={updates.data} updatesBusy={updates.busy} updatesError={updates.error} onCheckUpdates={()=>{void updates.refresh();}} /> : settings ? <div className="sds-settings-grid"><ConnectionSettings key={account.connection?.id ?? "new"} current={account.connection ?? null} environment={profile.environment} locked={profile.fixtureMode} onSaved={(connection) => { account.connectionSaved(connection); setPage("store"); }} /><TargetSettings /><AppearanceSettings /></div>
        : !account.connection || !account.session ? <p role="status">Checking your Store connection…</p>
        : account.session.status === "signedOut" || account.session.status === "expired" ? <SignIn key={account.connection.id} onSignedIn={account.signedIn} />
        : null}
      {account.session?.user && account.session.status !== "expired" && <div hidden={page!=="store" || settings}><CatalogPage key={`${account.connection?.id}:${account.session.user.id}:${account.generation}`} onInstall={install} /></div>}
      {operation && operation.context===reviewContext && <OperationFlow key={JSON.stringify(operation)} request={operation.request} chooseTarget onClose={()=>setOperation(null)} onCompleted={()=>setLibraryRevision(value=>value+1)}/>}
    </main>
  </div>;
}
