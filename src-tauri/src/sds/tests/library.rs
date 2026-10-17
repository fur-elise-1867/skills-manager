use super::operations::{package_server, request, setup};
use crate::sds::{
    contracts::OperationState,
    library::{CacheStatus, DeploymentStatus},
};

#[tokio::test]
async fn library_is_local_and_reports_healthy_modified_missing_and_replaced_deployments() {
    let server = package_server("zip", false).await;
    let root = tempfile::tempdir().unwrap();
    let (services, repo, operations) = setup(&server, root.path()).await;
    let plan = operations.prepare_install(request()).await.unwrap();
    assert_eq!(
        operations.execute(&plan.id, true).await.unwrap().state,
        OperationState::Completed
    );
    services.session.logout().await.unwrap();
    drop(server);
    let repository_root = repo.root().to_path_buf();
    drop(operations);
    drop(repo);
    let repo =
        std::sync::Arc::new(crate::sds::repository::Repository::open(&repository_root).unwrap());
    let operations = crate::sds::operations::OperationService::new(
        services.session.clone(),
        services.targets.clone(),
        repo.clone(),
    );
    let original = std::fs::read(plan.target.path.join("SKILL.md")).unwrap();
    let installed = operations.library().await.unwrap();
    let deployment_id = &installed[0].deployments[0].id;
    crate::sds::library::with_deployment_path(&repo, deployment_id, |path| {
        assert_eq!(path, plan.target.path);
        Ok(())
    })
    .unwrap();
    assert!(
        crate::sds::library::with_deployment_path::<()>(&repo, "../../unmanaged", |_| panic!(
            "invalid ID reached opener"
        ))
        .is_err()
    );
    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].cache_status, CacheStatus::Verified);
    assert_eq!(installed[0].release.id, plan.release.id);
    assert_eq!(installed[0].deployments[0].target, plan.target);
    assert_eq!(
        installed[0].deployments[0].status,
        DeploymentStatus::Installed
    );
    assert_eq!(installed[0].deployments[0].mcp.missing, plan.mcp.missing);
    std::fs::create_dir(plan.target.path.join("local-empty-directory")).unwrap();
    assert_eq!(
        operations.library().await.unwrap()[0].deployments[0].status,
        DeploymentStatus::Modified
    );
    std::fs::remove_dir(plan.target.path.join("local-empty-directory")).unwrap();
    let saved = root.path().join("moved-by-user");
    std::fs::rename(&plan.target.path, &saved).unwrap();
    assert!(
        crate::sds::library::with_deployment_path::<()>(&repo, deployment_id, |_| panic!(
            "missing target reached opener"
        ))
        .is_err()
    );
    assert_eq!(
        operations.library().await.unwrap()[0].deployments[0].status,
        DeploymentStatus::Missing
    );
    std::fs::create_dir(&plan.target.path).unwrap();
    std::fs::write(plan.target.path.join("SKILL.md"), &original).unwrap();
    assert!(
        crate::sds::library::with_deployment_path::<()>(&repo, deployment_id, |_| panic!(
            "replacement target reached opener"
        ))
        .is_err()
    );
    assert_eq!(
        operations.library().await.unwrap()[0].deployments[0].status,
        DeploymentStatus::Unverifiable
    );
    assert_eq!(std::fs::read(saved.join("SKILL.md")).unwrap(), original);
    assert_eq!(repo.deployments().unwrap().len(), 1);
}

#[tokio::test]
async fn a_corrupted_cache_does_not_claim_its_independent_target_was_modified() {
    let server = package_server("zip", false).await;
    let root = tempfile::tempdir().unwrap();
    let (_, repo, operations) = setup(&server, root.path()).await;
    let plan = operations.prepare_install(request()).await.unwrap();
    operations.execute(&plan.id, true).await.unwrap();
    let cached = repo
        .cached_release(
            &plan.context.profile_id,
            &plan.release.skill_id,
            &plan.release.id,
        )
        .unwrap()
        .unwrap();
    let preview = operations.preview(&cached.key).await.unwrap();
    assert!(!preview.is_empty());
    std::fs::write(
        cached.artifact.root.join("SKILL.md"),
        b"unexpected cache content",
    )
    .unwrap();
    let library = operations.library().await.unwrap();
    assert_eq!(library[0].cache_status, CacheStatus::Corrupt);
    assert_eq!(
        library[0].deployments[0].status,
        DeploymentStatus::Installed
    );
    assert_eq!(
        operations.preview(&cached.key).await.unwrap_err().code,
        "CACHE_CORRUPT"
    );
    assert!(operations.preview("../outside").await.is_err());
    assert!(!root.path().join("outside").exists());
}

#[cfg(windows)]
#[tokio::test]
async fn an_unsafe_deployment_link_is_reported_without_losing_the_rest_of_library() {
    let server = package_server("zip", false).await;
    let root = tempfile::tempdir().unwrap();
    let (_, _, operations) = setup(&server, root.path()).await;
    let plan = operations.prepare_install(request()).await.unwrap();
    operations.execute(&plan.id, true).await.unwrap();
    let external = root.path().join("unmanaged");
    std::fs::create_dir(&external).unwrap();
    std::fs::write(external.join("keep"), b"keep").unwrap();
    std::fs::rename(&plan.target.path, root.path().join("owned-copy")).unwrap();
    junction::create(&external, &plan.target.path).unwrap();
    let library = operations.library().await.unwrap();
    assert_eq!(library.len(), 1);
    assert_eq!(
        library[0].deployments[0].status,
        DeploymentStatus::Unverifiable
    );
    assert_eq!(std::fs::read(external.join("keep")).unwrap(), b"keep");
}
