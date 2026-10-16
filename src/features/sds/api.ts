import { invoke } from "@tauri-apps/api/core";
import type { ProfileInfo, SessionView, StoreProfile } from "./contracts";

export const getRuntimeProfile = () => invoke<ProfileInfo>("runtime_profile");
export const signIn = (email: string, password: string) => invoke<SessionView>("sds_login", { email, password });
export const getSession = () => invoke<SessionView>("sds_session_status");
export const signOut = () => invoke<void>("sds_logout");
export const getConnection = () => invoke<StoreProfile | null>("sds_get_connection");
export const saveConnection = (baseUrl: string) => invoke<StoreProfile>("sds_set_connection", { baseUrl });
