# Phase 2 integration checkpoint

Verified on 2026-10-18, Asia/Saigon, against the current working source in `.worktrees/sds-phase0-1`, baseline `be1cd17`. This is a checkpoint; G2 remains open pending the remaining acceptance work.

## Fresh executable checks

| Command | Actual result |
|---|---|
| `npm run test:sds` | 57 passed, 18 files, zero failed after snapshot-list corruption regression |
| `cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --features sds --lib sds::` | 150 passed, zero failed, 7 ignored; 157 matching tests, 558 filtered upstream tests; latest log `scoped-sds-final.log` in the owned task directory |
| `npx tsc -b` | exit0 |
| `npm run lint` | exit0; UI spec118 files; i18n checks passed |
| `node scripts/test-sds-contracts.mjs` | exit0; seven provider fixtures, v2 lifecycle schema, pilot compatibility, actual Rust serialization → TypeScript parity |
| Node launcher/fixture/bundle/installer suite with `SDS_BUNDLE_DIR=.superpowers/sdd/2026-10-17-sds-phase2/dist-sds` | 12 passed, zero failed; candidate bundle contains Phase2 IPC and excludes forbidden upstream commands |
| `npx vite build --outDir .superpowers/sdd/2026-10-17-sds-phase2/dist-upstream` | exit0; upstream frontend built; existing large-chunk warning retained |

The build candidate uses `scripts/sds.mjs::makeLaunch` with an explicit development environment, fixtures disabled, `sds` feature and `--no-default-features`. Its config overlay redirects frontend output into the owned task directory. It does not overwrite the user's pre-existing `dist-sds/` or `dist/`. A SHA256 inventory of each original output tree was captured before building. Native pilot application data was copied to the owned task backup before upgrade acceptance; no application was running during that copy.

## Native behavior covered by the scoped suite

- Migration001 is unchanged. Pilot rows, deployment baseline and pending v1 journal bytes survive additive migrations and reopen. A failed migration004 rolls back both its first CREATE and version advancement; future schema is rejected.
- AIPro project/global resolution, canonical identity deduplication and moved/recreated project rejection. MCP remains global.
- Snapshot capture includes added/changed/deleted files including a deleted SKILL.md, retains original baseline, validates copy/source before publication, rejects corrupt/missing bytes, blocked destination and depth33 without mutating source.
- The depth regression first crashed with Windows STATUS_STACK_OVERFLOW. The fix places the recursive scanner's64KiB read buffer on the heap without changing the32-depth,10000-entry or500MiB expanded limits. The normal-stack regression now passes.
- Replacement rejects changed staging bytes, recreated roots and unmanaged collisions while retaining old/staged copies. Repeated completed commits verify frozen evidence and live content. A restored edited snapshot remains Modified against the original release baseline.
- Actual subprocess exit73 at each of six durable replacement boundaries runs for **both update and restore**; each reopen recovers twice and verifies a valid old/new tree and original restored baseline. An additional subprocess exits before intent publication and proves the current tree remains unchanged and stale review expires.
- Actual cache-cleanup process exits cover five durable boundaries. Recovery is idempotent. Cache deletion refuses deployment/pending-operation references, including a reservation created after cleanup review.
- Selected updates change only the reviewed target among global and two projects, review new release MCP requirements, preserve Keep without network, stop after revoked permission or failed snapshot, and retry only the failed target through a fresh review without replaying successful targets.
- Modified removal requires explicit acknowledgement, finishes selected backup before removal and preserves other targets/cache. Snapshot restores after unreferenced cache removal with the original edited baseline.
- Offline cached deployment requires prior profile/account grant and verified bytes, performs zero HTTP, rejects signed-out/expired states and changed contexts, and cannot reuse a grant after HTTP403 or per-item unavailable/unknown-release followed by outage. The prior grant is rechecked inside the guarded commit.
- Competing update/remove commits run four actual thread races on the same target. Exactly one wins; the other retains current or verified staged content. This regression passed before the final150-pass run.

Snapshot listing now offers explicit Verify snapshot using a read-only, blocking-worker native command `sds_verify_snapshot({snapshotId})`. It shows checking/verified-at/corrupt-or-missing/error states and disables restore after failed verification. Restore independently re-verifies bytes. The UI regression first failed because the verification control was absent, then passed with the implementation. Hashing all historical snapshots is not forced on every list render.

Ignored entries comprise two opt-in external integrations (dedicated Store consumer and Windows Credential Manager) and five owned subprocess helpers. The crash parent tests execute their helper processes explicitly; the helper ignore markers do not mean those crash scenarios were skipped. The dedicated real Store provider evidence is separate in `phase2-store-updates.md`.

## Remaining gate evidence

Follow-up requirement audit added the focused workspace unregister guard regression in parent-owned `phase2_extra.rs`. Fresh focused run passed1/1, then the same freshly compiled binary ran all `sds::` tests: **151 passed,0 failed,7 ignored,558 filtered out**,15.66s. Saved output is `scoped-sds-reference-audit.log`. The new test covers persisted pending and completed/deployed references and preserves project contents; no product behavior changed. Earlier150-pass figures identify the prior candidate run.

The current full default Rust command finished with548 passed,3 failed,7 ignored in2362.15s. The failures are the two upstream directory-symlink tests requiring Windows privilege1314 (`commands::projects::tests::remove_workspace_skill_target_removes_directory_symlink_without_touching_target`, `core::sync_engine::tests::remove_target_removes_directory_symlink`) and upstream partial-clone setup (`core::git_fetcher::tests::a_detached_checkout_fails_on_a_missing_object_without_reaching_the_remote`). The latter setup could not produce its expected genuine partial clone after its owned remote was removed. These match the previously documented pilot environment failures; no tests were disabled or rewritten to obtain green. Full `--features sds` independently finished with698 passed,3 failed,14 ignored in2178.32s, reproducing exactly the same three upstream failures. No SDS test failed. These commands preceded the wording-only modified-target guidance correction.

Installer build/upgrade and native window checks, full Rust suite results, final two-axis review, fresh additive Store audit and final source commits must be recorded before declaring G2 complete. The known Phase0–1 upstream failures are not presumed unchanged until the current full commands return. No phase completion is inferred solely from the scoped green suite.
