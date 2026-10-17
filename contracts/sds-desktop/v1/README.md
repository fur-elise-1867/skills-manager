# SDS desktop contract v1

These synthetic fixtures describe the existing Store auth response and the additive desktop facade. Consumer and provider tests use this directory. Fixtures are test/development assets; runtime does not silently fall back to them.

Auth login keeps the Store's flat `data.userId`, `userName`, `accessToken`, `refreshToken` response. No `expiresIn` or nested `user` is required. Rust maps identity to a string ID/displayName and never sends token fields through IPC. Refresh rotates both tokens; the legacy `token` alias is tolerated.

All routes are relative to the configured Store base, including its `/skm` context path. A leading slash on `downloadPath` does not discard that context. Production Store profiles require verified HTTPS; development HTTP is restricted to loopback.

`types.json` is the wire schema. Definitions `catalog`, `detail`, `taxonomy`, `release`, `auth-login` and `auth-refresh` name the fixture envelopes. `detail` is flat on the wire; Rust maps it to the IPC domain shape `{summary, storePageUrl, latestRelease}`. Unknown response fields remain compatible; missing required values produce explicit errors. Error bodies and credentials are not copied into logs or UI errors.

Catalog queries combine q, mode and optional sourceId on the server, with one-based page/20 default pageSize (1–100). Ranking belongs to the server. The synthetic taxonomy advertises Trending/Hot unavailable, without choosing production ranking. Compatibility and latestRelease may be absent/null; neither implies installed or verified. Release package digest/bytes here are illustrative and MUST NOT be used for actual package acceptance. A06 supplies real archive fixtures and hashes.

For native UI development only, run `npm run sds:dev -- --fixtures`. The launcher starts an ephemeral loopback HTTP Store and couples the frontend/backend fixture flags. Use only the published sample account `employee@example.invalid` / `synthetic-password`. Search `__empty`, `__slow` or `__unavailable` for visual states. The app displays a fixture banner, uses `development-fixtures` data and an in-memory credential vault, and cannot change that session's Store URL. It does not use Windows credentials or silently switch from a real Store to fixtures. Neither release nor debug installer builds accept `--fixtures`. Closing the development app stops the fixture HTTP server.
