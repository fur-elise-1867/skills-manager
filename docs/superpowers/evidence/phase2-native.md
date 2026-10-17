# Phase 2 native execution evidence

Ownership: native SDS lifecycle except store_client.rs / updates_metadata.rs. No commits. Existing .gitignore and dist-sds changes preserved.

Ruling: retain PreparedOperation v1 wire shape, persist lifecycle options/context separately; keeps pending pilot intents readable. InstallRequest target is additive/default global. Schema migrations 002..007 are separate transactions; original 001 unchanged.

Initial migration test invocation blocked by parent-owned metadata module not yet present; not counted as behavior RED.

P2.1 RED: migrations 1 failed/1 passed, schema observed1 expected7. Compiled native workspace test: target discarded on decode (Null vs selected project). Implemented additive migration dispatcher and typed project registry/resolver.

P2.2 parent RED: depth33 snapshot test process STATUS_STACK_OVERFLOW with walk recursion reserving64KiB stack buffer each frame. Fixed narrowly: heap buffer, depth32 unchanged. Snapshot GREEN prior essential2/2; workspace GREEN2/2. Pending rerun for extra depth boundary.

P2.7 actual RED: item_access_denial_then_outage_cannot_reuse_old_cached_grant failed (8other lifecycle tests passed): cached plan was accepted after403 then outage because SessionService maps403 to ACCESS_DENIED. Added actual alias to scoped grant revocation. Peritem metadata Unavailable/UnknownRelease already invalidate only matching skill/account grants.
Ruling: grouped selection items add optional operation frozen payload because UI must render peroperation MCP/release review before dispatch; operationId/error shape preserved.

P2.7 regression RED then fix: repeated offline cached install of exact pristine managed release returned REVIEW_REQUIRED because shared lifecycle review incorrectly required deploymentId on install plans. Reused managed_install identity/fingerprint verification for install-kind plans. Latest affected run beforefix:12pass/1fail/1ignored; item403 and peritem unavailable/unknown grant revocation now GREEN, real cachecleanup exits at5boundaries GREEN, selected failure freshretry GREEN.
