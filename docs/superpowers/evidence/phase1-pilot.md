# Phase0–1 Windows acceptance checkpoint

2026-10-17. **Acceptance remains open only for actual AIPro recognition.** The user is preparing AIPro on this machine. Installed ZIP/7z, approved scoped removal, failure/recovery states, final builds and full suite outcomes are recorded below. No company-wide deployment or Phase2–5 completion is claimed.

## Installed release and Store integration

The fixture-free development installer from9b65a56 built successfully in3m59s and installed with exit0 into the task-owned registered `installed-shell` directory. The directory contains only the SDS executable and uninstaller. The generated installer regression passed and excludes the upstream CLI. The installed app displayed Local development, accepted `http://127.0.0.1:8080/skm`, signed in with the dedicated Store's synthetic account and rendered the real catalog in alphabetical order. Store S01–S04 evidence is in `store-baseline.md`, `store-catalog.md` and `store-pilot.md`.

The native Windows folder picker selected the existing task-owned `a10-target` directory. This custom destination is deliberately separate from a real AIPro profile. No real AIPro recognition is inferred from it.

| Native UI case | Observed result |
| --- | --- |
| ZIP1.0.0 / required jira-mcp | Missing configuration warning; Install anyway completed operation `b0b880f5-0876-4217-8aef-5853723fb013` |
| 7z1.0.1 / unknown requirements | Explicit unknown warning; Install anyway completed `ff68549d-80a2-4cf9-8332-0a5ac99c228a` |
| Library after restart | ZIP and7z remain Downloaded with Installed targets; cached1.0.2 is Downloaded with zero targets after the permission failure |
| Cancel prepared1.0.2 review | Review closed and no destination created |
| Package unavailable | Only the new synthetic1.0.2 package was temporarily moved aside with bytes preserved; `a19b55ca-209c-408e-9ec0-fd5692db00c2` displayed a failure and created no target. The package was restored and its original SHA-256 verified |
| Destination permission denied | Temporary non-inherited create-directory denial on the exact owned acceptance root made `928c2f7a-d2fe-4c26-aebb-61807aa64098` stop before rename, retain staging and require recovery. Original SDDL was restored exactly; no destination was created |
| Startup after permission failure | Existing deployments/session preserved; the uncommitted target remained absent. Recovery retained staging evidence rather than silently installing |
| Minimum window/light theme | Saved own window state set to client1100×640 while closed; actual native capture1102×672. Catalog had no horizontal overflow, detail actions were reachable through vertical scrolling, and install/result dialogs fit. Tab showed the skip link with a visible focus outline |
| Approved installed ZIP removal | After the user's specific action-time approval, the path was reviewed again and removal `4214312c-5f11-4667-9b60-c1d67017e006` completed. ZIP remained Downloaded with zero destinations; the skills root and all three 7z files remained unchanged |

The original1440×860 SDS window state was restored after the minimum tests. All six installed files were independently compared against the original archive streams by relative path, size and SHA-256 before the approved removal (`a10-native-resource-comparison.json` in the private ledger). The later removal left all three 7z hashes unchanged. The task's custom destination override was cleared through Settings/Use default destination; AIPro global currently reports not ready and no AIPro configuration was created or edited. No removal approval remains pending.

The native permission test exposed missing recovery guidance when `recoveryRequired` carried a different cause code. Commit9195cd6 adds a state-driven restart instruction without changing deployment behavior. Regression: one failed/35 passed before the fix, then36 passed/zero failed after it (`a10-recovery-guidance-{red,green}.log`); lint passed.

The final visual audit compared mapped W03 `7dcd2666d80d43dd8ae20556de775abc` HTML and screenshot directly with the installed app. It found MCP incorrectly placed in the right column and a fixed-width aside. Commit `eb00287` restores the left description/release-notes/compatibility stack, a separate MCP panel below it, 1.5:1 columns, and a framed package panel with Format/Size/Version/Install-folder rows and a full-width install button. Existing metadata, unknown/empty MCP handling, permission gating and callbacks are unchanged. All36 UI tests, lint and frontend build passed again. Independent source/snapshot review found no consequential issue. The new installed release was inspected at actual1102×672: the package action is visible, both columns wrap correctly, and MCP remains below the description on the left. The Library table/removal dialog also fit at this size with vertical scrolling.

Both `eb00287` installers were rebuilt and preserved. The development installer upgraded the owned installation with exit0 and retained the authenticated Store session and local Library. All actual installed removal checks above ran on this latest build. The final launcher/fixture/bundle/installer Node suite passed11 tests (`a10-stitch-node-tests.log`).

## Artifact inventory

Artifacts are preserved outside generated output under `.superpowers/sdd/2026-10-16-sds-app-phase0-1/artifacts`; generated NSIS filenames are reused between environments, so these copies matter.

