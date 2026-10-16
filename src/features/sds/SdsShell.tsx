import { useContext } from "react";
import { BookOpen, Box, CircleHelp, Library, Settings, SunMoon } from "lucide-react";
import type { ProfileInfo } from "./contracts";
import { SdsThemeContext } from "./theme";

export default function SdsShell({ profile }: { profile: ProfileInfo }) {
  const { theme, toggle } = useContext(SdsThemeContext);
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
        <button disabled title="Connection settings will be available soon"><Settings size={16} aria-hidden="true" />Settings</button>
        <button onClick={toggle} aria-label={`Switch to ${theme === "light" ? "dark" : "light"} theme`}><SunMoon size={16} aria-hidden="true" />{theme === "light" ? "Dark theme" : "Light theme"}</button>
      </nav>
      <div className="sds-sidebar-footer"><span className="sds-status-dot" />AIPro workspace</div>
    </aside>
    <main id="sds-main" className="sds-main" tabIndex={-1}>
      <div className="sds-page-heading"><div><p className="sds-eyebrow">GET STARTED</p><h1>Your skills workspace</h1><p>Find and manage skills from your internal Skills Store.</p></div><span className="sds-badge">Setup required</span></div>
      <section className="sds-welcome" aria-labelledby="sds-welcome-title"><div className="sds-welcome-icon"><BookOpen size={28} aria-hidden="true" /></div><h2 id="sds-welcome-title">Connect your Skills Store</h2><p>Your Store connection needs to be configured before you can sign in and browse skills.</p><div className="sds-notice"><CircleHelp size={17} aria-hidden="true" /><span>Connection setup will be available soon. Your existing agent skills stay in their current folders.</span></div></section>
      {profile.fixtureMode && <p role="status" className="sds-fixture">Development fixtures — sample data and isolated targets</p>}
    </main>
  </div>;
}
