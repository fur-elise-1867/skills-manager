import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { getConnection, getSession, signOut } from "../api";
import type { SessionView, StoreProfile } from "../contracts";
import { storeErrorMessage } from "../errors";
import { SdsSessionContext } from "./context";

const signedOut: SessionView = { status: "signedOut", user: null };
export default function SessionProvider({ children }: { children: ReactNode }) {
  const [session, setSession] = useState<SessionView>();
  const [connection, setConnection] = useState<StoreProfile | null>();
  const [generation, setGeneration] = useState(0);
  const [error, setError] = useState("");
  const epoch = useRef(0);
  useEffect(() => {
    let active = true; const started = epoch.current;
    getConnection().then((profile) => { if (active && epoch.current === started) setConnection(profile); }).catch((failure) => {
      if (active && epoch.current === started) { setConnection(null); setError(storeErrorMessage(failure)); }
    });
    getSession().then((view) => { if (active && epoch.current === started) setSession(view); }).catch((failure) => {
      if (active && epoch.current === started) { setSession(signedOut); setError(storeErrorMessage(failure)); }
    });
    return () => { active = false; };
  }, []);
  const signedIn = useCallback((view: SessionView) => { epoch.current += 1; setGeneration(epoch.current); setSession(view); setError(""); }, []);
  const connectionSaved = useCallback((profile: StoreProfile) => {
    if (profile.id === connection?.id) { setConnection(profile); setError(""); return; }
    epoch.current += 1; setGeneration(epoch.current); setConnection(profile); setSession(signedOut); setError("");
  }, [connection?.id]);
  const logout = useCallback(async () => {
    const started = ++epoch.current; setGeneration(started); setSession(signedOut); setError("");
    try { await signOut(); } catch (failure) { if (epoch.current === started) setError(storeErrorMessage(failure)); }
  }, []);
  const refreshStatus = useCallback(async () => {
    const started = epoch.current;
    try { const view = await getSession(); if (epoch.current === started) setSession(view); }
    catch (failure) { if (epoch.current === started) setError(storeErrorMessage(failure)); }
  }, []);
  const value = useMemo(() => ({ session, connection, generation, error, signedIn, connectionSaved, logout, refreshStatus }), [session, connection, generation, error, signedIn, connectionSaved, logout, refreshStatus]);
  return <SdsSessionContext.Provider value={value}>{children}</SdsSessionContext.Provider>;
}
