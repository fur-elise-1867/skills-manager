# Phase 2 offline policy and evidence

| Session / item state | Local Library/snapshot/restore/remove | Cached Store deployment |
|---|---|---|
| Authenticated online | Available | Fresh Store authorization |
| Prior account/profile, current outage | Available | Explicit Use cached copy, verified bytes and persisted grant |
| Expired / SignedOut | Available | Refused; sign-in required |
| Online access denial / unavailable / unknown release | Available | Refused for that item; no outage fallback |

There is no invented offline TTL or claim that deployed files are revoked. Legacy cache without account grant stays viewable but cannot silently acquire authorization. Grants contain profile/account/release provenance, not tokens. Cached commit rechecks the exact outage session and persisted grant inside its guarded commit. Online Store commits retain current-session authorization.

Green lifecycle tests cover zero-HTTP cached deployment, restart remembering prior identity, logout/expiry/403 distinctions, reconnect invalidation, corrupted cache, unknown legacy grants, per-item unavailable/unknown revocation, ACCESS_DENIED403 followed by outage and pristine repeated cached install. Session tests cover late refresh/catalog context rejection. The 403 alias and repeated-cache bugs had observed failing regressions before fixes.

Native Store was stopped using verified owned process identities. Library displayed outage/unknown current rights and disabled updates. Legacy ZIP cache was refused for missing grant. Prior-authorized7z installed successfully into project-b with zero listening Store process; file hashes match the original release and project-a edits remained intact. Store restart and Retry connection restored online actions. See [native acceptance](phase2-native-acceptance.md). Native expired/403 presentation remains open; automated denial tests do not close it.
