# Phase 2 acceptance audit — G2 achieved

This is a requirement-by-requirement audit, not a completion certificate. Product baseline is `be1cd17`; implementation and follow-up evidence are committed on `feature/sds-phase-0-1`. Store provider is separately committed as `b010b4c` on `sdsv-api`. User-owned changes and original generated outputs are preserved.

| Requirement / deliverable | Inspected evidence | Current result |
|---|---|---|
| AIPro project/global identity, legacy request compatibility | [Workspaces](phase2-workspaces.md), native global plus project-a/project-b, serde contracts | Implemented; focused persisted reference-removal regression passed1/1 |
| Additive migration, pilot state/journal compatibility | Migrations002–007; migration/future-schema/transaction rollback/preserved-byte tests in saved scoped run | Verified schema7 and reopen; installed upgrade preserved pilot session/cache |
| Snapshot contents and original baseline, missing/deleted SKILL.md | [Snapshots](phase2-snapshots.md), actual captured file hashes, isolated fault tests | Implemented and verified; native manual destination failure/refreshed retry completed with hashes |
| Restore and durable replacement, unmanaged/root guards | [Replacement](phase2-replacement.md), [crash matrix](../testing/phase2-crash-matrix.md), native restores | Verified automated boundaries and edited baseline; native prepared-operation kill/restart preserved targets and required new review |
| Authenticated update metadata, privacy, bounds | [Store provider](phase2-store-updates.md), [consumer](phase2-update-consumer.md), actual native metadata/update | Provider139 tests, consumer7 tests and real Spring/PostgreSQL harness; no package download on metadata route |
| Selected updates, Keep, new MCP review, partial retry | [Updates](phase2-updates.md), lifecycle tests, native project-a selection and hashes | Verified automated and native selected update; group global `dcc59c09…` completed while project-b failed before replacement, then fresh project-only `a180efb2…` completed without replaying global |
| Modified target removal, independent cache cleanup | [Removal](phase2-removal.md), actual crash tests, native backup/remove/cache-delete/restore | Native flows completed after explicit confirmations; concurrent publication race reproduced, fixed and independently cross-reviewed with new passing tests |
| Offline prior-account cache; expired/403 distinct | [Offline](phase2-offline.md), zero-HTTP automated assertions, actual outage/cached project-b install | Native detail403 then503 and cached-grant refusal verified; protected401 plus refresh401 showed Session expired, cache/update actions disabled, local Library available |
| Lifecycle crash/races/cancel/retry | [Recovery](phase2-recovery.md) and source assertions for each crash parent | Actual process exits/kills and four thread races verified; native precommit restart verified, durable boundary evidence remains automated |
| IPC/type compatibility, command isolation | [Integration](phase2-integration.md), actual Rust-to-TS fixtures, runtime.rs registrations | Passed; PreparedOperation shape retained, group frozen operation payload additive |
| Store existing source/schema immutability | Fresh guard, file inventory and schema.sql audit after native publication | Passed: original source/schema unchanged, five permitted file additions |
| Windows artifact profiles/TLS/fixtures | Final development/internal receipts, launcher/bundle/installer12-test run, successful final development upgrade | Both cache-fixed profiles built; fixture-free; internal native rejected persisted HTTP and required trusted HTTPS |
| Native layouts/theme/keyboard | Library/Backup1440x860 and1100x640 light/dark, long paths, keyboard project picker, native restore Tab/Shift-Tab/Enter and focus return | Representative native keyboard/layout checks passed; not an exhaustive accessibility certification |
| Independent Standards and Spec review | [Review](phase2-code-review.md) | Both reports completed; P1/P2 reproduced then fixed,167-test GREEN and independent patch cross-reviews found no blocker; one nonblocking P3 duplicate workspace guard deferred |
| Named evidence/handoff and plan bookkeeping | This audit, per-slice evidence, crash matrix, [handoff](phase2-handoff.md) | Evidence consolidated; native checklist and plan gate are closed with operation IDs, receipts and known Windows-only upstream test limits recorded |

## Commands and results

The detailed [integration report](phase2-integration.md) records57 UI tests, the initial150 SDS Rust tests, lint/typecheck/contracts and candidate checks. The later workspace-reference regression passed1/1. Current scoped Rust after the Windows rename correction passed172/0failed/7ignored (`rename-termination-green.log`); the final Node launcher/fixture/bundle/installer run passed12/12 against the final source.

Final full Rust with SDS:716 passed,3 failed,14 ignored (`full-sds-cache-fix.log`). The failures are the two upstream Windows directory-symlink tests, which require unavailable privilege1314, and an upstream partial-clone fixture setup. No SDS lifecycle test failed. No skipped rewrite or privilege/security change was used to mask the failures. Existing warnings and actual failed commands remain recorded.

Build verification used task-owned output roots to keep original `dist/` and `dist-sds/` unchanged. The requested build intent was fulfilled through the established SDS launcher/overlays and separate upstream Vite output, rather than overwriting those pre-existing outputs. Exact delivery hashes and native operations are in [native acceptance](phase2-native-acceptance.md).

## Gate ruling

G2 is achieved. P2.1–P2.8 have source, integration, crash and native evidence. The final native two-target case prepared and acknowledged both frozen reviews; clean global `dcc59c09-8a0e-4d45-8686-1f40618cb483` completed while modified project-b `37fea467-8f27-48d3-95bb-15f7893c8b2d` failed before replacement against the owned blocking snapshot root. After restoring the original snapshot tree, the UI offered only project-b for a fresh review; operation `a180efb2-4d16-458a-8735-b9b057294b34` completed and global was not replayed. The Store source/schema additive audit passed again after the owned release1.0.4 publication, excluding regenerated `backend/target` outputs from both inventories. The known full-suite Windows privilege/fixture failures are recorded above and do not cover SDS lifecycle behavior.
