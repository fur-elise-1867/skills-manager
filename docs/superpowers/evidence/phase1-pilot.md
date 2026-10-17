# Phase0–1 Windows acceptance checkpoint

2026-10-17. **Acceptance remains open:** real AIPro recognition, final upstream suite results and the pending action-time approval for removing the installed ZIP test deployment are not yet proven. No company-wide deployment or Phase2–5 completion is claimed.

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

The original1440×860 SDS window state was restored after the minimum test. Both installed resource trees preserved SKILL.md, binary data and the Unicode-named resource. The older debug fixture removal was separately approved and completed as documented in `phase1-removal-flow.md`; it does not stand in for the pending installed-release removal.

The native permission test exposed missing recovery guidance when `recoveryRequired` carried a different cause code. Commit9195cd6 adds a state-driven restart instruction without changing deployment behavior. Regression: one failed/35 passed before the fix, then36 passed/zero failed after it (`a10-recovery-guidance-{red,green}.log`); lint passed. Both final installers were rebuilt. The final development installer upgraded the owned installation with exit0; native startup retained Signed in and the real Store catalog. The removal review was cancelled without deleting the ZIP while its action-time question remains pending.

## Artifact inventory

Artifacts are preserved outside generated output under `.superpowers/sdd/2026-10-16-sds-app-phase0-1/artifacts`; generated NSIS filenames are reused between environments, so these copies matter.

| Source/environment | Bytes | SHA-256 |
| --- | --- | --- |
| 9b65a56 development, native-tested | 6023681 | 6f4b20296254f2218eeca15ef790eb234ed3755eedc266d2d19a9636bff5f526 |
| 9b65a56 internal, built | 6007632 | 6dbd54568ba6603e189bed235235268bfa71b00d044360a6deecf4db69d94221 |
| 9195cd6 development, built3m28s | 6007649 | 0e6c7f124e26ac79c39392578dc1e74f83d7d93308104d099d179c66f62a455d |
| 9195cd6 internal, built3m24s | 6014044 | 8de3022c85c20e2575d7a5aebc02c8d4a96cc3bfd3e20fc08e6946311d60f777 |

These are unsigned release builds with fixtures disabled. The development build permits only loopback HTTP; the internal build is HTTPS-only. WebView2 was already present on the pilot machine and provisioning is skipped by this NSIS configuration. The registered test installation must be uninstalled before deleting its retained task directory.

## Verification and limits of evidence

- Final SDS Rust consumer tests:106 passed, zero failed, four ignored (`a10-final-sds-rust.log`). The ignored entries are the separately executed live-provider/vault opt-ins and owned crash-child helpers; crash parent tests actually exercise those helpers. The live provider consumer test was explicitly run and passed as recorded in Store evidence; the vault opt-in is not claimed to have run here.
- Frontend36 passed after the recovery guidance correction. Lint, upstream frontend build, SDS check, six-schema/Rust-to-TypeScript contracts and real HTTP/Rust Store integration passed; logs are listed in the companion evidence.
- The two full Rust commands, default and `--features sds`, were launched and remain running through slow upstream Git fixtures. They are **not reported as passing**. Both have reproduced the existing symlink privilege failure. An independent exact test fails before product code at symlink creation with Windows1314 (`a10-baseline-symlink.log`). The partial-clone test also fails in setup before its product call because git cat-file reaches a deliberately absent promisor remote (`a10-baseline-missing-object.log`,26.16s). Both test source files are unchanged against upstream baselinee6dcedf. Additional uncompleted upstream tests remain unverified until these runners finish or are explicitly stopped and recorded.
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
| A08 Library/removal | Library and earlier fixture removal verified; installed-release removal awaits explicit UI deletion approval |
| A09 UI/IPC | Native Store→review→install→Library, errors, cancellation, minimum sizing/focus and final development installer startup verified |
| S01–S04 Store | Dedicated runtime, authenticated ACL facade, immutable publication, real consumer/provider contracts, additive audit and development installer connection verified |
| A10 delivery | Open until the remaining installed smoke/removal, final audit/results and actual AIPro recognition are recorded |
