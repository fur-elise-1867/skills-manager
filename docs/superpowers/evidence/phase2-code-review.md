# Phase 2 review checkpoint

Fixed point: `be1cd179ca3cba957317a8a1c877716fcb44ad0c`. Review includes new untracked Phase2 source, not only tracked diff. Excludes pre-existing `.gitignore`, generated `dist-sds/`, and main-checkout user changes.

The requested Matt Pocock workflow calls for separate Standards and Spec reviewers. Independent reviewer workers hit the account usage limit before delivering their reports. The checks below are a local integration review and do not claim that the independent two-axis gate has completed.

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
