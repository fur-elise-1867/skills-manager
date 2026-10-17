# Phase 2 execution checkpoint — G2 open

This is a continuation record, not a completion certificate. Source lives in `.worktrees/sds-phase0-1`, branch `feature/sds-phase-0-1`, baseline `be1cd17`. Main checkout documentation/user changes and worktree `.gitignore`, original `dist/` and `dist-sds/` remain separate.

## Authoritative continuation checkpoint, 2026-10-18 01:30 Asia/Saigon

The older execution notes below are historical where superseded here. Full suites have drained: default548 passed/3 failed/7 ignored; `--features sds`698 passed/3 failed/14 ignored. The same three upstream environment/setup failures appear in both; no SDS failure. Do not restart these long suites without a new reason. UI57/SDS scoped150/provider139 remain the source verification counts. Final candidate bundle/launcher/fixture/installer tests reran12/12 green.

Source commits now include `66a20be` (modified-target guidance and native recovery evidence) and `77f46ac` (full-suite/offline evidence), after implementation `a10a5a6`. Final wording-only change was compiled into both installers. Current receipts: development6,371,640bytes/SHA256 `82226e2d886949da0444f8fd8721ae550d0fb8d7e40816bc53e78aca93a6b3b8`; internal6,373,172bytes/SHA256 `688231d49d199a166beba02cc17308d00953c7ea0dfde7650ede56cfe441df69`. Both builds completed. Final development installed with exit0. Earlier installer hashes are superseded.

Native selected update, Verify snapshot, protected restore, manual snapshot then second-edit protection, and prior-authorized offline cached deployment all succeeded with hash proofs. Registered project-a and project-b plus the historical global deployment now exist. Full details/IDs are in `phase2-native-acceptance.md`; machine-readable proofs are in taskroot/native. Original unselected global files remain unchanged. Final installed Library displays the corrected modified-target guidance. Library/Backup checked light/dark at1440x860 and1100x640 client sizes; project selection exercised keyboard End/Return. Complete keyboard/dialog audit remains open.

Store currently runs in exec78878, latest JVM14860 at startup01:21:55. PostgreSQL retained on15432. Reverify process/listener identity before any stop. Fresh guard/source/schema audit after native publication/workflows passed with five additions and no existing source/schema changes. Raw inventories: `store-files-native-final.json`, `store-schema-native-final.json`. Never print the environment script's secrets.

Native sky window is now20057228, returned app path still the owned pilot installed-shell. Latest UI is **Remove skill** review for project-a deployment `a7dbb1c1-3148-4834-ac47-a8c2323f639f`; backup is selected and modified-copy acknowledgement was given. Final **Back up and remove** has NOT been clicked. An asynchronous human confirmation is pending for exactly this UI deletion, required by Computer Use confirmations (always confirm at action time). Reobserve before any action; do not treat elapsed time as consent. Global/project-b/cache remain outside that proposed deletion. Cache deletion will separately require action-time confirmation if performed through UI.

Remaining: approved native removal/cache workflows, snapshot failure UX, denied/expired UI, interrupted-operation native restart, fuller keyboard/dialog checks, internal native configuration acceptance, independent Standards/Spec reports (previous workers hit account limits), evidence consolidation and G2 handoff. Goal remains active; no completion claim.

## Implemented interfaces

Schema7 via additive002–007 migrations. AIPro registered project/global targets, snapshot capture/verify/restore, replacement journal/recovery, selected updates with frozen per-operation review/MCP payload, modified removal acknowledgement/backup, referenced-cache guards/journaled deletion, account-scoped offline cached deployment. Public IPC registrations are in `sds/runtime.rs`, not the upstream handler. PreparedOperation retains the pilot serialized shape; lifecycle context/options persist by operationId. Group success items add `operation` alongside `operationId` for authoritative review. `sds_verify_snapshot({snapshotId})` performs explicit read-only snapshot verification on a blocking worker.

## Verified source

- UI57 passed; Rust SDS150 passed,7 ignored (two external opt-ins and five helpers executed by parent crash tests); lint/typecheck/contracts passed. See `phase2-integration.md` and `phase2-update-consumer.md`.
- Store139 tests passed, real Spring/PostgreSQL HTTP harness passed,1,072 prior files and553 schema definitions unchanged. Provider committed separately as `b010b4c` on Store branch `sdsv-api`.
- Competing update/remove target race has one winner; actual update/restore process exits run twelve crash scenarios plus before-intent; cachecleanup has five actual process exits. Snapshot baseline remains distinct from edited content.

