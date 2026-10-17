# Phase 1 — recoverable install operations (A07)

Date: 2026-10-17. A07 backend source and seam tests are implemented and independently reviewed. This is not real Store, native installation-flow or AIPro acceptance; those remain A09/S01–S04/A10 gates.

## Implementation

- `OperationService` prepares from authenticated Store detail/release endpoints, not frontend metadata or paths. It freezes account/profile/generation, runtime epoch, release, MCP report, exact target and the live identity of the existing skills root or its existing parent. Preparation does not create the skills folder. Execution rechecks permission and immutable release metadata even on cache hits. Changed MCP observations require a new review/operation ID.
- The immutable cache is scoped by hashed Store profile + skill ID + release ID. Copying and hashing happen before publication; the cache process lock and ancestor pins remain owned until the short rename/SQLite publication. Publication checks both frozen session context and cancellation. Logout/cancellation before that boundary creates no verified cache row. A cache already validly published is retained if a later target stage fails or is cancelled; it is distinct from an installed deployment.
- Windows directory handles pin ancestors against replacement. A fresh stage is created on the same volume in a sibling `.sds-skills-manager-staging/<operation UUID>` namespace outside the configured discovery root. An alternate sibling name handles an override that itself uses the reserved name, including case variants. No archive script is run, no existing destination is overwritten, and retained recovery evidence is outside the configured skills root.
- SQLite records the intent before filesystem rename. The intent contains root/stage volume and 128-bit file identities, an immutable release key and content fingerprint; stage paths are derived rather than loaded as arbitrary raw paths. Rename uses the owned source handle with replacement disabled. Deployment insertion and the completed result share one SQLite transaction. SQLite write failure after rename retains the intent and never reports completed.
- Commit holds the target/process lock, session context and target-settings gate until bounded content verification, rename and SQLite completion finish. Hashing runs on a blocking worker; archive extraction/copying happen before this section. Cancel before commit preserves the destination; a cancel request once commit has entered does not interrupt its filesystem/DB completion. Generic result updates and intent failure cannot overwrite terminal success.
- Recovery runs through `SdsServices` before the coordinator accepts mutations. It approves roots by live identity, reconciles an owned renamed tree, or fails an uncommitted stage while preserving its evidence. Unexpected identity/content/path states remain `RecoveryRequired`; user files are never recursively removed by recovery. A per-repository process lock prevents a second coordinator from reconciling an active process's work. Old precommit records expire and require review after restart.
- Installing an unchanged, exact managed release confirms the existing deployment without adding another deployment row or replacing files. Unmanaged contents, edits and other releases are conflicts. A missing managed destination is also a pilot conflict, including two plans staged before the first deployment appears; final commit rechecks tracking under the retained lock.
- The dedicated `pilot/pilot.sqlite3` belongs to SDS; upstream databases are not opened or migrated. The main DB and ancestors remain pinned for the connection lifetime. Existing main/sidecar reparse points and hard links are refused before SQLite writes; stored JSON reads have byte bounds. This is not a claim of a custom SQLite VFS resistant to arbitrary concurrent mutation by another process with the same user's filesystem privileges.

Windows identity uses the documented [volume and 128-bit file ID](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_id_info). A trial retaining child handles caused actual Windows error 5 during directory rename and was removed; Microsoft's [rename rules](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_rename_information) describe that restriction. Current code checks fingerprints immediately before and after rename instead. Process-interruption tests below are not physical power-loss or storage-controller durability tests.

## Verification

Observed red/green development included missing APIs, ancestor creation, changed reviewed roots, changed cached release metadata, Unicode root-name collisions, oversized intents, terminal-result overwrite, main/sidecar hard links, cache publication after logout/cancel, missing managed destinations, the two-stage race and reserved staging-name collision. The initial independent review's two Important findings were corrected and checked again; final independent review found no remaining Important finding within A07 source scope.

Latest checks:

| Check | Result |
| --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --features sds --lib sds::tests` | 94 passed, 0 failed, 2 ignored; `a07-reviewed-final-rust.log` |
| `npm run sds:check` | exit 0; `a07-reviewed-final-check.log` |
| `npm run test:sds` | 22 passed; `a07-final-ui.log` |
| `npm run lint` | exit 0, UI spec 94 files; `a07-final-lint.log` |
| `npm run build:sds` | exit 0; `a07-final-build.log` |
| Node launcher/fixture/bundle checks | 10 passed; `a07-final-node.log` |

The ignored tests are the separately opt-in real Windows credential test and an owned crash-child helper. The parent crash test actually launches and kills that helper at three boundaries: before rename, after rename, and after SQLite commit. Before rename, recovery keeps the stage and records failure; after rename it records exactly one completed deployment; after DB commit it finds no pending intent. A second process cannot enter the same target commit while the child holds its lock. An injected SQLite insert failure separately verifies reconciliation after a filesystem rename.

All new operation/package/cache/deployment tests use temporary roots and synthetic HTTP/archives. No native application was launched, no real AIPro folder was written, and no live Store or Store database was changed for these tests. Earlier full-upstream baseline failures (Windows symlink permission and local Git timeout) are unchanged; this is not a full-upstream pass.

## Remaining delivery work

A08 must add Library deployment status and scoped remove. A09 must expose background execution/status/events through real IPC and complete native Settings/dialog/installation-flow inspection. S01–S04 must integrate the additive real Store provider. A10 must build a fresh installer from the final source, exercise native error/cancel/restart flows and prove AIPro actually recognizes the skill. The user will prepare AIPro on this machine before that acceptance. The installed earlier release artifact predates A07 and is not evidence for these features.
