# Phase 0 baseline — 2026-10-17 Asia/Saigon

Execution root: `D:/Documents/Git Project/skills-manager-app/.worktrees/sds-phase0-1`.
Code baseline: `161f6d4` (product code `e6dcedf`); input documentation snapshot committed as `1eb9207`.
Original checkout stays at `D:/Documents/Git Project/skills-manager-app/skills-manager`; user edits to .gitignore/logo and docs were not staged or modified there.

## Environment

- Windows x64; Node 24.20.0, npm on PATH 11.4.1, Rust/Cargo1.99.0, Java21.0.7.
- Visual Studio2022 BuildTools with VC.Tools.x86.x64 is installed. Initial Rust compilation produced and ran the test executable, proving the linker/build prerequisites used in that run work.
- WebView2 runtime155.0.4283.45 found in EdgeUpdate registry. Initial pilot packaging may skip provisioning on this machine; other machines require separately verified provisioning at A10.
- Native app create_worktree failed because chat cwd is the outer non-repository directory. Fallback Git worktree is detached, outside source checkout. No user changes copied except the requested docs inputs.

## Commands and outcomes

| Command | Result | Evidence |
|---|---|---|
| `npm ci` | exit0, 430 packages installed, approximately1min | `baseline-npm-ci.log` |
| `npm run lint` | exit0, UI spec68files and i18n checks pass | `baseline-lint.log` |
| `npm run build` | exit0, TypeScript and Vite pass; Vite build20.51s | `baseline-build.log` |
| `cargo test --manifest-path src-tauri/Cargo.toml` | NOT PASS; runner terminated after long local Git fetch test | `baseline-cargo-test.log` |
| test executable `--exact commands::projects::tests::remove_workspace_skill_target_removes_directory_symlink_without_touching_target --nocapture` | exit101/1, failure reproduced before removal at symlink creation | `Os code1314: A required privilege is not held by the client` |

The full Rust baseline reached558tests; this document does not report a completed suite or a pass count. Test `sync_transaction_commits_merges_and_pushes_in_one_call` waited on `git fetch origin main` against its own temp bare repository; tests using the shared test mutex accumulated waits. The owned runner and its identified temp-repository Git child were stopped, not unrelated user processes. No product code changed to suppress these baseline failures.

Ruling: Continue isolated SDS copy-runtime tests and compile checks while keeping upstream baseline failures explicit. No Windows Developer Mode/admin change, no ignored tests, and no claim that upstream regression acceptance is green. Cost if wrong: A10 remains blocked until the baseline failure/timeout has adequate remediation or an explicitly accepted limitation.

## Development tooling and input preservation

New test dependencies initially hit npm11.4.1 Arborist `edgesOut` null during optional peer resolution. The separately installed Node runtime bundles npm11.19.0; invoking that npm directly with pinned Vitest4.0.18/jsdom26 and Testing Library resolves dependencies without changing global npm. Test/build must validate this dependency set before task completion.

All current docs were copied, versioned and review amendments saved before source edits. Companion S01–S04 plan is now reachable. The file/hash inventory of the copy is preserved with this evidence. No Store files or database objects have been mutated.

## Task status

A00 baseline captured with known upstream environment failure and timeout; installer acceptance has not run. A01 in progress. Phase0/Phase1 are not complete.
