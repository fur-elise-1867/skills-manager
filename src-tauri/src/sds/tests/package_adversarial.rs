use super::*;
use crate::sds::commands::SdsServices;
use crate::sds::tests::support::{Response, TestServer};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};

fn auth_fixture(refresh: bool) -> serde_json::Value {
    serde_json::from_str(if refresh {
        include_str!("../../../../contracts/sds-desktop/v1/fixtures/auth-refresh.json")
    } else {
        include_str!("../../../../contracts/sds-desktop/v1/fixtures/auth-login.json")
    })
    .unwrap()
}

#[test]
fn zip_output_limit_uses_actual_expansion_even_when_the_header_lies() {
    let mut bytes = zip_bytes(&[("SKILL.md", &[b'A'; 4096])]);
    let central = bytes.windows(4).position(|p| p == b"PK\x01\x02").unwrap();
    bytes[22..26].copy_from_slice(&1u32.to_le_bytes());
    bytes[central + 24..central + 28].copy_from_slice(&1u32.to_le_bytes());
    let root = tempfile::tempdir().unwrap();
    let archive = write_archive(root.path(), &bytes);
    let result = prepare_package(
        &archive,
        &release(&bytes, "zip"),
        &root.path().join("verified"),
        &PackageLimits {
            expanded_bytes: 1024,
            ..Default::default()
        },
    );
    assert_eq!(result.unwrap_err().code, "PACKAGE_LIMIT");
    assert!(!root.path().join("verified").exists());
}

#[test]
fn zip_refuses_unbudgeted_codecs_and_caps_actual_central_directory_metadata() {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file(
            "SKILL.md",
            zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Xz),
        )
        .unwrap();
    writer
        .write_all(b"# Benign XZ fixture proves codec refusal before allocating a decoder")
        .unwrap();
    assert_refused(
        &writer.finish().unwrap().into_inner(),
        "zip",
        Some("PACKAGE_INVALID"),
    );
    let root = tempfile::tempdir().unwrap();
    let mut bytes = zip_bytes(&[("SKILL.md", b"# Safe")]);
    let archive = write_archive(root.path(), &bytes);
    assert_eq!(
        prepare_package(
            &archive,
            &release(&bytes, "zip"),
            &root.path().join("verified"),
            &PackageLimits {
                header_bytes: 1,
                ..Default::default()
            }
        )
        .unwrap_err()
        .code,
        "PACKAGE_LIMIT"
    );
    // A filename alone already exceeds 46 bytes; a false directory length cannot hide that.
    let footer = bytes.len() - 22;
    bytes[footer + 12..footer + 16].copy_from_slice(&46u32.to_le_bytes());
    let archive = write_archive(root.path(), &bytes);
    assert!(prepare_package(
        &archive,
        &release(&bytes, "zip"),
        &root.path().join("verified"),
        &PackageLimits {
            header_bytes: 48,
            ..Default::default()
        }
    )
    .is_err());
}

#[test]
fn zip_rejects_a_forged_final_directory_without_falling_back_or_scanning_binary_resources() {
    let mut bytes = zip_bytes(&[("SKILL.md", b"# Safe"), ("resource", b"keep")]);
    let mut forged = [0u8; 22];
    forged[..4].copy_from_slice(b"PK\x05\x06");
    forged[8..10].copy_from_slice(&1u16.to_le_bytes());
    forged[10..12].copy_from_slice(&1u16.to_le_bytes());
    forged[12..16].copy_from_slice(&46u32.to_le_bytes());
    bytes.extend_from_slice(&forged);
    assert_refused(&bytes, "zip", Some("PACKAGE_INVALID"));
    let root = tempfile::tempdir().unwrap();
    let nested = zip_bytes(&[("inside.txt", b"not recursively extracted")]);
    let payload = b"arbitrary binary PK\x05\x06\x00\x00 PK\x06\x06 bytes";
    let bytes = zip_bytes(&[
        ("SKILL.md", b"# Safe"),
        ("resources/nested.zip", &nested),
        ("resources/data.bin", payload),
    ]);
    let archive = write_archive(root.path(), &bytes);
    let verified = prepare_package(
        &archive,
        &release(&bytes, "zip"),
        &root.path().join("verified"),
        &PackageLimits::default(),
    )
    .unwrap();
    assert_eq!(
        std::fs::read(verified.root.join("resources/nested.zip")).unwrap(),
        nested
    );
    assert_eq!(
        std::fs::read(verified.root.join("resources/data.bin")).unwrap(),
        payload
    );
}