| Source/environment | Bytes | SHA-256 |
| --- | --- | --- |
| 9b65a56 development, native-tested | 6023681 | 6f4b20296254f2218eeca15ef790eb234ed3755eedc266d2d19a9636bff5f526 |
| 9b65a56 internal, built | 6007632 | 6dbd54568ba6603e189bed235235268bfa71b00d044360a6deecf4db69d94221 |
| 9195cd6 development, built3m28s | 6007649 | 0e6c7f124e26ac79c39392578dc1e74f83d7d93308104d099d179c66f62a455d |
| 9195cd6 internal, built3m24s | 6014044 | 8de3022c85c20e2575d7a5aebc02c8d4a96cc3bfd3e20fc08e6946311d60f777 |
| eb00287 development, latest/native-tested, built3m06s | 6005743 | bdb41260ec4bd3c92140f0267ffc3c59df5d83dd2bf5617c2b87efe1ad30a9f8 |
| eb00287 internal, latest, built2m49s | 6007828 | 3c054e393390ff704f096af7ef916713fc1449b7e8e9af6966b7dcafa8568b8e |

These are unsigned release builds with fixtures disabled. The development build permits only loopback HTTP; the internal build is HTTPS-only. WebView2 was already present on the pilot machine and provisioning is skipped by this NSIS configuration. The registered test installation must be uninstalled before deleting its retained task directory.

## Verification and limits of evidence

- Final SDS Rust consumer tests:106 passed, zero failed, four ignored (`a10-final-sds-rust.log`). The ignored entries are the separately executed live-provider/vault opt-ins and owned crash-child helpers; crash parent tests actually exercise those helpers. The live provider consumer test was explicitly run and passed as recorded in Store evidence; the vault opt-in is not claimed to have run here.
- Frontend36 passed after the recovery guidance correction. Lint, upstream frontend build, SDS check, six-schema/Rust-to-TypeScript contracts and real HTTP/Rust Store integration passed; logs are listed in the companion evidence.
- Both full Rust commands finished normally with exit101 and the same three upstream setup failures. Default:548 passed/3 failed/7 ignored,2558.46s (`a10-default-tests.log`). `--features sds`:654 passed/3 failed/11 ignored,2560.76s (`a10-sds-features-tests.log`); this includes106 SDS passes and zero SDS failures. These suites are **not reported as passing**. Two symlink setup tests fail with Windows1314 before product logic (`a10-baseline-symlink.log`, `a10-baseline-second-symlink.log`). The partial-clone test fails its setup assertion before the product call because git cat-file reaches the deliberately absent promisor remote without the asserted word in its diagnostic (`a10-baseline-missing-object.log`,26.16s). All three test source files are unchanged against baselinee6dcedf. No test was disabled or product behavior changed to hide these failures.
- A scoped30-second process/TCP observation during the installed flow saw only SDS plus its WebView2 descendants and loopback Store traffic127.0.0.1:8080. No Git/CLI children or public connections were observed in that interval (`a10-runtime-observation.json` in the private ledger). This bounded sample is not a claim about all possible future network traffic.
- Before/after snapshots of five upstream application data roots matched exactly; all remained absent. This proves no initialization for those roots, not preservation of pre-existing data that was absent.
- Store commit `b5a7c69` contains only37 verified baseline-new owned files. Its `--only` commit preserved the exact pre-existing staged entries; no user source was staged. The post-commit audit compared1006 raw files, excluding Git bookkeeping, and passed with400 allowed additions including build output and86 new desktop schema objects. Original source/schema definitions remain unchanged. Raw final checkpoint inventories are `store-{files,schema}-a10-checkpoint.json` in the private ledger; audit again after any remaining acceptance mutation.

## Acceptance ledger

| Work | Status/evidence |
| --- | --- |
| A00 baseline/input preservation | Captured; known upstream environment failures remain explicit in `phase0-baseline.md` |
| A01–A04 runtime/shell/auth/catalog | Implemented and tested; native internal shell and current real-provider development connection evidenced |
| A05 target/MCP | Implemented; native missing/unknown/empty handling and earlier fixture recheck verified; actual AIPro still pending |
| A06 packages | Verified bounded ZIP/7z, adversarial tests, immutable real-provider bytes and native installs |
| A07 operations/recovery | Journal/context/root guards and crash tests; native permission failure/restart did not commit a target |
| A08 Library/removal | Library/restart and both fixture and specifically approved installed-release removal verified; downloaded bytes and other target preserved |
| A09 UI/IPC | Native Store→review→install→Library, errors, cancellation, minimum sizing/focus and final development installer startup verified |
| S01–S04 Store | Dedicated runtime, authenticated ACL facade, immutable publication, real consumer/provider contracts, additive audit and development installer connection verified |
| A10 delivery | Installed Windows flow, final artifacts/results and scoped removal verified; actual AIPro recognition remains open while the user prepares AIPro |
