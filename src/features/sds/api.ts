import { invoke } from "@tauri-apps/api/core";
import type {OperationRequest,OperationResult,PreparedOperation,WorkspaceRef,TargetSelection,Target,SnapshotRecord,SnapshotReason,UpdateCheck,UpdateRequest,PreparedUpdateSelection,UpdateSelectionResult,CacheDeletePlan} from "./contracts";
import type { CatalogQuery, LibraryItem, Page, ProfileInfo, SessionView, SkillDetail, SkillSummary, StoreProfile, Taxonomy, TargetSettings } from "./contracts";

export const getRuntimeProfile = () => invoke<ProfileInfo>("runtime_profile");
export const signIn = (email: string, password: string) => invoke<SessionView>("sds_login", { email, password });
export const getSession = () => invoke<SessionView>("sds_session_status");
export const signOut = () => invoke<void>("sds_logout");
export const getConnection = () => invoke<StoreProfile | null>("sds_get_connection");
export const saveConnection = (baseUrl: string) => invoke<StoreProfile>("sds_set_connection", { baseUrl });
export const getCatalogTaxonomy = () => invoke<Taxonomy>("sds_catalog_taxonomy");
export const searchCatalog = (query: CatalogQuery) => invoke<Page<SkillSummary>>("sds_catalog_search", { query });
export const getSkillDetail = (skillId: string) => invoke<SkillDetail>("sds_skill_detail", { skillId });
export const getTargetSettings = () => invoke<TargetSettings>("sds_get_target_settings");
export const saveTargetOverride = (overrideRoot: string | null) => invoke<TargetSettings>("sds_set_target_override", { overrideRoot });
export const getLibrary = () => invoke<LibraryItem[]>("sds_library");
export const getLibraryPreview = (releaseKey: string) => invoke<string>("sds_library_preview", { releaseKey });
export function prepareOperation(request:OperationRequest) {
  switch(request.kind) {
    case "install": return invoke<PreparedOperation>("sds_prepare_install",{request:{skillId:request.skillId,releaseId:request.releaseId,...(request.target?{target:request.target}:{})}});
    case "remove": return invoke<PreparedOperation>("sds_prepare_remove",{deploymentId:request.deploymentId,...(request.backup!==undefined?{request:{deploymentId:request.deploymentId,backup:request.backup,acknowledgeLocalEdits:request.acknowledgeLocalEdits??false}}:{})});
    case "restore": return invoke<PreparedOperation>("sds_prepare_restore",{snapshotId:request.snapshotId,deploymentId:request.deploymentId,protectCurrent:request.protectCurrent});
    case "update": return invoke<PreparedOperation>("sds_prepare_update",{request:{deploymentId:request.deploymentId,releaseId:request.releaseId,localEdits:request.localEdits}});
    case "cached": return invoke<PreparedOperation>("sds_prepare_cached_install",{releaseKey:request.releaseKey,target:request.target??{agentKey:"aipro",workspaceId:"global"}});
  }
}
export const executeOperation = (operationId:string,ackMcp:boolean) => invoke<{operationId:string}>("sds_execute",{operationId,ackMcp});
export const cancelOperation = (operationId:string) => invoke<OperationResult>("sds_cancel",{operationId});
export const getOperationStatus = (operationId:string) => invoke<OperationResult>("sds_operation_status",{operationId});
export const revealDeployment = (deploymentId:string) => invoke<void>("sds_reveal_deployment",{deploymentId});
export const revealLogs = () => invoke<void>("sds_reveal_logs");
export const listWorkspaces = () => invoke<WorkspaceRef[]>("sds_list_workspaces");
export const addWorkspace = (root:string) => invoke<WorkspaceRef>("sds_add_workspace",{root});
export const removeWorkspace = (workspaceId:string) => invoke<void>("sds_remove_workspace",{workspaceId});
export const resolveTarget = (selection:TargetSelection) => invoke<Target>("sds_resolve_target",{selection});
export const listSnapshots = () => invoke<SnapshotRecord[]>("sds_snapshots");
export const verifySnapshot = (snapshotId:string) => invoke<SnapshotRecord>("sds_verify_snapshot",{snapshotId});
export const captureSnapshot = (deploymentId:string,expectedFingerprint:string,reason:SnapshotReason="manual") => invoke<SnapshotRecord>("sds_capture_snapshot",{deploymentId,expectedFingerprint,reason});
export const checkUpdates = () => invoke<UpdateCheck>("sds_check_updates");
export const prepareUpdates = (requests:UpdateRequest[]) => invoke<PreparedUpdateSelection>("sds_prepare_updates",{requests});
export const executeUpdates = (selectionId:string,acknowledgedOperationIds:string[]) => invoke<{selectionId:string}>("sds_execute_updates",{selectionId,acknowledgedOperationIds});
export const getUpdateSelectionStatus = (selectionId:string) => invoke<UpdateSelectionResult>("sds_update_selection_status",{selectionId});
export const cancelUpdateSelection = (selectionId:string) => invoke<UpdateSelectionResult>("sds_cancel_update_selection",{selectionId});
export const prepareCacheDelete = (releaseKey:string) => invoke<CacheDeletePlan>("sds_prepare_cache_delete",{releaseKey});
export const executeCacheDelete = (planId:string) => invoke<OperationResult>("sds_execute_cache_delete",{planId});
