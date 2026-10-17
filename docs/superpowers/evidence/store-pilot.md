# Store S03/S04 integration checkpoint — 2026-10-17

The new Store desktop package now serves immutable release metadata and authenticated original ZIP/7z bytes. Every release/package request rechecks the existing visibility and download rules. New release/package/source tables, constraints, functions and triggers belong exclusively to `desktop_*`; the separate migration history follows upstream bootstrap. No existing source file, business version or schema object was modified.

Actual local operator publication on the dedicated database:

| Skill | Release | Format | Bytes | SHA-256 |
| --- | --- | --- | --- | --- |
| 1 | 8b523c68-f7e5-4ea6-a1a2-cdfd4593c105 | ZIP | 502 | 77ba4db5f932142a372cdb04204ee730479e9cf835663665c999bf6cc8c65a40 |
| 2 | 94de5a6f-5e44-4770-9e86-6a8da12d7fe5 | 7z | 369 | d7a52ca765e24c3f233c1f901786958e5ad50b9c0ea0c2cebebffc0598efe27a |

ZIP metadata requires `jira-mcp`; the first 7z release explicitly requires none. Concurrent publications for skill2 sequence2 produced exactly one persisted release, `ddd574f0-4ec0-4501-9d87-e7ab8ffbe65a`, with unknown/null MCP requirements. The other transaction failed the unique skill/sequence constraint. Its newly created unreferenced package was retained for review. The original sequence1 metadata and bytes remain available unchanged. An identical ZIP retry returned unchanged; a changed-metadata retry was refused. Four real PostgreSQL UPDATE/DELETE attempts against immutable package/release records were refused in rolled-back transactions.

The publisher's first real query exposed psql's lack of variable substitution in `-c` SQL. Queries now use `-f` script input with quoted bound variables; temporary scripts contain placeholders only. Real HTTP then exposed Spring environment binding overriding the package subdirectory: `desktop.pilot.storage` collided with `DESKTOP_PILOT_STORAGE`. The service/profile/tests now use `desktop.pilot.package-root`. The same live HTTP check then passed for both archives.

Verification:

- Full Store suite: 127 passed, zero failures/errors/skips, 30.182s (`s03-store-full-tests.log`). After the configuration correction, all five focused release tests passed again (`s03-package-root-tests.log`), including real Windows deny-write/delete sharing while streaming.
- `node scripts/test-sds-provider.mjs`: live auth/refresh, JSON Schema catalog/detail/taxonomy/release contracts, original ZIP/7z bytes, length/MIME/disposition/no-store/SHA-256, unauthenticated401, invalid400, unavailable422 and missing404 passed (`s04-live-provider.log`). Credentials/tokens stay in environment/memory and are not logged.
- The actual Rust `StoreClient` and `SessionService` connected to the real provider, downloaded both archives and passed product package validation/extraction with identical fingerprints (`s04-live-rust-consumer.log`: one explicit opt-in test passed). No real target or persisted vault was used for this check.
- Publisher validation, five database guard refusals and additive audit negative cases passed. Real audit: 937 files compared with537 original entries; 400 allowed additions including build output and86 desktop schema objects. All original source/schema definitions unchanged. Git bookkeeping alone is excluded; raw inventories are retained under the ignored task ledger as `store-{files,schema}-s03.json`.
- Both independent reviewers found no consequential S03 findings; their reviews were read-only and did not substitute for runtime tests.

Runtime handoff: loopback `http://127.0.0.1:8080/skm`, own PostgreSQL18.1 cluster at port15432/database `sds_desktop_pilot_20261017`, marker `SDS_DESKTOP_PILOT_ISOLATED_20261017`. Secrets remain only in ignored task runtime files. Restart the cluster with `pg_ctl -D <ledger>/postgres-pilot/data -o "-h 127.0.0.1 -p 15432" -l <ledger>/postgres-pilot/restart.log -w start`; the explicit port/address are required because they were command-line options, not server defaults. Then dot-source the ignored `store-env.ps1` and run the guarded `desktop-pilot/start.ps1`. Current foreground Store handle is8620, startup log `s03-store-package-root.log`; inspect process identity before stopping. Stop only this task's cluster with its exact `-D` directory, never the existing PostgreSQL service.

S04's HTTP/consumer/audit checks are proven. Fresh development installer connection and the A10 installed app/real AIPro acceptance are still pending; this checkpoint does not claim Phase0–1 completion. A09 minimum-window/focus/error native checks also remain pending. The user will prepare AIPro before acceptance.
