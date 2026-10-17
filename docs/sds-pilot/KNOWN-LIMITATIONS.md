# Pilot limits

- Phase0–1 acceptance is still in progress. Real AIPro recognition is pending preparation of AIPro on the test machine. Fixture and HTTP/Rust integration results do not satisfy that requirement.
- The pilot supports AIPro global copy deployment. Project destinations, three other agents, presets, selective update, snapshot/restore and full offline workflows remain in phases2–5 of the approved design.
- All Time is temporary alphabetical ordering, not a leaderboard. Trending/Hot require a later approved ranking formula.
- Missing compatibility or MCP metadata means unknown. MCP checks verify bounded local configuration presence only; they do not verify server health, credentials or agent behavior. The app never writes MCP configuration or executes package scripts.
- Unmanaged existing content and modified deployments are not overwritten. Recovery can require an app restart and manual inspection of reported retained content. Scoped removal keeps Library bytes and unknown extra files.
- Package maximum100MiB; expanded content maximum500MiB, 10,000 entries, depth32 and bounded decoder/header memory. Encrypted and multipart archives are unsupported. ZIP methods are Store/Deflate/Deflate64/Bzip2; other methods require repacking. SDS requires Rust1.97.1 despite the upstream package's preserved earlier rust-version declaration.
- The local development installer permits HTTP only on loopback. Internal distribution requires trusted HTTPS. Neither release build can enable fixtures. No company-wide deployment or signing certificate is supplied by this pilot.
- WebView2 must already be installed for this installer configuration. Test artifacts are unsigned unless separately signed and evidenced by IT.
- Store additions are kept separate from the team's existing dirty source snapshot. They require the dedicated guarded pilot database/profile; no existing Store source or schema object is rewritten. Failed concurrent publication can leave its newly copied unreferenced package for operator review.
- Existing upstream test failures must be recorded separately; a known upstream failure never excuses an SDS regression. See the baseline and final verification evidence.
