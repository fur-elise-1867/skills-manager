# Phase 2 update metadata consumer

`StoreClient::check_updates` is metadata-only. It captures one SessionContext for all chunks, deduplicates exact skill/release pairs, preserves input order, sends at most100 pairs per POST and skips HTTP for empty input. SessionService retains authentication/ACL/outage semantics and rejects stale account/profile generations. No target path, fingerprint, MCP configuration or credential is present in the request DTO.

Each response must have a valid RFC3339 checkedAt and exactly one result for every requested pair. Unknown/duplicate/missing/mismatched pairs, unknown status, invalid release references and metadata disclosed for unavailable/unknown_release fail `INVALID_RESPONSE`. Auth/ACL errors remain errors rather than up_to_date or cached fallback.

## Evidence

- Initial compile RED: new public `check_updates` method absent. Initial ACL expectation was corrected from FORBIDDEN to the existing product's ACCESS_DENIED code; this was a test expectation correction, not a product change.
- Scoped Rust metadata suite: **7 passed**, zero failed. Includes empty request, 101 unique + duplicate chunking, same skill with two current releases, malformed/leaking responses, invalid IDs before HTTP, fixture serde shape and late response after logout.
- `node scripts/test-sds-updates-contracts.mjs`: passed four statuses, response privacy, 1/100 request acceptance, 0/101 and duplicate rejection.
- `node scripts/test-sds-contracts.mjs`: passed seven wire fixtures and actual Rust serde → TypeScript parity, including UpdateCheck. The runner fails if the Rust emitter does not actually run.
- Fixture endpoint regression: first failed HTTP405 versus required401; after implementation both fixture tests passed. The endpoint now supports authenticated metadata checks with the same request bounds.
- `scripts/test-sds-updates-provider.mjs` exercised through a fresh synthetic loopback fixture: passed one authorized release,100-result boundary,0/101/duplicate rejection, unauthenticated401 and denied metadata privacy. **This is a fixture harness check, not evidence against Spring Store.** The dedicated real-provider run is recorded separately.

The live harness requires credentials in process environment/memory and logs only counts/status outcomes. It restricts its target to HTTP127.0.0.1 `/skm/`, uses request timeouts, validates wire schema and logs out in finally. It never downloads packages.
