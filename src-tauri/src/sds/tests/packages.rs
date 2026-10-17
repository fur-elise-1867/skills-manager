use crate::sds::{
    contracts::Release,
    packages::{prepare_package, PackageLimits},
};
use sha2::{Digest, Sha256};
#[path = "package_adversarial.rs"]
mod adversarial;
use std::{
    io::{Cursor, Write},
    path::Path,
};

fn zip_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        zip.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap().into_inner()
}
fn release(bytes: &[u8], format: &str) -> Release {
    let mut release: Release = serde_json::from_value(
        serde_json::from_str::<serde_json::Value>(include_str!(
            "../../../../contracts/sds-desktop/v1/fixtures/release.json"
        ))
        .unwrap()["data"]
            .clone(),
    )
    .unwrap();
    release.package.format = serde_json::from_value(serde_json::json!(format)).unwrap();
    release.package.file_name = format!("synthetic.{format}");
    release.package.size_bytes = bytes.len() as u64;
    release.package.sha256 = hex::encode(Sha256::digest(bytes));
    release
}
fn write_archive(root: &Path, bytes: &[u8]) -> std::path::PathBuf {
    let path = root.join("package.archive");
    std::fs::write(&path, bytes).unwrap();
    path
}

fn seven_zip_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = sevenz_turbo::ArchiveWriter::new(Cursor::new(Vec::new())).unwrap();
    for (name, bytes) in entries {
        writer
            .push_archive_entry(
                sevenz_turbo::ArchiveEntry::new_file(name),
                Some(Cursor::new(*bytes)),
            )
            .unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[test]
fn seven_zip_and_zip_accept_one_enclosing_folder_with_identical_content_fingerprints() {
    let root = tempfile::tempdir().unwrap();
    let entries: &[(&str, &[u8])] = &[
        ("wrapped/SKILL.md", b"# Synthetic\n"),
        ("wrapped/resources/data.bin", &[0, 255, 17, 0]),
        ("wrapped/resources/hướng-dẫn.txt", "Tiếng Việt".as_bytes()),
    ];
    let mut manifests = Vec::new();
    for (format, bytes) in [
        ("zip", zip_bytes(entries)),
        ("7z", seven_zip_bytes(entries)),
    ] {
        let archive = write_archive(root.path(), &bytes);
        let artifact = prepare_package(
            &archive,
            &release(&bytes, format),
            &root.path().join(format),
            &PackageLimits::default(),
        )
        .unwrap();
        assert_eq!(
            std::fs::read(artifact.root.join("resources/data.bin")).unwrap(),
            [0, 255, 17, 0]
        );
        assert!(artifact.root.join("resources/hướng-dẫn.txt").is_file());
        assert!(!artifact.root.join("wrapped").exists());
        manifests.push((artifact.manifest, artifact.fingerprint));
    }
    assert_eq!(manifests[0], manifests[1]);
}

#[test]
fn verified_zip_preserves_binary_and_unicode_resources_and_builds_a_content_manifest() {
    let root = tempfile::tempdir().unwrap();
    let bytes = zip_bytes(&[
        ("SKILL.md", b"# Synthetic\n"),
        ("resources/huong-dan.txt", "Tiếng Việt".as_bytes()),
        ("resources/data.bin", &[0, 255, 17, 0]),
    ]);
    let archive = write_archive(root.path(), &bytes);
    let artifact = prepare_package(
        &archive,
        &release(&bytes, "zip"),
        &root.path().join("verified"),
        &PackageLimits::default(),
    )
    .unwrap();
    assert_eq!(
        std::fs::read(artifact.root.join("resources/data.bin")).unwrap(),
        [0, 255, 17, 0]
    );
    assert_eq!(
        std::fs::read_to_string(artifact.root.join("resources/huong-dan.txt")).unwrap(),
        "Tiếng Việt"
    );
    assert_eq!(
        artifact
            .manifest
            .entries
            .iter()
            .filter(|entry| entry.sha256.is_some())
            .count(),
        3
    );
    assert_eq!(artifact.fingerprint.len(), 64);
    assert_eq!(
        artifact
            .manifest
            .entries
            .iter()
            .find(|entry| entry.path == "resources/data.bin")
            .unwrap()
            .sha256
            .as_deref(),
        Some("c1be14cd5e104a6eb72acd516d91fe14c6fc00a57f3c2a45686586f43690938b")
    );
}

fn assert_refused(bytes: &[u8], format: &str, expected: Option<&str>) {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("existing-agent");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("unmanaged.bin"), [7, 0, 255]).unwrap();
    let archive = write_archive(root.path(), bytes);
    let staging = root.path().join("verified");
    let error = prepare_package(
        &archive,
        &release(bytes, format),
        &staging,
        &PackageLimits::default(),
    )
    .unwrap_err();
    if let Some(expected) = expected {
        assert_eq!(error.code, expected, "{error}");
    }
    assert!(!staging.exists());
    assert_eq!(
        std::fs::read(target.join("unmanaged.bin")).unwrap(),
        [7, 0, 255]
    );
    assert_eq!(std::fs::read_dir(&target).unwrap().count(), 1);
    assert!(!std::fs::read_dir(root.path()).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".sds-package-")));
}

