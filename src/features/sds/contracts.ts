export interface ProfileInfo {
  kind: "upstream" | "sds";
  environment: "development" | "internal";
  fixtureMode: boolean;
}