#[test]
fn zip_data_descriptors_are_checked_and_multipart_flags_are_refused() {
    let mut output = Vec::new();
    let mut writer = rawzip::ZipArchiveWriter::new(&mut output);
    let (mut entry, config) = writer
        .new_file("SKILL.md")
        .compression_method(rawzip::CompressionMethod::DEFLATE)
        .start()
        .unwrap();
    let encoder = flate2::write::DeflateEncoder::new(&mut entry, flate2::Compression::default());
    let mut wrapped = config.wrap(encoder);
    wrapped.write_all(b"# Descriptor package").unwrap();
    let (encoder, descriptor) = wrapped.finish().unwrap();
    encoder.finish().unwrap();
    entry.finish(descriptor).unwrap();
    writer.finish().unwrap();
    let root = tempfile::tempdir().unwrap();
    let archive = write_archive(root.path(), &output);
    let artifact = prepare_package(
        &archive,
        &release(&output, "zip"),
        &root.path().join("verified"),
        &PackageLimits::default(),
    )
    .unwrap();
    assert_eq!(
        std::fs::read(artifact.root.join("SKILL.md")).unwrap(),
        b"# Descriptor package"
    );
    let descriptor = output.windows(4).position(|v| v == b"PK\x07\x08").unwrap();
    output[descriptor + 4] ^= 255;
    assert_refused(&output, "zip", Some("PACKAGE_INVALID"));
    let original = zip_bytes(&[("SKILL.md", b"# Safe")]);
    let central = original
        .windows(4)
        .position(|v| v == b"PK\x01\x02")
        .unwrap();
    for offset in [original.len() - 22 + 4, central + 34] {
        let mut bytes = original.clone();
        bytes[offset] = 1;
        assert_refused(&bytes, "zip", Some("PACKAGE_INVALID"));
    }
    let mut encrypted = original;
    encrypted[6] |= 1;
    encrypted[central + 8] |= 1;
    assert_refused(&encrypted, "zip", Some("PACKAGE_INVALID"));
}

#[test]
fn checked_in_contract_archives_match_receipts_and_have_identical_verified_trees() {
    let receipts: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../contracts/sds-desktop/v1/fixtures/packages/receipts.json"
    ))
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    let mut manifests = Vec::new();
    for (format, bytes) in [
        (
            "zip",
            include_bytes!(
                "../../../../contracts/sds-desktop/v1/fixtures/packages/sds-synthetic.zip"
            )
            .as_slice(),
        ),
        (
            "7z",
            include_bytes!(
                "../../../../contracts/sds-desktop/v1/fixtures/packages/sds-synthetic.7z"
            )
            .as_slice(),
        ),
    ] {
        let receipt = receipts
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["format"] == format)
            .unwrap();
        assert_eq!(receipt["sizeBytes"].as_u64().unwrap(), bytes.len() as u64);
        assert_eq!(
            receipt["sha256"].as_str().unwrap(),
            hex::encode(Sha256::digest(bytes))
        );
        let archive = write_archive(root.path(), bytes);
        let verified = prepare_package(
            &archive,
            &release(bytes, format),
            &root.path().join(format),
            &PackageLimits::default(),
        )
        .unwrap();
        assert_eq!(
            std::fs::read(verified.root.join("resources/data.bin")).unwrap(),
            [0, 255, 17, 0]
        );
        manifests.push((verified.manifest, verified.fingerprint));
    }
    assert_eq!(manifests[0], manifests[1]);
}

