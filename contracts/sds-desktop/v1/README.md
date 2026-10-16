# SDS desktop contract v1

These synthetic fixtures describe the existing Store auth response and the additive desktop facade. Consumer and provider tests use this directory. Fixtures are test/development assets; runtime does not silently fall back to them.

Auth login keeps the Store's flat `data.userId`, `userName`, `accessToken`, `refreshToken` response. No `expiresIn` or nested `user` is required. Rust maps identity to a string ID/displayName and never sends token fields through IPC. Refresh rotates both tokens; the legacy `token` alias is tolerated.

All routes are relative to the configured Store base, including its `/skm` context path. A leading slash on `downloadPath` does not discard that context. Production Store profiles require verified HTTPS; development HTTP is restricted to loopback.

`types.json` is the wire schema. Catalog/release definitions are added with the slices that introduce them. Unknown response fields remain compatible; missing required values produce explicit errors. Error bodies and credentials are not copied into logs or UI errors.