#[test]
fn both_formats_refuse_ambiguous_missing_and_nested_only_roots_without_changing_the_target() {
    for entries in [
        vec![("readme.txt", b"no skill".as_slice())],
        vec![
            ("a/SKILL.md", b"one".as_slice()),
            ("b/SKILL.md", b"two".as_slice()),
        ],
        vec![("a/b/SKILL.md", b"too deep".as_slice())],
        vec![("nested.zip", b"PK synthetic".as_slice())],
        vec![
            ("a/SKILL.md", b"one".as_slice()),
            ("outside.txt", b"unrelated".as_slice()),
        ],
    ] {
        for (format, bytes) in [
            ("zip", zip_bytes(&entries)),
            ("7z", seven_zip_bytes(&entries)),
        ] {
            assert_refused(&bytes, format, Some("PACKAGE_ROOT"));
        }
    }
}

#[test]
fn both_formats_reject_windows_aliases_path_escapes_and_case_collisions() {
    for name in [
        "../escape.txt",
        "/absolute.txt",
        "C:/drive.txt",
        "//server/share/file",
        "folder\\backslash",
        "folder/../escape",
        "folder/./file",
        "folder//file",
        "name:stream",
        "CON.txt",
        "LPT1",
        "COM¹.txt",
        "name.",
        "name ",
    ] {
        let entries: &[(&str, &[u8])] = &[("SKILL.md", b"# Safe"), (name, b"must not escape")];
        for (format, bytes) in [
            ("zip", zip_bytes(entries)),
            ("7z", seven_zip_bytes(entries)),
        ] {
            assert_refused(&bytes, format, None);
        }
    }
    for entries in [
        vec![
            ("SKILL.md", b"one".as_slice()),
            ("skill.md", b"two".as_slice()),
        ],
        vec![
            ("SKILL.md", b"one".as_slice()),
            ("Resources/a", b"a".as_slice()),
            ("resources/b", b"b".as_slice()),
        ],
        vec![
            ("SKILL.md", b"one".as_slice()),
            ("x", b"file".as_slice()),
            ("x/child", b"child".as_slice()),
        ],
        vec![
            ("SKILL.md", b"one".as_slice()),
            ("é.txt", b"NFC".as_slice()),
            ("é.txt", b"NFD".as_slice()),
        ],
    ] {
        for (format, bytes) in [
            ("zip", zip_bytes(&entries)),
            ("7z", seven_zip_bytes(&entries)),
        ] {
            assert_refused(&bytes, format, None);
        }
    }
}

