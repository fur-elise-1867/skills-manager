# SDS Skills Manager pilot

This pilot covers the Windows SDS app, authenticated internal Store catalog, verified ZIP/7z release installation to AIPro global skills, local Library and scoped removal. The approved design baseline is `2026-10-16-stitch-01`; English is the default language.

## Build and install

Use Node with the checked-in lockfile, Windows build tools and Rust **1.97.1 or later**. Run `npm ci` on a fresh checkout. Run `npm run sds:build -- --environment development --bundles nsis` for the loopback Store acceptance build. Run `npm run sds:build -- --environment internal --bundles nsis` for an internal HTTPS-only build. Both are release builds with fixtures disabled. The default environment is internal; the frontend cannot change it.

The generated installer is `src-tauri/target/release/bundle/nsis/SDS Skills Manager_1.42.0_x64-setup.exe`. Preserve a copy and SHA-256 before building another environment because the output name is reused. Check the acceptance evidence for the exact tested artifact. The installer is not claimed to be signed. This pilot's NSIS config skips WebView2 provisioning, so the test machine must already have a functioning WebView2 runtime.

Install for the current Windows user. Open SDS Skills Manager, open Settings, enter the full Store base address including `/skm`, then save. Development permits HTTP only on loopback; internal requires a trusted HTTPS certificate. Changing Store signs out the current session. Sign in with a Store account supplied for that environment; do not paste tokens or passwords into logs or tickets.

## Local Store runtime

The Store additions are in `skills-store/skill-management/backend/desktop-pilot` and the new Java `desktop` package. Read that directory's README. They reuse existing authentication/ACL and only add desktop source/schema objects. The provided guards refuse shared/default/remote database URLs or missing/mismatched dedicated database markers before startup/publication.

Pilot base: `http://127.0.0.1:8080/skm`; dedicated PostgreSQL loopback port15432/database `sds_desktop_pilot_20261017`. Its task-owned data/storage and private process environment driver are retained under `.superpowers/sdd/2026-10-16-sds-app-phase0-1`. Do not delete that directory while its database or registered test installation is in use. Restart the own database with its exact data directory and explicit `-o "-h 127.0.0.1 -p 15432"`; then load the private environment and call the guarded `desktop-pilot/start.ps1`. Never stop the existing PostgreSQL service or reset a shared database. Stop the own Store process after verifying its identity, then use `pg_ctl -D <exact-own-data-directory> -w stop` for this cluster only.

Publication uses the guarded local `publish.ps1`, not an anonymous HTTP endpoint. It creates an immutable release/package without changing business skill versions. A conflicting ID/sequence fails. An identical retry verifies stored metadata and bytes. Failed publication may leave a newly created unreferenced package for manual review.

## Install and inspect

1. Browse All Time or combine text and source filters. All Time temporarily orders by name A–Z, case-insensitive, with skill ID as tie-break before pagination. Trending and Hot show unavailable.
2. Open a released skill and review its version, requirements, exact destination and current MCP report. Null requirements mean unknown; an empty list means none required. Check again rereads configuration. The app never configures or starts an MCP server.
3. AIPro global defaults to `%USERPROFILE%/.aipro/skills`. Prepare AIPro on this machine before acceptance. A custom existing local skills folder can be configured in Settings; it does not change the global MCP configuration location.
4. Confirm the reviewed installation. The app downloads authenticated original bytes, checks digest/size, validates and extracts to owned staging, then records and commits the copy. It never runs package scripts. Existing unmanaged content or local edits block replacement.
5. Library shows downloaded releases separately from installed/modified/missing targets. Preview reads verified local Markdown. Copy and Reveal use the recorded destination. Settings provides the local logs folder.
6. Remove from target reviews one managed deployment and preserves its downloaded Library release, other targets and MCP configuration. Local edits block removal; unexpected extra files are retained and reported. A recovery-required result needs startup recovery and log review before another attempt.

SQLite/cache/logs live under `%LOCALAPPDATA%/SDS/SkillsManager`; explicit debug fixtures use a separate `development-fixtures` root. Credentials use the Windows credential vault and never frontend/SQLite/log storage. Codex-launched tests can encounter Windows package data redirection; verify the actual destination shown by the independently installed app and AIPro.

## Verification and acceptance

Run `npm run lint`, `npm run build`, `npm run test:sds`, `npm run sds:check`, the Rust suites and `node scripts/test-sds-contracts.mjs`. For an explicitly running local provider, set pilot credentials only in the process environment and run `node scripts/test-sds-provider.mjs`; its companion opt-in Rust test exercises real downloading and package preparation. These checks do not prove AIPro recognition.

Installed Windows acceptance must include login, Store/detail, MCP missing/recheck/acknowledgement, ZIP and7z, Library, removal, error/cancel/restart states and actual AIPro recognition. Follow `docs/superpowers/evidence/phase1-pilot.md` when available for current verified/pending results. Preserve Store before/after file and schema inventories and run the additive audit after the final test. Never report the pilot accepted from file presence alone.
