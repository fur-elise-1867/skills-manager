# Phase 2 review checkpoint

Current ruling: independent Standards/Spec review and cross-reviews completed; P1/P2 are fixed in ee3661f. The later cache-missing restore regression is fixed in0a9d094 with observed RED then168 scoped GREEN and an independent patch review finding no blocker. Actual installed global retest passed with hashes. Earlier unresolved/account-limit/build statements below are historical, superseded by their resolution sections and current acceptance report. One P3 duplicated workspace guard remains a nonblocking maintenance item.

Fixed point: `be1cd179ca3cba957317a8a1c877716fcb44ad0c`. Review includes new untracked Phase2 source, not only tracked diff. Excludes pre-existing `.gitignore`, generated `dist-sds/`, and main-checkout user changes.

The requested Matt Pocock workflow calls for separate Standards and Spec reviewers. Independent reviewer workers hit the account usage limit before delivering their reports. The checks below are a local integration review and do not claim that the independent two-axis gate has completed.

## Independent follow-up review of be1cd17...2bf2e05

Two fresh independent read-only reviewers completed the actual diff review. These reports supersede the earlier account-limit-only status, but **do not approve G2** while the Spec defects remain unresolved. They did not run tests/builds or modify files.

### Standards

No hard breach substantiated in inspected paths. Two P3 heuristic findings, both judgment calls: duplicated six-code denial classification/revoke action in operations.rs can drift between detail/release paths; duplicated workspace deployment/pending-reference guard exists in commands.rs and WorkspaceService::remove, causing duplicate scans/error text. The latter is a maintenance suggestion, not a demonstrated safety regression. The reviewer limits the conclusion to focused static inspection of native/React/contracts and documented constraints.

### Spec

P1: commands.rs skill-detail browsing calls StoreClient directly, bypassing the grant revocation in authorized_release. A prior grant can therefore survive a known item403/404 followed by outage, contrary to P2.7's no-denial-fallback rule. P2: cache cleanup checks references under cache_lock, but new operation reference publication does not share that lock, allowing a reservation after the check and before deletion, contrary to P2.6's shared lock/reservation invariant. The prior reservation test only inserts before execution, not inside this interleaving. No actionable scope creep identified. Both are source-derived findings pending deterministic reproduction and narrow fixes.

Counts: Standards0 hard breaches/2 P3 heuristics; Spec2 actionable findings, worst P1. Reviewers: Planck `01a14cf3-142e-7523-997b-440f87eb59cb`, Peirce `01a14cf3-251e-7da0-b4bb-bfa5210829d7`. Separate test-first follow-up slices are assigned with disjoint source/test ownership and a single parent-controlled Cargo run.

## Review regression and resolution, commit ee3661f

Parent registered both independent test modules, then ran combined RED before either security/race implementation:149 passed,7 failed,7 ignored (the `sds::tests::` filter excludes seven metadata tests). Four failures demonstrated cached preparation still permitted after UI detail403/404/410 or canDownload=false; three demonstrated pending/committed reference publication bypassing cleanup's cache lock. Stale-account guards and existing publication/snapshot-restore controls passed. Saved log: `review-combined-red.log`.

P1 now routes actual SDS detail IPC through a context-captured OperationService entrypoint. Denials and canDownload=false revoke only requesting profile/account/skill under the current-context guard; detail/release authorization share one denial classifier. P2 now holds publication leases across reference writes, gives cleanup an independent exclusive lock, retains durable-intent references and rejects publication during interrupted pre-DB deletion. Existing-lease reuse avoids re-entry; missing physical cache remains valid for self-contained snapshot restoration.

Fresh full `sds::` GREEN:167 passed,0 failed,7 ignored,558 filtered out in21.54s (`review-combined-green.log`), including metadata and four added cache recovery controls. UI57, lint/UI-spec/i18n and Rust→TS contract parity reran successfully. Cross-review was independent of each patch's author: Planck found no blocker in P1 and confirmed classifier duplication resolved; Peirce found no blocker in P2 after inspecting same/second-handle contention, nonblocking lock order, interrupted deletion and post-DB republication. Neither cross-reviewer ran tests; test counts above are the parent's actual output.

Remaining Standards P3: duplicate workspace guard is explicitly deferred as nonblocking maintenance; the authoritative service guard and persisted reference-removal regression remain intact. No unresolved P1/P2 review finding remains at this checkpoint. Native/full-suite/artifact acceptance remains a separate gate; post-review installers and full Rust run are in progress.

## Standards

Inspected the module boundaries and changed session/operation/repository/runtime paths. Lifecycle options are separated from pending v1 journal DTOs; snapshot, replacement, offline and cache cleanup have distinct ownership. Shared copy, fingerprint, target locks and no-follow directory handles are reused. No edits to existing migration001 or Store source/config/security/dependency files. Lint/typecheck and candidate IPC boundary checks passed. Existing compact JSX and SQL statements follow surrounding style; no speculative architecture rewrite is required for this phase.

## Spec and correctness

Reviewed guards for Store-vs-offline commit context, prior-account grant revocation and commit recheck, target identity/path verification, independent snapshot baseline, crash evidence reconciliation and manifest-bounded cleanup. Read the selected-group UI preparation/review/acknowledgement/polling/retry path against the native persisted DTO. The additive `operation` payload is required to show the frozen MCP/release review and is now covered by schema/serde parity. UI receives current observed fingerprint for manual snapshot capture. Actual competing update/remove commits now run four rounds with exactly one winner and retained content for the losing replacement; the scoped regression passed.

Previously discovered defects were fixed and exercised: snapshot scanner stack overflow; completed replacement replay; HTTP403 ACCESS_DENIED grant invalidation; cached pristine reinstall; missing frozen group review payload. No unresolved data-loss defect was identified in these inspected paths. This is bounded review evidence, not proof that every plan acceptance item is complete.

## Build/acceptance finding

The first owned candidate build overlay used an absolute Windows `frontendDist`. Tauri interpreted the drive prefix as a URL and the installed app exposed an Index-of-directory WebView instead of embedded assets. This was observed in native accessibility state, not inferred from a successful build. The task-owned wrapper now uses a path relative to `src-tauri`; both candidate installers need rebuilding and native verification. Product `tauri.sds.conf.json` already uses a relative path and was not responsible for this harness defect. The initial installer hashes are not accepted artifacts.

## Open verification

- Full default finished548 passed/3 failed/7 ignored; full `--features sds` finished698 passed/3 failed/14 ignored. Both reproduced the same two Windows symlink privilege failures and one upstream partial-clone fixture failure; no SDS test failed. See `phase2-integration.md`.
- Corrected development/internal artifacts built successfully. Native upgrade, selected update/MCP review, snapshot verification, manual/protected restore and prior-authorized offline deployment were observed. Light/dark Library/Backup were inspected at1440x860 and1100x640 client sizes. Remaining native cases and precise receipts are recorded in `phase2-native-acceptance.md`.
- The snapshot-list gap was fixed after this review: explicit Verify snapshot now displays checking/verified-at/corrupt-or-missing/error states and blocks restore after verification failure. Its UI test was observed red then green; backend restore independently verifies again. Native Verify reported Verified and the subsequent restore succeeded.
- Independent Standards/Spec reports and final G2 handoff remain open.
