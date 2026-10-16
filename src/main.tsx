import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { getRuntimeProfile } from "./features/sds/api";
import { assertRuntimeProfile } from "./runtimeProfile";

if (import.meta.env.VITE_APP_PROFILE === "sds") {
  const { default: SdsApp } = await import("./features/sds/SdsApp");
  createRoot(document.getElementById("root")!).render(<StrictMode><SdsApp /></StrictMode>);
} else {
  try {
    assertRuntimeProfile(await getRuntimeProfile(), "upstream");
  const [{ i18nReady }, { logStartupEvent }, { default: App }] = await Promise.all([
    import("./i18n"), import("./lib/tauri"), import("./App.tsx"), import("./index.css"),
  ]);
  await i18nReady;
  logStartupEvent("i18n_ready", performance.now()).catch(() => {});
  createRoot(document.getElementById("root")!).render(<StrictMode><App /></StrictMode>);
  logStartupEvent("root_rendered", performance.now()).catch(() => {});
  } catch {
    createRoot(document.getElementById("root")!).render(<main role="alert" style={{ padding: 40, fontFamily: "Segoe UI, sans-serif" }}><h1>Unable to start Skills Manager</h1><p>Desktop profile mismatch. Start this application with its matching desktop runtime.</p></main>);
  }
}