#[test]
fn package_integrity_limits_cancellation_and_existing_staging_fail_without_publication() {
    use crate::sds::packages::prepare_package_cancellable;
    use std::sync::atomic::AtomicBool;
    let root = tempfile::tempdir().unwrap();
    for (format, bytes) in [
        ("zip", zip_bytes(&[("SKILL.md", &[0u8; 2048])])),
        ("7z", seven_zip_bytes(&[("SKILL.md", &[0u8; 2048])])),
    ] {
        let archive = write_archive(root.path(), &bytes);
        let staging = root.path().join("verified");
        let good = release(&bytes, format);
        let mut bad = good.clone();
        bad.package.sha256 = "0".repeat(64);
        assert_eq!(
            prepare_package(&archive, &bad, &staging, &PackageLimits::default())
                .unwrap_err()
                .code,
            "PACKAGE_INTEGRITY"
        );
        bad = good.clone();
        bad.package.size_bytes += 1;
        assert_eq!(
            prepare_package(&archive, &bad, &staging, &PackageLimits::default())
                .unwrap_err()
                .code,
            "PACKAGE_INTEGRITY"
        );
        for limits in [
            PackageLimits {
                archive_bytes: 1,
                ..Default::default()
            },
            PackageLimits {
                expanded_bytes: 1024,
                ..Default::default()
            },
        ] {
            assert_eq!(
                prepare_package(&archive, &good, &staging, &limits)
                    .unwrap_err()
                    .code,
                "PACKAGE_LIMIT"
            );
        }
        assert_eq!(
            prepare_package_cancellable(
                &archive,
                &good,
                &staging,
                &PackageLimits::default(),
                &AtomicBool::new(true)
            )
            .unwrap_err()
            .code,
            "CANCELLED"
        );
        assert!(!staging.exists());
        std::fs::create_dir(&staging).unwrap();
        std::fs::write(staging.join("keep"), b"keep").unwrap();
        assert!(prepare_package(&archive, &good, &staging, &PackageLimits::default()).is_err());
        assert_eq!(std::fs::read(staging.join("keep")).unwrap(), b"keep");
        std::fs::remove_file(staging.join("keep")).unwrap();
        std::fs::remove_dir(&staging).unwrap();
    }
}

