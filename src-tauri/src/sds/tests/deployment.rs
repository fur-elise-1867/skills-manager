use crate::sds::{deployment::fingerprint_tree, packages::Manifest};
use std::path::Path;

fn tree(root: &Path) {
    std::fs::create_dir(root).unwrap();
    std::fs::write(root.join("SKILL.md"), b"# Owned test skill").unwrap();
    std::fs::create_dir(root.join("resources")).unwrap();
    std::fs::write(root.join("resources/data.bin"), [0, 255, 17, 0]).unwrap();
}

#[cfg(windows)]
#[test]
fn the_repository_refuses_hardlinked_database_and_sidecar_files_before_sqlite_writes() {
    use crate::sds::repository::Repository;
    for sidecar in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let data = root.path().join("sds");
        std::fs::create_dir(&data).unwrap();
        let outside = root.path().join("other-user-data.sqlite3");
        let connection = rusqlite::Connection::open(&outside).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE sentinel(value TEXT); INSERT INTO sentinel VALUES('keep');",
            )
            .unwrap();
        drop(connection);
        let before = std::fs::read(&outside).unwrap();
        std::fs::hard_link(
            &outside,
            data.join(if sidecar {
                "pilot.sqlite3-journal"
            } else {
                "pilot.sqlite3"
            }),
        )
        .unwrap();
        assert!(Repository::open(&data).is_err());
        assert_eq!(std::fs::read(&outside).unwrap(), before);
    }
}

#[test]
fn a_target_fingerprint_detects_added_changed_and_deleted_files_and_empty_directories() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("skill");
    tree(&root);
    let (manifest, baseline) = fingerprint_tree(&root).unwrap();
    assert_eq!(baseline.len(), 64);
    let encoded = serde_json::to_string(&manifest).unwrap();
    let saved: Manifest = serde_json::from_str(&encoded).unwrap();
    assert_eq!(manifest, saved);
    std::fs::write(root.join("new.txt"), b"local addition").unwrap();
    assert_ne!(fingerprint_tree(&root).unwrap().1, baseline);
    std::fs::remove_file(root.join("new.txt")).unwrap();
    assert_eq!(fingerprint_tree(&root).unwrap().1, baseline);
    std::fs::write(root.join("resources/data.bin"), [0, 255, 17, 1]).unwrap();
    assert_ne!(fingerprint_tree(&root).unwrap().1, baseline);
    std::fs::write(root.join("resources/data.bin"), [0, 255, 17, 0]).unwrap();
    std::fs::create_dir(root.join("empty-directory")).unwrap();
    assert_ne!(fingerprint_tree(&root).unwrap().1, baseline);
    std::fs::remove_dir(root.join("empty-directory")).unwrap();
    std::fs::remove_file(root.join("resources/data.bin")).unwrap();
    assert_ne!(fingerprint_tree(&root).unwrap().1, baseline);
}

#[test]
fn immutable_release_cache_survives_restart_scopes_store_ids_and_detects_local_tampering() {
    use crate::sds::{contracts::Release, packages::VerifiedArtifact, repository::Repository};
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    tree(&source);
    let (manifest, fingerprint) = fingerprint_tree(&source).unwrap();
    let artifact = VerifiedArtifact {
        root: source.clone(),
        manifest,
        fingerprint,
    };
    let release: Release = serde_json::from_value(
        serde_json::from_str::<serde_json::Value>(include_str!(
            "../../../../contracts/sds-desktop/v1/fixtures/release.json"
        ))
        .unwrap()["data"]
            .clone(),
    )
    .unwrap();
    let data = root.path().join("sds-data");
    let repo = Repository::open(&data).unwrap();
    let cached = repo.cache_release("store-a", &release, &artifact).unwrap();
    assert_eq!(
        std::fs::read(cached.artifact.root.join("resources/data.bin")).unwrap(),
        [0, 255, 17, 0]
    );
    let again = repo.cache_release("store-a", &release, &artifact).unwrap();
    assert_eq!(again.key, cached.key);
    let other = repo.cache_release("store-b", &release, &artifact).unwrap();
    assert_ne!(other.key, cached.key);
    drop(repo);
    let restarted = Repository::open(&data).unwrap();
    assert_eq!(restarted.list_library().unwrap().len(), 2);
    let cache = restarted
        .cached_release("store-a", &release.skill_id, &release.id)
        .unwrap()
        .unwrap();
    assert_eq!(cache.artifact.fingerprint, artifact.fingerprint);
    let mut changed = release.clone();
    changed.package.sha256 = "0".repeat(64);
    assert_eq!(
        restarted
            .cache_release("store-a", &changed, &artifact)
            .unwrap_err()
            .code,
        "RELEASE_CHANGED"
    );
    std::fs::write(
        cache.artifact.root.join("resources/data.bin"),
        b"local change",
    )
    .unwrap();
    assert_eq!(
        restarted
            .cached_release("store-a", &release.skill_id, &release.id)
            .unwrap_err()
            .code,
        "CACHE_CORRUPT"
    );
    assert_eq!(
        std::fs::read(source.join("resources/data.bin")).unwrap(),
        [0, 255, 17, 0]
    );
}

