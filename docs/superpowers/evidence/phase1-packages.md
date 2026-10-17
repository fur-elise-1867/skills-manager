# Phase 1 — package preparation and protected download

Date: 2026-10-17. A06 implementation checkpoint; no real agent target has been changed.

## Behavior and limits

Package preparation first copies a bounded archive to an owned stable temporary file while computing SHA-256. Exact size/digest must match the release before extraction. Readers extract only into a private temporary tree; a verified root is published to a previously absent staging path after all validation. Failure and cancellation remove the owned temporary tree and never modify an existing destination.

ZIP and 7z preserve binary and Unicode resource bytes. Exactly one `SKILL.md` must be at the root or inside one enclosing directory. Unrelated files, multiple/nested-only roots, traversal, backslashes, absolute/drive/UNC paths, Windows device names, alternate streams, trailing dots/spaces, duplicate/case/Unicode-equivalent paths, file-parent collisions, symlinks/reparse attributes, encrypted and split archives are refused. Nested archives are ordinary resources and are never recursively extracted. No package code is executed.

Pilot defaults are 100 MiB archive, 500 MiB expanded content, 10,000 entries, and 32 path levels. Limits apply to actual decompressed bytes. Header metadata is capped at 8 MiB; 7z decoder memory at 256 MiB. These are product limits proposed in the plan, not measurements of production Store packages.

Protected downloads stream through Rust, preserve the Store application path (including `/skm`), use the existing authenticated session/401 single-flight refresh boundary, and refuse redirects. Credentials never pass through frontend IPC. A complete downloaded archive is published under a short session context guard so logout/profile changes cannot invalidate it between the final context check and publication. Cancellation, bad size/digest, network failures and stale contexts leave no completed archive.

## Reader selection

The cached source and `cargo info` metadata were inspected for sevenz-rust 0.6.1, sevenz-rust2 0.24.0 and sevenz-turbo 0.29.0. The first two do not expose the required combined header/decoder allocation bounds at reader construction. SDS pins `sevenz-turbo = 0.29.0` with default features disabled and `ArchiveLimits` supplied before parsing or decoding. See the [reader source](https://docs.rs/sevenz-turbo/0.29.0/sevenz_turbo/struct.ArchiveReader.html) and checked-in lockfile. Compression/AES dependencies are for synthetic adversarial fixtures and the example generator; password-protected archives are unsupported by the desktop.

SDS ZIP metadata uses `rawzip = 0.5.2`, a file-backed streaming iterator with fixed buffers. It checks declared and actual entry counts, actual record bytes, multipart fields, local/central metadata, compressed data ranges, CRC and data descriptors. This avoids zip 2.4.2's allocation from untrusted entry counts and fallback to earlier EOCD records. No payload bytes are scanned for EOCD signatures, so nested archive resources remain intact. The existing upstream zip dependency/features remain unchanged. SDS explicitly supports Store/Deflate/Deflate64/Bzip2 methods with bounded codec state; other ZIP codecs need repacking. [rawzip reader documentation](https://docs.rs/rawzip/0.5.2/rawzip/struct.ZipArchive.html).

**SDS build toolchain minimum: Rust 1.97.1** (sevenz-turbo requirement), verified against installed Rust 1.99. The upstream package's pre-existing `rust-version = 1.77.2` is preserved; this does not claim SDS compiles on that version.

## Review and verification record

Independent read-only A06 review found two Important issues initially: ZIP could instantiate an unbudgeted XZ/LZMA decoder, and downloaded files could be published in the gap after the last session-generation check. A metadata-budget finding was subsequently promoted to Important because zip 2.4.2 can fall back to an earlier directory and allocate its count before the post-construction check. The implementation now uses explicit bounded codecs/streaming ZIP metadata and a context-guarded publication boundary.

Regression evidence includes an observed successful XZ extraction before the codec restriction (red test), and a missing publication guard API followed by a passing test that blocks logout while publication runs and refuses an old context. The original ZIP64 case failed before ZIP64 support and then passed. Direct/one-wrapper ZIP and 7z trees produce identical manifests/fingerprints. Checked-in synthetic archives and their independent PowerShell SHA-256 results match `contracts/sds-desktop/v1/fixtures/packages/receipts.json`.

The narrow follow-up review identified a ZIP64 multipart policy bypass when optional ZIP64 records accompany non-sentinel classic footer fields. The added regression first accepted that archive (red); the reader now probes the ZIP64 locator unconditionally, checks consistency and rejects split disk fields.

Final verification: `cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --features sds --lib sds::tests` passed 59 tests, with one opt-in Windows credential integration test ignored (log `a06-final-rust.log`). `npm run sds:check`, `npm run build:sds`, and `npm run lint` exited 0. UI tests passed 22 cases; launcher/fixture/bundle Node tests passed 10. Fixture generation completed through the Rust example. Existing upstream warnings and the previously documented broad-suite baseline limitations are not erased by these focused results.

This evidence does not constitute real Store or AIPro acceptance; journal/deployment/recovery and native operation flow remain A07–A10 work.
