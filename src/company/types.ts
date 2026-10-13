export interface User { id: string; username: string; displayName: string }
export interface SessionView { user: User | null; expiresAt: string | null; persistent: boolean }
export interface Environment { mock: boolean; storeUrl: string; profileRoot: string; dataRoot: string; configured: boolean; recoveryWarnings?: string[] }
export interface CompanyError { code: string; message: string; requestId?: string; retryAfterSeconds?: number }
export interface PackageMeta { format: 'zip' | '7z'; sizeBytes: number; sha256: string; etag: string }
export interface Skill { id: string; slug: string; name: string; descriptionExcerpt?: string; description?: string; updatedAt: string; package: PackageMeta }
export interface SkillPage { items: Skill[]; pagination: { page: number; pageSize: number; total: number; totalPages: number }; cache?: { fetchedAt: string; stale: boolean } }
export interface Project { id: string; name: string; path: string }
export type ContentState = 'original' | 'customized' | 'unknown' | 'missing';
export type StoreState = 'current' | 'new_package' | 'unknown' | 'unavailable';
export interface Installation { id: string; skillId: string; name: string; target: string; packageHash: string; installedAt: string; mcpBypassed: boolean; projectId?: string | null; contentState?: ContentState; storeState?: StoreState; checkedAt?: string | null; storeError?: string | null }
export interface FilePreview { kind: 'text' | 'binary' | 'directory'; sizeBytes: number }
export interface FileChange { path: string; kind: 'added' | 'modified' | 'deleted'; current?: FilePreview | null; incoming?: FilePreview | null }
export interface Backup { id: string; installationId: string; createdAt: string; reason: string; path: string; fullHash: string; installation: Installation }
export interface McpReport { state: 'not_required' | 'undeclared' | 'present' | 'missing' | 'unknown'; required: string[]; present: string[]; missing: string[]; reason: string | null; checkedAt: string }
export interface OperationView { id: string; state: 'awaiting_decision' | 'downloading' | 'validating' | 'staging' | 'committing' | 'succeeded' | 'cancelled' | 'failed' | 'recovery_required'; target: string; mcp: McpReport; mcpPath?: string; skill?: Skill; progressBytes: number; totalBytes: number; error: CompanyError | null; kind?: 'install' | 'update'; previewReady?: boolean; customized?: boolean; changes?: FileChange[] }

export function errorMessage(error: unknown): string {
  if (typeof error === 'object' && error !== null && 'message' in error && typeof error.message === 'string') return error.message;
  return typeof error === 'string' ? error : 'Không thể hoàn tất thao tác. Vui lòng thử lại.';
}

export function isSessionError(error: unknown): boolean {
  return typeof error === 'object' && error !== null && 'code' in error &&
    (error.code === 'SESSION_EXPIRED' || error.code === 'UNAUTHENTICATED');
}
export function displayPath(path: string): string {
  return path.replace(/^\\\\\?\\UNC\\/i, '\\\\').replace(/^\\\\\?\\/, '');
}
