# Phase 2 acceptance audit — G2 open

This is a requirement-by-requirement audit, not a completion certificate. Product baseline is `be1cd17`; implementation and follow-up evidence are committed on `feature/sds-phase-0-1`. Store provider is separately committed as `b010b4c` on `sdsv-api`. User-owned changes and original generated outputs are preserved.

| Requirement / deliverable | Inspected evidence | Current result |
|---|---|---|
| AIPro project/global identity, legacy request compatibility | [Workspaces](phase2-workspaces.md), native global plus project-a/project-b, serde contracts | Implemented; focused persisted reference-removal regression passed1/1 |
| Additive migration, pilot state/journal compatibility | Migrations002–007; migration/future-schema/transaction rollback/preserved-byte tests in saved scoped run | Verified schema7 and reopen; installed upgrade preserved pilot session/cache |
| Snapshot contents and original baseline, missing/deleted SKILL.md | [Snapshots](phase2-snapshots.md), actual captured file hashes, isolated fault tests | Implemented and verified; native manual destination failure/refreshed retry completed with hashes |
| Restore and durable replacement, unmanaged/root guards | [Replacement](phase2-replacement.md), [crash matrix](../testing/phase2-crash-matrix.md), native restores | Verified automated boundaries and edited baseline; native prepared-operation kill/restart preserved targets and required new review |
| Authenticated update metadata, privacy, bounds | [Store provider](phase2-store-updates.md), [consumer](phase2-update-consumer.md), actual native metadata/update | Provider139 tests, consumer7 tests and real Spring/PostgreSQL harness; no package download on metadata route |
| Selected updates, Keep, new MCP review, partial retry | [Updates](phase2-updates.md), lifecycle tests, native project-a selection and hashes | Verified automated and native selected update; no auto-update or installName migration |
| Modified target removal, independent cache cleanup | [Removal](phase2-removal.md), actual crash tests, native backup/remove/cache-delete/restore | Native flows completed after explicit confirmations; concurrent publication race reproduced, fixed and independently cross-reviewed with new passing tests |
| Offline prior-account cache; expired/403 distinct | [Offline](phase2-offline.md), zero-HTTP automated assertions, actual outage/cached project-b install | Native detail403 then503 and cached-grant refusal verified; protected401 plus refresh401 showed Session expired, cache/update actions disabled, local Library available |
| Lifecycle crash/races/cancel/retry | [Recovery](phase2-recovery.md) and source assertions for each crash parent | Actual process exits/kills and four thread races verified; native precommit restart verified, durable boundary evidence remains automated |
| IPC/type compatibility, command isolation | [Integration](phase2-integration.md), actual Rust-to-TS fixtures, runtime.rs registrations | Passed; PreparedOperation shape retained, group frozen operation payload additive |
| Store existing source/schema immutability | Fresh guard, file inventory and schema.sql audit after native publication | Passed: original source/schema unchanged, five permitted file additions |
| Windows artifact profiles/TLS/fixtures | Final development/internal receipts, launcher/bundle/installer12-test run, successful final development upgrade | Both cache-fixed profiles built; fixture-free; internal native rejected persisted HTTP and required trusted HTTPS |
| Native layouts/theme/keyboard | Library/Backup1440x860 and1100x640 light/dark, long paths, keyboard project picker, native restore Tab/Shift-Tab/Enter and focus return | Representative native keyboard/layout checks passed; not an exhaustive accessibility certification |
| Independent Standards and Spec review | [Review](phase2-code-review.md) | Both reports completed; P1/P2 reproduced then fixed,167-test GREEN and independent patch cross-reviews found no blocker; one nonblocking P3 duplicate workspace guard deferred |
| Named evidence/handoff and plan bookkeeping | This audit, per-slice evidence, crash matrix, [handoff](phase2-handoff.md) | Evidence consolidated; plan still must retain open acceptance checks until proven |

## Commands and results

The detailed [integration report](phase2-integration.md) records57 UI tests, the initial150 SDS Rust tests, lint/typecheck/contracts and candidate checks. The later workspace-reference regression passed1/1, followed by the full newly compiled `sds::` binary run:151 passed,0 failed,7 ignored,558 filtered out,15.66s (`scoped-sds-reference-audit.log`). No product behavior changed in this follow-up. Final Node launcher/fixture/bundle/installer run passed12/12 against the current candidate bundle.

Full default Rust:548 passed,3 failed,7 ignored. Full with SDS:698 passed,3 failed,14 ignored. Both fail on the same two upstream Windows symlink privilege tests and partial-clone fixture setup; no SDS failure. No skipped rewrite or privilege/security change was used to mask the failures. Existing warnings and actual failed commands remain recorded.

Build verification used task-owned output roots to keep original `dist/` and `dist-sds/` unchanged. The requested build intent was fulfilled through the established SDS launcher/overlays and separate upstream Vite output, rather than overwriting those pre-existing outputs. Exact delivery hashes and native operations are in [native acceptance](phase2-native-acceptance.md).

## Gate ruling

P2.1–P2.8 product slices are implemented with substantial source, integration and crash evidence. G2 remains open pending the final full-suite verdict and the plan's native two-target mixed-outcome/retry case. Single-target actual snapshot failure and fresh retry, fresh-global cache replenishment, expired/403 and internal HTTPS checks are now verified. The two-target selection/isolation/retry behavior is already covered by service/UI tests, but those do not substitute for the specifically planned native case. Independent review completed and its P1/P2 fixes passed cross-review.

Post-review full run was interrupted by the10:42 tool/runtime reset; no terminal verdict exists. A new snapshot-restore/cache-missing lifecycle regression failed with CACHE_CORRUPT before the fix, then the complete scoped suite passed168/0failed/7ignored. Present tampered cache still fails closed; absent cache triggers a verified Store download, while cached-only offline deployment remains blocked. Native retest passed in installed0a9d094, three global files matched and seven unselected project files were unchanged. New full run is saved to `full-sds-cache-fix.log`; inspect its live handle/terminal output before reporting a result.
