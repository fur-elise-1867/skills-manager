# Phase 1 — operation and Library foundations (A07 in progress)

Date: 2026-10-17. No operation service, journal recovery or actual AIPro installation is claimed complete here.

Implemented foundations:

- A content manifest/fingerprint includes file bytes, relative paths and empty directories. Native Windows junctions at roots, ancestors or children are refused. Existing directory handles are kept open without delete sharing during reads; content files are opened without following reparse leaves and without write/delete sharing. Deployment must still recheck fingerprints under its target/process lock at commit.
- A dedicated `pilot.sqlite3` inside the supplied SDS root has versioned release/deployment/operation/catalog tables, foreign keys and full synchronous SQLite commits. The existing upstream DB is not opened or migrated. Runtime does not initialize this repository yet; all current tests use temporary roots.
- Release cache identity is hashed from Store profile + skill ID + release ID. Content is copied and rehashed before publication. Repeating the same immutable release reuses it; a changed metadata body is refused. Reopening verifies stored metadata identities, manifests and actual content. Local cache edits and path-key tampering fail closed. A process lock protects cache writes.
- Frozen operations persist release, exact target, expected fingerprint, MCP report, account/profile/generation and runtime epoch. No credential fields are serialized. A short status record persists independently; terminal outcomes cannot be overwritten by earlier progress. Generic status updates cannot mark success: successful deployment must later commit its Library row and completed result together.

Focused development seam tests were observed red before the fingerprint/repository APIs existed, then green. A malformed initial status was accepted in a regression red run and is now rejected before persistence. The latest focused SDS run (`a07-foundation-rust-final.log`) passed **65 tests**, with one explicitly opt-in Windows credential integration test ignored. Six tests cover the new foundation, including restart/idempotent cache, content tampering, database key tampering, manifest parity with package preparation, local changes/empty directories, native junctions and persisted frozen operations.

This is a partial A07 checkpoint. Required next work: operation preparation/execution using real Store/session seams; immutable cache authorization; target/process locks; durable intent before filesystem rename; atomic deployment/result commit; crash reconciliation with file identity and safe path validation; cancellation and real process-termination tests. Prepared records surviving restart are historical state, not authorization to execute: the operation service must refuse records from an earlier runtime epoch until reviewed again. A07 independent review remains pending.

A05 native minimum-window Settings/dialog inspection remains pending A09, following the user's earlier Computer Use stop. Store S01–S04 and real Windows/AIPro pilot acceptance remain unimplemented gates.
