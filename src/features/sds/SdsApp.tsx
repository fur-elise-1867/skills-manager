import { useEffect, useState } from "react";
import { getRuntimeProfile } from "./api";
import type { ProfileInfo } from "./contracts";
import SdsShell from "./SdsShell";
import SdsThemeProvider from "./SdsThemeProvider";
import "./sds.css";
import { assertRuntimeProfile } from "../../runtimeProfile";
import SessionProvider from "./session/SessionProvider";

export default function SdsApp() {
  const [result, setResult] = useState<{ profile?: ProfileInfo; error?: string }>({});
  useEffect(() => {
    let active = true;
    getRuntimeProfile().then((profile) => {
      const environment = import.meta.env.VITE_SDS_ENVIRONMENT ?? "internal";
      const fixtures = import.meta.env.DEV && import.meta.env.VITE_SDS_FIXTURES === "1";
      assertRuntimeProfile(profile, "sds", environment, fixtures);
      if (active) setResult({ profile });
    }).catch(() => {
      if (active) setResult({ error: "Desktop profile mismatch. Start SDS Skills Manager with its matching desktop runtime." });
    });
    return () => { active = false; };
  }, []);
  if (result.error) return <main className="sds-startup" role="alert"><h1>Unable to start SDS Skills Manager</h1><p>{result.error}</p></main>;
  if (!result.profile) return <main className="sds-startup" role="status">Checking desktop runtime…</main>;
  return <SdsThemeProvider><SessionProvider><SdsShell profile={result.profile} /></SessionProvider></SdsThemeProvider>;
}
