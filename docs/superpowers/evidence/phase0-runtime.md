# SDS Phase 0 development checkpoint — 2026-10-17 Asia/Saigon

Workspace: `D:/Documents/Git Project/skills-manager-app/.worktrees/sds-phase0-1` (manual Git worktree, detached HEAD).
Input/plan commit `1eb9207`; baseline evidence `f1b0e06`; runtime/shell `78986c3`; byte-preserving docs `5b7e0f5`; installer isolation `1e3d383`.

## Delivered source

- SDS runtime dispatch before upstream initialization, isolated data root `%LOCALAPPDATA%/SDS/SkillsManager`, dedicated identifier `com.sds.skillsmanager.desktop`.
- Backend binary/config mismatch serves a minimal identity-only runtime with no storage initialization. Both frontend entry paths reject mismatched backend identity before importing upstream bootstrap/context.
- SDS runtime does not register public installs, Git, CLI, updater, or upstream background initializers. Dedicated capabilities/CSP exclude updater/shell/public assets.
- Launcher synchronizes profile/environment/fixture flags, rejects long/short config/feature/runner overrides, and passes the effective SDS overlay to Cargo check via TAURI_CONFIG.
- English shell with one token source, independent `sds.theme` preference, runtime handshake and disabled unfinished navigation. Store/session work starts at A03; no real Store requests exist yet.
- Distinct `sds-skills-manager.exe` prevents the NSIS process-name guard from targeting upstream `skills-manager.exe`. CLI source content is unchanged; its default-feature-gated target was moved outside auto-discovery so Tauri CLI2.10 excludes it from SDS bundles. Upstream default builds retain CLI.

## Verification

| Check | Observed result |
|---|---|
| `cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --features sds --lib sds::tests::profile` | 6 pass; final no-default-features run exit0, compile44.46s |
| React/profile handshake tests | 6 pass |
| Node launcher tests | 6 pass |
| SDS frontend bundle test | 1 pass; no upstream command/context bundle |
| Generated NSIS script test | 1 pass; separate main binary/identifier, no CLI |
| `npm run lint` | pass; UI spec79files and i18n checks |
| `npm run build` | pass; upstream entry still builds |
| `npm run build:sds` | pass; static SDS entry, local assets |
| `npm run sds:check` | final no-default-features check exit0, compile20.44s |
| `cargo test --manifest-path src-tauri/Cargo.toml --bin skills-manager-cli` | 6 pass, source moved without content edits |
| Docs integrity | 104 docs match raw Git blob bytes; all57 snapshot asset SHA256 values match archived manifest |

An independent early review found short CLI alias overrides, inherited frontend fixture flag and Cargo-check config drift. Each was reproduced by a failing test, fixed, and verified. The reverse-profile bootstrap case was also corrected and tested. Full pilot code review at A10 remains outstanding.

## Native debug installer smoke

Command: `npm run sds:build -- --debug --bundles nsis`.

Artifact: `src-tauri/target/debug/bundle/nsis/SDS Skills Manager_1.42.0_x64-setup.exe`.
Size: 9,209,769 bytes. SHA256: `25cdd06099a1f95fa9725c3837c856040df07729fe88af918bd7d6be340c6001`.
This is an unsigned **debug shell installer**, not the Phase1 pilot/release installer.

Silent per-user install exit0 into task-owned `.superpowers/sdd/2026-10-16-sds-app-phase0-1/installed-shell`.
Installed files: `sds-skills-manager.exe`, `uninstall.exe`; no `skills-manager-cli.exe`.
Windows UI Automation observed the installed SDS window, Internal environment, disabled Store/Library/Settings and setup content; no mismatch alert. Light/dark theme was visually inspected. Installed app was closed, absence of its process/window verified, then reopened; dark theme persisted. Upstream installed process remained alive under its own original path/PID.

First NSIS attempt encountered OS error32 (file in use); closing the smoke executable and retrying produced a bundle. Subsequent generated-script inspection revealed the auto-discovered CLI and same process basename; neither earlier artifact was installed. Only the isolated artifact whose script checks pass was installed.

## Decisions and practical limits

