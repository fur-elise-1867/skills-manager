# Phase 2 acceptance audit — G2 open

This is a requirement-by-requirement audit, not a completion certificate. Product baseline is `be1cd17`; implementation and follow-up evidence are committed on `feature/sds-phase-0-1`. Store provider is separately committed as `b010b4c` on `sdsv-api`. User-owned changes and original generated outputs are preserved.

| Requirement / deliverable | Inspected evidence | Current result |
|---|---|---|
| AIPro project/global identity, legacy request compatibility | [Workspaces](phase2-workspaces.md), native global plus project-a/project-b, serde contracts | Implemented; focused persisted reference-removal regression passed1/1 |
| Additive migration, pilot state/journal compatibility | Migrations002–007; migration/future-schema/transaction rollback/preserved-byte tests in saved scoped run | Verified schema7 and reopen; installed upgrade preserved pilot session/cache |
| Snapshot contents and original baseline, missing/deleted SKILL.md | [Snapshots](phase2-snapshots.md), actual captured file hashes, isolated fault tests | Implemented and verified; native destination-failure recovery UX remains open |
| Restore and durable replacement, unmanaged/root guards | [Replacement](phase2-replacement.md), [crash matrix](../testing/phase2-crash-matrix.md), two native restores | Verified automated boundaries and edited baseline; installed interrupted-operation restart remains open |
| Authenticated update metadata, privacy, bounds | [Store provider](phase2-store-updates.md), [consumer](phase2-update-consumer.md), actual native metadata/update | Provider139 tests, consumer7 tests and real Spring/PostgreSQL harness; no package download on metadata route |
| Selected updates, Keep, new MCP review, partial retry | [Updates](phase2-updates.md), lifecycle tests, native project-a selection and hashes | Verified automated and native selected update; no auto-update or installName migration |
| Modified target removal, independent cache cleanup | [Removal](phase2-removal.md), actual crash tests and cache-independent restore integration | Implemented/tested; prepared native backup/remove review awaits human action-time confirmation |
| Offline prior-account cache; expired/403 distinct | [Offline](phase2-offline.md), zero-HTTP automated assertions, actual outage/cached project-b install | Core policy verified; native expired/403 presentation remains open |
| Lifecycle crash/races/cancel/retry | [Recovery](phase2-recovery.md) and source assertions for each crash parent | Actual process exits/kills and four thread races verified; installed GUI interruption still open |
| IPC/type compatibility, command isolation | [Integration](phase2-integration.md), actual Rust-to-TS fixtures, runtime.rs registrations | Passed; PreparedOperation shape retained, group frozen operation payload additive |
| Store existing source/schema immutability | Fresh guard, file inventory and schema.sql audit after native publication | Passed: original source/schema unchanged, five permitted file additions |
| Windows artifact profiles/TLS/fixtures | Final development/internal receipts, launcher/bundle/installer12-test run, successful final development upgrade | Built; fixture-free; internal native HTTPS-only configuration acceptance remains open |
| Native layouts/theme/keyboard | Library/Backup1440x860 and1100x640 light/dark, long paths, keyboard project picker | Partial; complete lifecycle-dialog/keyboard audit remains open |
| Independent Standards and Spec review | [Review](phase2-code-review.md) | Open: prior reviewer workers hit account usage limit; local review is not substituted as independent approval |
| Named evidence/handoff and plan bookkeeping | This audit, per-slice evidence, crash matrix, [handoff](phase2-handoff.md) | Evidence consolidated; plan still must retain open acceptance checks until proven |

## Commands and results

The detailed [integration report](phase2-integration.md) records57 UI tests, the initial150 SDS Rust tests, lint/typecheck/contracts and candidate checks. The later workspace-reference regression passed1/1, followed by the full newly compiled `sds::` binary run:151 passed,0 failed,7 ignored,558 filtered out,15.66s (`scoped-sds-reference-audit.log`). No product behavior changed in this follow-up. Final Node launcher/fixture/bundle/installer run passed12/12 against the current candidate bundle.

Full default Rust:548 passed,3 failed,7 ignored. Full with SDS:698 passed,3 failed,14 ignored. Both fail on the same two upstream Windows symlink privilege tests and partial-clone fixture setup; no SDS failure. No skipped rewrite or privilege/security change was used to mask the failures. Existing warnings and actual failed commands remain recorded.

Build verification used task-owned output roots to keep original `dist/` and `dist-sds/` unchanged. The requested build intent was fulfilled through the established SDS launcher/overlays and separate upstream Vite output, rather than overwriting those pre-existing outputs. Exact delivery hashes and native operations are in [native acceptance](phase2-native-acceptance.md).

## Gate ruling

P2.1–P2.8 product slices are implemented with substantial source, integration and crash evidence. G2 is **not passed** while the native acceptance and independent-review items above remain unverified. Do not infer completion from green scoped tests, successful builds, this file's existence or elapsed time waiting for confirmation.
