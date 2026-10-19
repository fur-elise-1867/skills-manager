# AX Portal Review Remediation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve the outstanding AX Portal review findings without overwriting pre-existing build artifacts or exposing credentials.

**Architecture:** Keep product build artifacts outside the user-owned `dist-sds/` tree during automated verification. Tighten the existing desktop-to-Web fallback tests, make native minimum-window sizing fit the actual content viewport, and correct the handoff evidence rather than treating it as executable configuration.

**Tech Stack:** Node test runner, Vite, React/CSS, Rust/Tokio tests, PowerShell evidence tooling.

**Spec:** `docs/superpowers/plans/2026-10-19-ax-portal-gemini-development-test.md` and the review findings for `e84f2a1...89cc048`.

## Global Constraints

- Do not modify DB, backend, credentials, or the Store repository.
- Do not write, delete, restore, stage, or regenerate user-owned `dist-sds/**` files.
- Credentials may only be supplied through environment variables; reports, commands, and screenshots must not contain their values.
- Preserve the SDS identifier, binary name, runtime profile, storage root, and upstream branding.
- Native minimum window remains 1100×640, but its client content must not horizontally overflow.

## Review Focus

1. Running the Node SDS bundle suite from a stale or absent `dist-sds/` must build and inspect an isolated temporary output instead of touching user artifacts.
2. A desktop 401 or 503 with a working Web endpoint must retain its error classification and must not request Web routes.
3. At the native 1100×640 minimum, the root grid must fit the client viewport without a horizontal scrollbar.
4. Handoff artifacts must never contain credentials and must name the exact final commit.
5. Existing untracked user files remain untouched; missing historical `dist-sds` changes are explicitly reported rather than reconstructed.

---

### Task 1: Isolated bundle verification

**Files:**
- Modify: `scripts/sds-bundle.test.mjs`

**Interfaces:**
- Produces: a temporary Vite SDS output directory used by all bundle assertions, deleted after the test process completes.

- [ ] **Step 1: Write the failing test setup** so the suite requires an isolated SDS build when `SDS_BUNDLE_DIR` is unset.
- [ ] **Step 2: Run** `node --test scripts/sds-bundle.test.mjs` and verify the existing stale `dist-sds/index.html` failure.
- [ ] **Step 3: Implement** a temp-directory Vite build using the local Vite binary; retain `SDS_BUNDLE_DIR` as an explicit override and clean only the created temp directory.
- [ ] **Step 4: Run** `node --test scripts/sds-bundle.test.mjs` and verify all assertions pass without changing `dist-sds/**`.

### Task 2: Fallback and minimum-window regressions

**Files:**
- Modify: `src-tauri/src/sds/tests/catalog.rs`
- Modify: `src/features/sds/sds.css`

**Interfaces:**
- Produces: explicit no-Web-fallback coverage for 401 and 503; root layout constrained to its native viewport.

- [ ] **Step 1: Write failing Rust tests** where only desktop paths return 401/503 but Web routes would return valid catalog data; assert the Web paths are never requested.
- [ ] **Step 2: Run** the filtered catalog tests and verify they fail before production changes.
- [ ] **Step 3: Implement** the smallest testable fallback behaviour adjustment only if the RED test finds a defect; otherwise retain the established exact-500 policy and refactor tests only.
- [ ] **Step 4: Remove the CSS root `min-width: 1100px`/`min-height: 640px` constraint, relying on Tauri’s outer-window minimum and `min-width: 0` grid sizing.**
- [ ] **Step 5: Run** catalog tests plus the SDS frontend suite; rebuild/run native UAT only through a path that does not overwrite `dist-sds`.

### Task 3: Evidence integrity and handoff

**Files:**
- Modify: `docs/superpowers/evidence/2026-10-19-ax-portal-gemini-handoff.md`

**Interfaces:**
- Produces: a redacted handoff tied to the actual remediation commit, with no false minimum-window claim.

- [ ] **Step 1: Add a failing documentation guard** to the Node test suite that rejects credential assignments or credential literals in AX Portal handoff evidence.
- [ ] **Step 2: Run** that guard and verify it fails against the existing report.
- [ ] **Step 3: Redact all credential values, update the remediation SHA to the real final commit after implementation, and state that the old user-owned `dist-sds` state is unavailable for recovery from Git.**
- [ ] **Step 4: Update the minimum-window claim only after fresh native evidence; otherwise state the native visual recheck is pending.**
- [ ] **Step 5: Run** `git diff --check`, lint, frontend tests, catalog tests, and Node SDS tests. Commit only source/tests/docs created by this remediation.

## Known Blocker

The pre-review `dist-sds/**` worktree changes were uncommitted and have no Git/reflog recovery object. Do not invent replacements or overwrite the current tree. Recovery needs the user’s backup or an external artifact source.