## Native observations so far

The task-owned installer wrapper is `.superpowers/sdd/2026-10-17-sds-phase2/build-candidate.mjs`. An initial absolute Windows frontendDist incorrectly parsed as a URL; fixed to a path relative to src-tauri. Initial bad artifacts are not accepted. Corrected development build installed with exit0 into the existing owned pilot `installed-shell`; native WebView now uses `http://tauri.localhost/` and displays the actual app.

Pilot user/session and three cache releases survived upgrade. An actual Store outage displayed offline Library/provenance/disabled update checks. The dedicated Store/DB had stopped after worker lifecycle termination; both were freshly verified absent before restarting only the owned PostgreSQL15432 and protected Spring8080. `Retry connection` performed the protected request and returned to authenticated Library, then metadata check exposed a real update.

Registered the existing owned `native/project-a` through the Windows folder picker. Installed the real prior7z release into `<project-a>/.aipro/skills/sds-pilot-7z`, operation `a7dbb1c1-3148-4834-ac47-a8c2323f639f`, native result Installed successfully. Three files at the unselected original pilot target retained their exact SHA256 hashes. The added owned `phase2-local.txt` was detected as Local changes in selected-update review. Keep current preserved it.

Native latest sequence3 used another installName (`sds-pilot-unavailable`); prepare correctly rejected it with explicit migration-unavailable guidance, no package install and current content preserved. To exercise a positive same-name update, published only a new immutable desktop release into the guard-verified dedicated database: skill2, release `05548974-6462-437b-8ff0-fa8d9f5a66e6`, sequence4/version1.0.3, installName `sds-pilot-7z`, synthetic MCP `phase2-synthetic-mcp`, original verified369-byte7z fixture. Receipt digest `d7a52ca765e24c3f233c1f901786958e5ad50b9c0ea0c2cebebffc0598efe27a`. Existing releases/business data/schema were not modified. Owned manifest is `native/release-004.json`.

A real long source-profile hash overflowed Library cards in the prior candidate. Source now wraps that metadata and toolbar controls; verify the latest built candidate at both sizes before closing visual acceptance.

## Pending work and authoritative handles

All paths below are under `.superpowers/sdd/2026-10-17-sds-phase2` unless absolute:

- Full default suite: exec session92623, `full-default-tests.log`; full `--features sds`: session34580, `full-sds-features-tests.log`. Inspect current handles/processes, do not restart from elapsed time alone. Default output already reproduces a known missing-promisor-object upstream failure; wait for final counts. Full suites are not green by assumption.
- Final corrected development installer source includes verify-snapshot and Library wrap fixes; receipt `artifacts/development/receipt.json`:6,373,398bytes, SHA256 `3c4cb9ab64df1256284b064024a7e28f574d411fc94530e00559080969b1e50d`. It has not yet upgraded the currently open earlier native candidate.
- Final internal build session66891 is running; inspect returned completion and receipt. Prior internal build is superseded by latest CSS fix.
- Native Store session91579, log `store-native.log`, Java41048 at latest startup. PostgreSQL was started via pg_ctl into the owned pilot data root on loopback15432. Reverify listeners/process identity before stopping/restarting. Environment source is the pre-existing owned `store-env.ps1`; do not print credentials.
- Finish same-name selected project update, new-MCP acknowledgement, snapshot content/baseline evidence, manual snapshot/restore, modified remove/cache delete, offline prior-grant deployment, expired/403 distinction and minimum/light-dark native acceptance. `phase2-manual.md` remains unchecked until each actual observation.
- Native automation uses node_repl `sky` from the computer-use skill. Cached state is point-in-time; reobserve after any user input. The last UI is the installName-change rejection review; its Keep current action was interrupted by user input and must be reobserved, not assumed executed.
- Independent review workers hit account usage limits before reports. Local review is recorded in `phase2-code-review.md`; independent two-axis final review remains open.
- Refresh additive Store/schema audit after the new synthetic publication, record final artifacts/acceptance, complete plan evidence checkboxes and final G2 handoff only when proven. No push/merge/release publication has been performed.
