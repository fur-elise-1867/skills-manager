import { invoke } from "@tauri-apps/api/core";
import type { ProfileInfo } from "./contracts";

export const getRuntimeProfile = () => invoke<ProfileInfo>("runtime_profile");
