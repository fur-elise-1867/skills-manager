# Phase 1 — AIPro destination and MCP inspection

Date: 2026-10-17. Isolated worktree, synthetic temporary roots only.

## Implemented behavior

- Resolve AIPro Global as the user's `.aipro/skills`; do not create the real agent directory. An absent `.aipro` reports `TARGET_NOT_READY`.
- Settings selects an existing local directory. Rust validates and atomically saves the override beneath the SDS data root. Invalid selections leave the saved choice intact; resetting restores default resolution. IPC exposes settings, not an executable destination supplied by the frontend.
- Reject relative/UNC/verbatim paths, alternate streams, reserved names, dot aliases, trailing dots/spaces, and reparse roots/ancestors. Real Windows junction tests verify rejection and preservation of external sentinel files. This is a preflight snapshot; deployment still needs locked revalidation in A07.
- MCP reads only the global `.aipro/mcp.json`, including when a custom skills destination is selected. Reads are bounded to 1 MiB, refuse reparse files/ancestors, and export no configuration values. Exact keys with nonempty objects indicate configuration presence, never operational health. Undeclared requirements remain unknown; an explicit empty list is not required. Each check rereads the file.
- Explicit debug fixtures own `agent-home/.aipro` inside their isolated data root and reject override changes. They never use the real user's AIPro directory.
- Settings and the existing installation review component display the resolved path and MCP warning. Installation review awaits operation service integration in A07/A09.

## Verification

Observed failing tests before implementation: missing resolver/reader/service APIs; an accepted junction; MCP object interpretation; missing target Settings component and shell integration. Settings persistence and service tests were then green.

Final focused verification:

- `cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --features sds --lib sds::tests`: 42 passed, 1 ignored opt-in Windows credential integration test, 0 failures. Upstream unused-import warnings remain.
- `npm run test:sds`: 22 passed across 7 files.
- `npm run build:sds`: exit 0.
- `npm run lint`: exit 0; UI specification check 94 files, translation checks passed.
- `npm run sds:check`: exit 0.
- `node --test scripts/sds.test.mjs scripts/sds-fixtures.test.mjs scripts/sds-bundle.test.mjs`: 10 passed. The checker previously mistook the suffix of `sds_get_target_settings` for `get_settings`; the regression distinguishes complete quoted command names and still rejects actual upstream command literals.

Full logs remain in the ignored execution workspace/root `a05-*.log` files. The fixture credential test's former empty-root assertion now permits only its owned `agent-home/.aipro` tree and still verifies no credential files were persisted.

## Outstanding acceptance

The user stopped Computer Use with Escape before this slice's native UI inspection. Settings and minimum-window dialog keyboard/layout checks remain unverified and must run during A09 native integration. A05 is not marked fully accepted on component tests alone.

The user will prepare AIPro on this machine before A10. Actual AIPro skill recognition remains a real acceptance gate. No AIPro installation or real agent configuration mutation was performed by this slice.