#[test]
fn encrypted_split_truncated_and_excessive_header_or_decoder_archives_are_refused() {
    use sevenz_turbo::{
        encoder_options::{AesEncoderOptions, Lzma2Options},
        ArchiveEntry, ArchiveWriter, Password,
    };
    for encrypted_header in [false, true] {
        let mut writer = ArchiveWriter::new(Cursor::new(Vec::new())).unwrap();
        writer.set_encrypt_header(encrypted_header);
        writer.set_content_methods(vec![
            AesEncoderOptions::new(Password::new("synthetic-fixture-only"))
                .with_num_cycles_power(1)
                .into(),
            Lzma2Options::from_level(0).into(),
        ]);
        writer
            .push_archive_entry(
                ArchiveEntry::new_file("SKILL.md"),
                Some(Cursor::new(b"# Encrypted")),
            )
            .unwrap();
        assert_refused(
            &writer.finish().unwrap().into_inner(),
            "7z",
            Some("PACKAGE_INVALID"),
        );
    }
    let root = tempfile::tempdir().unwrap();
    let bytes = seven_zip_bytes(&[("SKILL.md", &[b'A'; 4096])]);
    let archive = write_archive(root.path(), &bytes);
    for limits in [
        PackageLimits {
            decoder_memory_bytes: 1,
            ..Default::default()
        },
        PackageLimits {
            header_bytes: 1,
            ..Default::default()
        },
    ] {
        assert_eq!(
            prepare_package(
                &archive,
                &release(&bytes, "7z"),
                &root.path().join("verified"),
                &limits
            )
            .unwrap_err()
            .code,
            "PACKAGE_LIMIT"
        );
    }
    let mut split = release(&bytes, "7z");
    split.package.file_name.push_str(".001");
    assert_eq!(
        prepare_package(
            &archive,
            &split,
            &root.path().join("verified"),
            &PackageLimits::default()
        )
        .unwrap_err()
        .code,
        "PACKAGE_INVALID"
    );
    assert_refused(&bytes[..bytes.len() / 2], "7z", None);
    let zip = zip_bytes(&[("SKILL.md", b"# Safe")]);
    assert_refused(&zip[..zip.len() / 2], "zip", None);
    let mut duplicate = zip_bytes(&[("SKILL.md", b"# Safe"), ("OTHER.md", b"# Duplicate")]);
    for at in (0..duplicate.len() - 8)
        .filter(|&at| &duplicate[at..at + 8] == b"OTHER.md")
        .collect::<Vec<_>>()
    {
        duplicate[at..at + 8].copy_from_slice(b"SKILL.md");
    }
    assert_refused(&duplicate, "zip", Some("PACKAGE_INVALID"));
}

