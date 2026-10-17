# Phase 2 recovery gate evidence

[Crash matrix](../testing/phase2-crash-matrix.md) maps actual subprocess exits/kills, durable state and assertions for install, update, restore, remove and cache cleanup. The saved scoped log has150 passed/zero failed/seven ignored; five ignored helpers are executed by parent tests, two remaining integrations are opt-in. Both full suites also ran: default548 passed/three upstream failures/seven ignored, with SDS698 passed/the same three upstream failures/fourteen ignored. No test was disabled to hide those failures.

Recovery uses saved intent plus directory identities, hashes and committed DB evidence. It does not infer commit from UI progress, require Store login to reconcile a durable local intent, or auto-resume precommit downloads. Pre-intent review expires on a new runtime. Unknown current content/recreated roots remain RecoveryRequired with old/stage/current evidence preserved. A second reconciliation does not replay successful writes.

Four actual update/remove thread races assert a single winner. Cache cleanup rechecks a newly created pending reference after review. Selected-group retries create new reviews for failed items without replaying success. UI authoritative status/polling and stale context guards are covered separately in the57-pass UI suite.

These are isolated filesystem/SQLite/process observations. The installed native interrupted-operation restart and final independent two-axis review remain open, so this report does not declare G2 passed.