#[test]
fn cache_rechecks_artifact_bytes_and_rejects_a_tampered_database_path_key() {
    use crate::sds::{contracts::Release, packages::VerifiedArtifact, repository::Repository};
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    tree(&source);
    let (manifest, fingerprint) = fingerprint_tree(&source).unwrap();
    let artifact = VerifiedArtifact {
        root: source.clone(),
        manifest,
        fingerprint,
    };
    let release: Release = serde_json::from_value(
        serde_json::from_str::<serde_json::Value>(include_str!(
            "../../../../contracts/sds-desktop/v1/fixtures/release.json"
        ))
        .unwrap()["data"]
            .clone(),
    )
    .unwrap();
    let data = root.path().join("sds-data");
    let repo = Repository::open(&data).unwrap();
    std::fs::write(source.join("SKILL.md"), b"changed after verification").unwrap();
    assert_eq!(
        repo.cache_release("store", &release, &artifact)
            .unwrap_err()
            .code,
        "CACHE_CORRUPT"
    );
    assert!(repo.list_library().unwrap().is_empty());
    std::fs::write(source.join("SKILL.md"), b"# Owned test skill").unwrap();
    let cached = repo.cache_release("store", &release, &artifact).unwrap();
    drop(repo);
    let connection = rusqlite::Connection::open(data.join("pilot.sqlite3")).unwrap();
    connection
        .execute(
            "UPDATE releases SET cache_key='../outside' WHERE cache_key=?1",
            [cached.key],
        )
        .unwrap();
    drop(connection);
    let reopened = Repository::open(&data).unwrap();
    assert_eq!(
        reopened
            .cached_release("store", &release.skill_id, &release.id)
            .unwrap_err()
            .code,
        "CACHE_CORRUPT"
    );
    assert!(!root.path().join("outside").exists());
}

#[test]
fn prepared_package_and_deployment_fingerprints_agree_including_empty_directories() {
    use crate::sds::{
        contracts::Release,
        packages::{prepare_package, PackageLimits},
    };
    use std::io::{Cursor, Write};
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file("skill/SKILL.md", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"# Safe").unwrap();
    zip.add_directory("skill/empty/", zip::write::SimpleFileOptions::default())
        .unwrap();
    let bytes = zip.finish().unwrap().into_inner();
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("archive.zip");
    std::fs::write(&path, &bytes).unwrap();
    let mut release: Release = serde_json::from_value(
        serde_json::from_str::<serde_json::Value>(include_str!(
            "../../../../contracts/sds-desktop/v1/fixtures/release.json"
        ))
        .unwrap()["data"]
            .clone(),
    )
    .unwrap();
    use sha2::{Digest, Sha256};
    release.package.sha256 = hex::encode(Sha256::digest(&bytes));
    release.package.size_bytes = bytes.len() as u64;
    let prepared = prepare_package(
        &path,
        &release,
        &root.path().join("verified"),
        &PackageLimits::default(),
    )
    .unwrap();
    assert_eq!(
        fingerprint_tree(&prepared.root).unwrap(),
        (prepared.manifest, prepared.fingerprint)
    );
}

