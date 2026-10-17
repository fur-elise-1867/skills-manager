# Phase 1 — local Library slice (A08 in progress)

Date: 2026-10-17, following A07 source commit `b8f9810`.

Library now has real SDS IPC and a local React page, available without a Store connection or signed-in session. It displays cached release identity/version and each recorded target path, independent cache/deployment status, and MCP observations explicitly labelled as installation history. Local search, refresh, errors/retry and a read-only skill document preview are implemented. Remove/install callbacks are not supplied by the shell yet, so this slice exposes no unfinished mutation action.

The Rust reader reopens local metadata, checks cache contents and compares each deployment's parent identity, target identity and content fingerprint. Changes include file bytes, added/deleted files and empty directories. Missing directories and unverifiable/replaced/junction destinations remain visible as distinct states. A changed cache does not erase or mark an independent, unchanged installation as modified. Historical deployment paths receive lexical validation when loaded; every filesystem observation/action still uses live path validation and ancestor pins.

Preview accepts only an opaque, validated Library release key. It reads only the verified release root's `SKILL.md`, limits text to 1 MiB, verifies expected content digest and UTF-8, and returns explicit errors for unsafe/unavailable content. Markdown renders without raw HTML, remote images or clickable external links. No frontend filesystem path is accepted.

Observed API/UI red runs preceded the implementation. Current source checks: Rust SDS **97 passed, 2 ignored**, UI **26 passed**, Node launcher/fixture/bundle **10 passed**, lint (96 UI files), SDS build and SDS production check exit 0. Logs: `a08-library-final-rust.log`, `a08-library-all-ui.log`, `a08-library-node.log`, `a08-library-lint.log`, `a08-library-build.log`, `a08-library-check.log`. The two ignored tests retain the A07 meanings: opt-in real vault, and child helper invoked by the passing process-kill parent test.

Default-feature `cargo check --manifest-path src-tauri/Cargo.toml` also exits 0 (`a08-library-upstream-check.log`), checking shared DTO/command compilation without enabling SDS storage modules. This is not a rerun or pass of the known-failing full upstream test baseline.

New Library tests close/reopen the repository/service, sign out and stop synthetic HTTP before reading local state; check empty-directory edits, missing/identically replaced targets, a native Windows junction, and cache corruption versus a healthy target. Frontend tests check local-only navigation without Store setup, search, safe preview and explicit read failures instead of false empty results. These are source/seam tests, not native W07 or real AIPro acceptance.

A08 is not complete. Scoped removal, its durable quarantine/recovery and review UI are still next. A09 must connect real background install/remove execution and inspect the integrated native flow. No native app was launched, no real agent folder was changed and no Store service/database was changed for this slice.