- Worktree native create/attach failed because the chat starts outside the nested repository and the fallback checkout is not managed by Codex. Use the explicit workspace path above; no branch merge/push has occurred.
- Debug NSIS is an early packaging smoke. Release packaging, min-window overflow and complete native keyboard acceptance remain open before closing A02/Phase0. Native Tab observations reported the WebView pane, so they do not prove focus reaches the skip link; React focus tests pass.
- Live upstream SQLite files were locked by the already-running upstream app during byte comparison. A complete no-change filesystem audit is inconclusive; do not claim all upstream files remained byte-identical. Storage roots/startup spies/command boundaries were verified separately.
- Rust upstream baseline still has OS1314 symlink-creation failure and the local Git-test timeout documented in phase0-baseline.md. Full upstream suite is not green. No Windows security/Developer Mode settings were changed and no tests were disabled.
- Manual worktree isolation and combined A01/A02 source commit preserve the original checkout. Cost: coarser commit separation and manual lifecycle. Explicit token/snapshot choice may need a later visual delta review. CLI path change is covered by its six default-bin tests.
- `.gitattributes` limits byte preservation to docs, avoiding CRLF changes invalidating snapshot hashes; source line-ending behavior is unchanged.
- Store companion S01–S04 is documented. Store source/database, real auth, package install/remove and AIPro acceptance have not been implemented or tested. All Time ranking still needs a business decision before S02.

## Resume

A00 baseline evidence captured with known failures. A01 source implemented and focused checks pass. A02 source and debug installer smoke implemented; release/native acceptance items remain above. A03–A10 and Store S01–S04 have not started. Keep the installed smoke directory while it is registered; uninstall it before deleting this plan's scratch workspace at final cleanup.

## Release/native follow-up — 2026-10-17 10:37–10:41 Asia/Saigon

Later checkpoint superseding the pending release items above. Source commit `72f3f59` includes A03 session and A04 catalog; the intervening `3a1bff8` only records the user's pilot ordering decision. Rust and dist-sds were held unchanged during this release build. Newly added development-fixture scripts and the unconnected InstallReview component are not included in this artifact.

`npm run sds:build -- --bundles nsis` completed exit0, Rust release compile7m44s. Environment internal, fixtures0, default CLI feature disabled. Raw transcript: [phase0-release-installer.txt](phase0-release-installer.txt).

Artifact: `src-tauri/target/release/bundle/nsis/SDS Skills Manager_1.42.0_x64-setup.exe`.
Size4,763,666 bytes; SHA256 `b3efba2e09ddeea048a42bf75819012b71d1b66a0dfcf9a85b0d064141516140`.

Generated release NSIS test passes with `SDS_INSTALLER_PROFILE=release node --test scripts/sds-installer.test.mjs`. The script sets currentUser/user execution level, SDS-specific binary/identifier, and excludes upstream CLI/public updater. Silent install over the prior debug shell exit0 into the same task-owned registered installed-shell directory. Files: main exe17,960,448bytes and uninstall.exe79,204bytes only. No upstream process was running or closed by the test.

The installed release started with Internal identity, preserved the previous dark preference, rendered actual Store settings, and closed/reopened normally. Native Tab showed the skip link's visible focus; Shift+Tab showed Save connection and Store URL focus despite UIA reporting only the containing WebView document. Screenshots: [skip focus](screenshots/release-keyboard-skip.jpg), [URL focus](screenshots/release-keyboard-url.jpg).

At the configured minimum client1100×640 (captured outer1102×672), heading/form/footer/buttons remained visible without horizontal overflow: [minimum layout](screenshots/release-min-settings.jpg). A corner drag did not resize the window, so the test closed SDS, saved its own window-state file, set width/height to1100/640, reopened and inspected the actual restored native window. Original1440×860 state was restored after closing. No upstream configuration was edited.

Entering `http://127.0.0.1:8080/skm` into the installed Internal build and clicking Save returned the static validation error through real IPC. The profile was not saved and no auth/network request was made: [rejected HTTP](screenshots/release-internal-http-rejected.jpg).

Fresh before/after snapshots covered `%USERPROFILE%/.skills-manager`, `%APPDATA%/skills-manager`, `%APPDATA%/com.agentskills.desktop`, `%LOCALAPPDATA%/com.agentskills.desktop`, `%LOCALAPPDATA%/skills-manager`. All five were absent before and after. This proves those roots were not initialized by this smoke test; it does not test preservation of pre-existing upstream user data, which was no longer present when the fresh audit ran. Earlier locked-file audit remains inconclusive. Scratch manifests/script are retained under this plan's `.superpowers/sdd` directory.

A02 release startup, restart/theme, minimum layout and observed keyboard checks now have native evidence. Full upstream baseline is still NOT PASS. A04's standalone fixture/native catalog states, A05–A10 and Store integration/additive audit are still outstanding. The registered installed-shell directory now contains this release and must be uninstalled before deletion; the app was closed at the end of the test.
