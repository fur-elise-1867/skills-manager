# A03 — Store session and connection settings

Date: 2026-10-17 Asia/Saigon. Checkout: `D:/Documents/Git Project/skills-manager-app/.worktrees/sds-phase0-1`.

## Delivered behavior

The SDS runtime now exposes only its identity, Store settings and session commands. It uses real reqwest HTTP with redirects disabled, verified HTTPS and loopback-only HTTP in development builds. API URL construction retains the configured context path. Settings persist atomically under the separate SDS data root; changing profiles invalidates the previous session.

Identity crosses IPC as `SessionView`; access/refresh tokens remain in Rust. Only the remembered identity and refresh token are stored in Windows Credential Manager under an SDS-specific service/profile key. No plaintext fallback is available. A durable signout marker prevents restoration when OS credential deletion fails. Transport and credential error diagnostics are replaced by static messages before IPC serialization.

Concurrent 401 responses share one refresh, requests retry once, and 403 never refreshes. Generation checks reject late responses after logout or profile replacement. Network failure retains offline identity; rejected refresh expires the session. Logout clears local state even when server revocation fails. The frontend clears its password field on submit and does not import upstream account/state providers.

Existing Store sources use a flat LoginResponse (`userId`, `userName`, `accessToken`, `refreshToken`) without `expiresIn`. `AuthenticationServiceImpl` and `BusinessErrorCode` also establish that invalid passwords and missing users produce HTTP400. A failing desktop test first reproduced `INVALID_RESPONSE`; the adapter now whitelists those two machine codes as the same `AUTH_FAILED` message, without echoing Store diagnostics or exposing which credential was wrong.

## Verification

| Command | Observed result |
|---|---|
| `cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --features sds --lib sds::tests::session` | Exit0; 15 passed, 1 opt-in native integration test ignored |
| Same command filtered to `windows_credential_vault_round_trip` with `-- --ignored` | Exit0; 1 passed against actual Windows Credential Manager, unique synthetic entry cleaned up |
| `npm run test:sds` | Exit0; 10 UI/profile tests passed |
| `npm run lint` | Exit0; UI spec86files and i18n checks pass |
| `npm run build:sds` | Exit0; TypeScript and SDS Vite bundle pass |
| `npm run sds:check` | Exit0; effective SDS configuration and Rust runtime compile |

Session tests exercise the public service over a real ephemeral loopback HTTP server with synthetic credentials. They cover context-path routes, unsafe origins/traversal/redirects, actual auth wire shape, concurrent refresh, repeated unauthorized responses, offline/expired behavior, late refresh after logout/profile switch, restart restoration, persisted settings and credential failure diagnostic redaction. UI tests exercise settings-to-signin and password clearing through the IPC seam.

Raw local command outputs are in `a03-session-final.log`, `a03-native-vault.log`, `a03-ui-final.log`, `a03-lint-final.log`, `a03-build-final.log`, `a03-check-final.log`. Synthetic fixtures under `contracts/sds-desktop/v1` are deliberately separate from production data and do not provide automatic fallback.

## Limits and next checkpoint

These results establish A03 service/UI behavior, not real Store/native pilot acceptance. No Store source/database change or actual account authentication was performed. The previously installed debug shell predates A03. A10 must verify the rebuilt installer with real Store auth, package install/recognition/removal and recovery. Phase0 release/native acceptance items and the known upstream baseline failures remain recorded in `phase0-runtime.md` and `phase0-baseline.md`. A04 catalog work follows this checkpoint.