#[tokio::test]
async fn downloads_refresh_once_but_never_retry_forbidden_redirects_or_bad_chunked_payloads() {
    for (status, code, refresh_count) in [
        (401, None, 1),
        (403, Some("ACCESS_DENIED"), 0),
        (302, Some("INVALID_RESPONSE"), 0),
        (200, Some("PACKAGE_INTEGRITY"), 0),
    ] {
        let refreshes = Arc::new(AtomicUsize::new(0));
        let seen = refreshes.clone();
        let bytes = vec![37u8; 128 * 1024];
        let expected = bytes.clone();
        let server = TestServer::start(move |request| {
            let seen = seen.clone();
            let bytes = bytes.clone();
            async move {
                if request.path.ends_with("/auth/login") {
                    return Response::json(200, auth_fixture(false));
                }
                if request.path.ends_with("/auth/refresh") {
                    seen.fetch_add(1, Ordering::SeqCst);
                    return Response::json(200, auth_fixture(true));
                }
                if status == 401
                    && request.authorization.as_deref() != Some("Bearer synthetic-access-two")
                {
                    return Response::json(401, serde_json::Value::Null);
                }
                if status == 403 {
                    return Response::json(403, serde_json::Value::Null);
                }
                if status == 302 {
                    return Response::redirect("http://127.0.0.1:9/must-not-follow".into());
                }
                let mut response = Response::binary(bytes);
                response.chunked = true;
                response
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
        let mut release = release(&expected, "zip");
        if status == 200 {
            release.package.size_bytes = 1;
        }
        let dest = root.path().join("download.zip");
        let result = services
            .store
            .download(&release, &dest, &AtomicBool::new(false))
            .await;
        if let Some(code) = code {
            assert_eq!(result.unwrap_err().code, code);
            assert!(!dest.exists());
        } else {
            result.unwrap();
            assert_eq!(std::fs::read(&dest).unwrap(), expected);
        }
        assert_eq!(refreshes.load(Ordering::SeqCst), refresh_count);
        assert!(!std::fs::read_dir(root.path()).unwrap().any(|e| e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".sds-download-")));
    }
}

#[tokio::test]
async fn a_cancelled_or_old_account_download_never_publishes_the_archive() {
    for logout in [false, true] {
        let entered = Arc::new(tokio::sync::Notify::new());
        let observed = entered.clone();
        let bytes = vec![37u8; 2 * 1024 * 1024];
        let expected = bytes.clone();
        let server = TestServer::start(move |request| {
            let observed = observed.clone();
            let bytes = bytes.clone();
            async move {
                if request.path.ends_with("/auth/login") {
                    return Response::json(200, auth_fixture(false));
                }
                if request.path.ends_with("/auth/logout") {
                    return Response::json(200, serde_json::json!({"success":true}));
                }
                observed.notify_one();
                let mut response = Response::binary(bytes);
                response.chunk_delay_ms = 10;
                response.chunked = true;
                response
            }
        })
        .await;
        let root = tempfile::tempdir().unwrap();
        let services = Arc::new(SdsServices::new_fixture(root.path(), &server.base_url).unwrap());
        services
            .session
            .login("employee@example.invalid", "synthetic-password")
            .await
            .unwrap();
        let services_job = services.clone();
        let dest = root.path().join("download.zip");
        let job_dest = dest.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        let job_cancel = cancel.clone();
        let job = tokio::spawn(async move {
            services_job
                .store
                .download(&release(&expected, "zip"), &job_dest, &job_cancel)
                .await
        });
        tokio::time::timeout(std::time::Duration::from_secs(2), entered.notified())
            .await
            .unwrap();
        if logout {
            services.session.logout().await.unwrap();
        } else {
            cancel.store(true, Ordering::Release);
        }
        let result = tokio::time::timeout(std::time::Duration::from_secs(2), job)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            result.unwrap_err().code,
            if logout {
                "REVIEW_REQUIRED"
            } else {
                "CANCELLED"
            }
        );
        assert!(!dest.exists());
        assert!(!std::fs::read_dir(root.path()).unwrap().any(|e| e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".sds-download-")));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn publishing_a_frozen_download_is_serialized_against_logout_and_refuses_an_old_context() {
    let server = TestServer::start(|request| async move {
        Response::json(
            200,
            if request.path.ends_with("/auth/login") {
                auth_fixture(false)
            } else {
                serde_json::json!({"success":true})
            },
        )
    })
    .await;
    let root = tempfile::tempdir().unwrap();
    let services = Arc::new(SdsServices::new_fixture(root.path(), &server.base_url).unwrap());
    services
        .session
        .login("employee@example.invalid", "synthetic-password")
        .await
        .unwrap();
    let (_, context) = services.session.scope().await.unwrap();
    let staged = root.path().join("staged");
    let published = root.path().join("published");
    std::fs::write(&staged, b"verified bytes").unwrap();
    let entered = Arc::new(tokio::sync::Notify::new());
    let entered_job = entered.clone();
    let (release, wait) = std::sync::mpsc::sync_channel(1);
    let service_job = services.clone();
    let frozen = context.clone();
    let job_path = published.clone();
    let publishing = tokio::spawn(async move {
        service_job
            .session
            .with_current_context(&frozen, move || {
                entered_job.notify_one();
                wait.recv().unwrap();
                std::fs::rename(staged, job_path).unwrap();
                Ok(())
            })
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(2), entered.notified())
        .await
        .unwrap();
    let service_logout = services.clone();
    let logout = tokio::spawn(async move { service_logout.session.logout().await });
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    assert!(
        !logout.is_finished(),
        "logout cannot invalidate a context inside its commit boundary"
    );
    release.send(()).unwrap();
    publishing.await.unwrap().unwrap();
    logout.await.unwrap().unwrap();
    assert_eq!(std::fs::read(published).unwrap(), b"verified bytes");
    let called = AtomicBool::new(false);
    let result = services
        .session
        .with_current_context(&context, || {
            called.store(true, Ordering::Release);
            Ok(())
        })
        .await;
    assert_eq!(result.unwrap_err().code, "REVIEW_REQUIRED");
    assert!(!called.load(Ordering::Acquire));
}