#[test]
fn both_formats_reject_entry_depth_limits_and_directory_links() {
    let root = tempfile::tempdir().unwrap();
    let entries: &[(&str, &[u8])] = &[("SKILL.md", b"# Safe"), ("deep/path/file", b"resource")];
    for (format, bytes) in [
        ("zip", zip_bytes(entries)),
        ("7z", seven_zip_bytes(entries)),
    ] {
        let archive = write_archive(root.path(), &bytes);
        for limits in [
            PackageLimits {
                entries: 1,
                ..Default::default()
            },
            PackageLimits {
                depth: 2,
                ..Default::default()
            },
        ] {
            assert_eq!(
                prepare_package(
                    &archive,
                    &release(&bytes, format),
                    &root.path().join("verified"),
                    &limits
                )
                .unwrap_err()
                .code,
                "PACKAGE_LIMIT"
            );
        }
    }
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file("SKILL.md", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"# Safe").unwrap();
    zip.add_symlink(
        "linked",
        "../external",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    assert_refused(
        &zip.finish().unwrap().into_inner(),
        "zip",
        Some("PACKAGE_INVALID"),
    );
    for attributes in [0x400u32, 0o120777u32 << 16] {
        let mut writer = sevenz_turbo::ArchiveWriter::new(Cursor::new(Vec::new())).unwrap();
        writer
            .push_archive_entry(
                sevenz_turbo::ArchiveEntry::new_file("SKILL.md"),
                Some(Cursor::new(b"# Safe")),
            )
            .unwrap();
        let mut link = sevenz_turbo::ArchiveEntry::new_file("linked");
        link.has_windows_attributes = true;
        link.windows_attributes = attributes;
        writer
            .push_archive_entry(link, Some(Cursor::new(b"../external")))
            .unwrap();
        assert_refused(
            &writer.finish().unwrap().into_inner(),
            "7z",
            Some("PACKAGE_INVALID"),
        );
    }
}

#[test]
fn bounded_zip64_footer_is_supported_without_trusting_its_entry_count() {
    let bytes = zip_bytes(&[("SKILL.md", b"# Small ZIP64")]);
    let at = bytes.len() - 22;
    let footer = &bytes[at..];
    let mut zip64 = bytes[..at].to_vec();
    zip64.extend_from_slice(b"PK\x06\x06");
    zip64.extend_from_slice(&44u64.to_le_bytes());
    zip64.extend_from_slice(&45u16.to_le_bytes());
    zip64.extend_from_slice(&45u16.to_le_bytes());
    zip64.extend_from_slice(&0u32.to_le_bytes());
    zip64.extend_from_slice(&0u32.to_le_bytes());
    zip64.extend_from_slice(&1u64.to_le_bytes());
    zip64.extend_from_slice(&1u64.to_le_bytes());
    zip64.extend_from_slice(
        &(u32::from_le_bytes(footer[12..16].try_into().unwrap()) as u64).to_le_bytes(),
    );
    zip64.extend_from_slice(
        &(u32::from_le_bytes(footer[16..20].try_into().unwrap()) as u64).to_le_bytes(),
    );
    zip64.extend_from_slice(b"PK\x06\x07");
    zip64.extend_from_slice(&0u32.to_le_bytes());
    zip64.extend_from_slice(&(at as u64).to_le_bytes());
    zip64.extend_from_slice(&1u32.to_le_bytes());
    let mut footer = footer.to_vec();
    footer[8..12].fill(255);
    footer[12..20].fill(255);
    zip64.extend_from_slice(&footer);
    let root = tempfile::tempdir().unwrap();
    let archive = write_archive(root.path(), &zip64);
    let artifact = prepare_package(
        &archive,
        &release(&zip64, "zip"),
        &root.path().join("verified"),
        &PackageLimits::default(),
    )
    .unwrap();
    assert_eq!(
        std::fs::read(artifact.root.join("SKILL.md")).unwrap(),
        b"# Small ZIP64"
    );
    // ZIP64 may retain the real classic footer fields instead of max-value sentinels.
    let mut classic = zip64.clone();
    let classic_at = classic.len() - 22;
    classic[classic_at..].copy_from_slice(&bytes[at..]);
    for disk_offset in [at + 16, at + 56 + 16] {
        let mut split = classic.clone();
        split[disk_offset] = 2;
        assert_refused(&split, "zip", Some("PACKAGE_INVALID"));
    }
    zip64[at + 32..at + 40].copy_from_slice(&u64::MAX.to_le_bytes());
    assert_refused(&zip64, "zip", None);
}

#[tokio::test]
async fn package_download_streams_more_than_the_json_limit_with_auth_and_keeps_the_store_context_path(
) {
    use super::support::{Response, TestServer};
    use crate::sds::{commands::SdsServices, contracts::SessionStatus};
    use std::sync::{atomic::AtomicBool, Arc, Mutex};
    let bytes: Vec<u8> = (0..3 * 1024 * 1024).map(|i| (i % 251) as u8).collect();
    let expected = bytes.clone();
    let observed = Arc::new(Mutex::new(Vec::new()));
    let requests = observed.clone();
    let server = TestServer::start(move |request| {
        let bytes = bytes.clone();
        let requests = requests.clone();
        async move {
            if request.path.ends_with("/auth/login") {
                return Response::json(
                    200,
                    serde_json::from_str(include_str!(
                        "../../../../contracts/sds-desktop/v1/fixtures/auth-login.json"
                    ))
                    .unwrap(),
                );
            }
            requests
                .lock()
                .unwrap()
                .push((request.path, request.authorization));
            Response::binary(bytes)
        }
    })
    .await;
    let root = tempfile::tempdir().unwrap();
    let services = SdsServices::new_fixture(root.path(), &server.base_url).unwrap();
    services
        .session
        .login("employee@example.invalid", "synthetic-password")
        .await
        .unwrap();
    let dest = root.path().join("download.zip");
    services
        .store
        .download(&release(&expected, "zip"), &dest, &AtomicBool::new(false))
        .await
        .unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), expected);
    let requests = observed.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].0,
        "/skm/api/v1/desktop/skills/skill-123/releases/rel-003/package"
    );
    assert_eq!(
        requests[0].1.as_deref(),
        Some("Bearer synthetic-access-one")
    );
    drop(requests);
    assert_eq!(
        services.session.status().await.unwrap().status,
        SessionStatus::Authenticated
    );
}
