import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { getTargetSettings, saveTargetOverride } from "../api";
import type { TargetSettings as Settings } from "../contracts";
import { storeErrorMessage } from "../errors";

export default function TargetSettings() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  useEffect(() => {
    let active = true;
    getTargetSettings().then(value => { if (active) setSettings(value); })
      .catch(failure => { if (active) setError(storeErrorMessage(failure)); });
    return () => { active = false; };
  }, []);
  async function change(choose: boolean) {
    if (pending) return;
    setPending(true); setError("");
    try {
      const path = choose ? await open({ directory: true, multiple: false, title: "Choose AIPro skills folder" }) : null;
      if (choose && path === null) return;
      if (Array.isArray(path)) throw new Error("Expected one folder");
      setSettings(await saveTargetOverride(path));
    } catch (failure) { setError(storeErrorMessage(failure)); }
    finally { setPending(false); }
  }
  return <section className="sds-welcome" aria-labelledby="sds-target-title">
    <h2 id="sds-target-title">AIPro destination</h2>
    <p>Skills install to your global AIPro skills folder. You can select an existing local skills folder after preparing AIPro.</p>
    {settings?.target && <p className="sds-target-path">{settings.target.path}</p>}
    {settings?.error && <p role="status">{storeErrorMessage(settings.error)}</p>}
    {!settings && !error && <p role="status">Checking AIPro destination…</p>}
    {settings?.locked && <p>Development fixtures use an isolated destination fixed for this session.</p>}
    <p className="sds-field-help">This setting does not install AIPro or change its MCP configuration. MCP checks use the global AIPro configuration.</p>
    {error && <p role="alert" className="sds-error">{error}</p>}
    <div className="sds-review-actions">
      <button type="button" disabled={pending || settings?.locked || (!settings && !error)} onClick={() => { void change(true); }}>{pending ? "Saving…" : "Choose skills folder"}</button>
      <button type="button" disabled={pending || settings?.locked || (!settings && !error)} onClick={() => { void change(false); }}>Use default destination</button>
    </div>
  </section>;
}
