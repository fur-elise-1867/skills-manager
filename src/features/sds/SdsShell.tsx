import { useContext, useState } from "react";
import { BookOpen, Box, Library, Settings, SunMoon } from "lucide-react";
import type { ProfileInfo } from "./contracts";
import { SdsThemeContext } from "./theme";
import { useSdsSession } from "./session/context";
import SignIn from "./session/SignIn";
import ConnectionSettings from "./settings/ConnectionSettings";

export default function SdsShell({ profile }: { profile: ProfileInfo }) {
  const { theme, toggle } = useContext(SdsThemeContext);
  const account = useSdsSession();
  const [page, setPage] = useState<"store" | "settings">("store");
  const settings = page === "settings" || account.connection === null;
  return <div className="sds-app">
    <a className="sds-skip" href="#sds-main">Skip to content</a>
    <header className="sds-titlebar"><Box size={15} aria-hidden="true" /><span>SDS Skills Manager</span><span className="sds-environment">{profile.environment === "development" ? "Local development" : "Internal"}</span></header>
    <aside className="sds-sidebar">
      <div className="sds-brand"><span className="sds-brand-mark">SDS</span><div><strong>Skills Manager</strong><small>Your internal workspace</small></div></div>
      <nav aria-label="Main navigation">
        <p className="sds-nav-group">SKILLS</p>
        <button disabled title="Configure your Store connection first"><Box size={16} aria-hidden="true" />Skills Store</button>
        <button disabled title="Your installed skills will appear here"><Library size={16} aria-hidden="true" />Library</button>
        <p className="sds-nav-group">PREFERENCES</p>
        <button onClick={() => setPage("settings")} aria-current={settings ? "page" : undefined}><Settings size={16} aria-hidden="true" />Settings</button>
        <button onClick={toggle} aria-label={`Switch to ${theme === "light" ? "dark" : "light"} theme`}><SunMoon size={16} aria-hidden="true" />{theme === "light" ? "Dark theme" : "Light theme"}</button>
      </nav>
      <div className="sds-sidebar-footer"><span className="sds-status-dot" />{account.session?.user?.displayName ?? "AIPro workspace"}</div>
      {account.session?.user && <button onClick={() => { void account.logout(); }}>Sign out</button>}
    </aside>
    <main id="sds-main" className="sds-main" tabIndex={-1}>
      <div className="sds-page-heading"><div><p className="sds-eyebrow">INTERNAL WORKSPACE</p><h1>{settings ? "Settings" : "Skills Store"}</h1><p>Find and manage skills from your internal Skills Store.</p></div><span className="sds-badge" data-status={account.session?.status}>{account.session?.status === "authenticated" ? "Signed in" : account.connection ? "Connection configured" : "Setup required"}</span></div>
      {account.error && <p role="alert" className="sds-error">{account.error}</p>}
      {settings ? <ConnectionSettings key={account.connection?.id ?? "new"} current={account.connection ?? null} environment={profile.environment} onSaved={(connection) => { account.connectionSaved(connection); setPage("store"); }} />
        : !account.connection || !account.session ? <p role="status">Checking your Store connection…</p>
        : account.session.status === "signedOut" || account.session.status === "expired" ? <SignIn key={account.connection.id} onSignedIn={account.signedIn} />
        : <section className="sds-welcome"><div className="sds-welcome-icon"><BookOpen size={28} aria-hidden="true" /></div><h2>{account.session.status === "offline" ? "Skills Store is unavailable" : "Connected to Skills Store"}</h2><p>{account.session.status === "offline" ? "Check your Store connection before browsing online content." : "Your account is ready. Skill browsing will be available soon."}</p></section>}
      {profile.fixtureMode && <p role="status" className="sds-fixture">Development fixtures — sample data and isolated targets</p>}
    </main>
  </div>;
}
