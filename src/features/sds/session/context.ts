import { createContext, useContext } from "react";
import type { SessionView, StoreProfile } from "../contracts";

export interface SdsSessionState {
  session: SessionView | undefined;
  connection: StoreProfile | null | undefined;
  generation: number;
  error: string;
  signedIn: (view: SessionView) => void;
  connectionSaved: (profile: StoreProfile) => void;
  logout: () => Promise<void>;
  refreshStatus: () => Promise<void>;
}
export const SdsSessionContext = createContext<SdsSessionState | null>(null);
export function useSdsSession() {
  const value = useContext(SdsSessionContext);
  if (!value) throw new Error("SDS session provider is missing");
  return value;
}
