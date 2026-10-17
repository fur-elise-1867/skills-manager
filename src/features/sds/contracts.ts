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

export type CatalogMode = "all_time" | "trending" | "hot";
export interface CatalogQuery { q: string; mode: CatalogMode; sourceId: string | null; page: number; pageSize: number }
export interface Page<T> { items: T[]; page: number; pageSize: number; totalItems: number }
export interface Taxonomy { sources: {id:string; label:string}[]; catalogModes: {id:CatalogMode; label:string; available:boolean}[] }
export interface ReleaseRef { id: string; sequence: number; version: string }
export interface SkillSummary {
  id: string; name: string; description: string; metadataRevision: number;
  roles: string[]; tags: string[]; categoryIds: string[]; sourceIds: string[];
  creator: UserView; latestRelease: ReleaseRef | null; compatibleAgents: string[] | null;
  canDownload: boolean; updatedAt: string;
}
export interface PackageInfo { format: "zip" | "7z"; fileName: string; sizeBytes: number; sha256: string; downloadPath: string }
export interface Release {
  skillId: string; id: string; sequence: number; version: string; installName: string;
  publishedAt: string; changelog: string; compatibleAgents: string[] | null; requiredMcps: string[] | null; package: PackageInfo;
}
export interface SkillDetail { summary: SkillSummary; storePageUrl: string | null; latestRelease: Release | null }
export interface Target { agentKey: string; workspaceId: string; path: string }
export interface McpReport { state: "notRequired" | "unknown" | "configured" | "missing" | "unverifiable"; missing: string[]; reason: string | null; checkedAt: string }
export interface PreparedOperation {
  id: string; kind: "install" | "remove"; release: Release; target: Target; expectedFingerprint: string | null;
  mcp: McpReport; requiresMcpAck: boolean; canExecute: boolean; conflict: string | null;
}