#[cfg(windows)]
#[test]
fn fingerprint_refuses_junctions_inside_a_tree_or_its_ancestors() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    tree(&source);
    let external = tempfile::tempdir().unwrap();
    std::fs::write(external.path().join("sentinel"), b"keep").unwrap();
    junction::create(external.path(), source.join("linked")).unwrap();
    assert!(fingerprint_tree(&source).is_err());
    junction::create(&source, root.path().join("alias")).unwrap();
    assert!(fingerprint_tree(&root.path().join("alias")).is_err());
    assert_eq!(
        std::fs::read(external.path().join("sentinel")).unwrap(),
        b"keep"
    );
}

#[test]
fn frozen_operations_survive_restart_without_storing_credentials_and_terminal_results_are_immutable(
) {
    use crate::sds::{
        contracts::{
            InstallRequest, McpReport, McpState, OperationResult, OperationState,
            PreparedOperation, Release, SessionContext, Target,
        },
        repository::Repository,
    };
    let root = tempfile::tempdir().unwrap();
    let data = root.path().join("sds-data");
    let repo = Repository::open(&data).unwrap();
    let release: Release = serde_json::from_value(
        serde_json::from_str::<serde_json::Value>(include_str!(
            "../../../../contracts/sds-desktop/v1/fixtures/release.json"
        ))
        .unwrap()["data"]
            .clone(),
    )
    .unwrap();
    let context = SessionContext {
        profile_id: "store-a".into(),
        account_id: "1001".into(),
        generation: 17,
    };
    let request = InstallRequest {
        skill_id: release.skill_id.clone(),
        release_id: release.id.clone(),
    };
    std::fs::create_dir(root.path().join("agent")).unwrap();
    let operation = PreparedOperation {
        id: uuid::Uuid::new_v4().to_string(),
        kind: "install".into(),
        release,
        target: Target {
            agent_key: "aipro".into(),
            workspace_id: "global".into(),
            path: root.path().join("agent/skills/jira-weekly-report"),
        },
        reviewed_root: crate::sds::contracts::ReviewedRoot::capture(
            &root.path().join("agent/skills"),
        )
        .unwrap(),
        expected_fingerprint: None,
        mcp: McpReport {
            state: McpState::Unknown,
            missing: vec![],
            reason: None,
            checked_at: chrono::Utc::now().to_rfc3339(),
        },
        requires_mcp_ack: true,
        can_execute: true,
        conflict: None,
        context,
        runtime_epoch: "test-runtime-one".into(),
    };
    assert_eq!(request.skill_id, operation.release.skill_id);
    let pending = OperationResult {
        operation_id: operation.id.clone(),
        state: OperationState::Prepared,
        stage: "review".into(),
        error: None,
    };
    let mut invalid_plan = operation.clone();
    invalid_plan.id = uuid::Uuid::new_v4().to_string();
    let invalid = OperationResult {
        operation_id: invalid_plan.id.clone(),
        stage: String::new(),
        ..pending.clone()
    };
    assert!(repo.create_operation(&invalid_plan, &invalid).is_err());
    repo.create_operation(&operation, &pending).unwrap();
    drop(repo);
    let reopened = Repository::open(&data).unwrap();
    let saved = reopened.load_operation(&operation.id).unwrap();
    assert_eq!(saved.context.generation, 17);
    assert_eq!(saved.target.path, operation.target.path);
    assert_eq!(
        reopened.operation_status(&operation.id).unwrap().state,
        OperationState::Prepared
    );
    let completed = OperationResult {
        state: OperationState::Failed,
        stage: "validation".into(),
        ..pending.clone()
    };
    reopened.record_result(&completed).unwrap();
    assert_eq!(
        reopened.operation_status(&operation.id).unwrap().state,
        OperationState::Failed
    );
    assert!(reopened.record_result(&pending).is_err());
    let body = serde_json::to_string(&saved).unwrap();
    assert!(!body.contains("accessToken"));
    assert!(!body.contains("refreshToken"));
}
