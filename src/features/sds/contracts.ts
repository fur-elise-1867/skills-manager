export interface ProfileInfo {
  kind: "upstream" | "sds";
  environment: "development" | "internal";
  fixtureMode: boolean;
}

export interface UserView { id: string; displayName: string }
export interface SessionView {
  status: "signedOut" | "authenticated" | "expired" | "offline";
  user: UserView | null;
}
export interface StoreProfile { id: string; baseUrl: string; environment: ProfileInfo["environment"] }
