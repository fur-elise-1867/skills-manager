# A04 — Catalog/detail implementation checkpoint

Date: 2026-10-17 Asia/Saigon. Worktree: `D:/Documents/Git Project/skills-manager-app/.worktrees/sds-phase0-1`.

## Implemented

The SDS Store gateway now calls authenticated desktop taxonomy/search/detail/release routes, preserving the Store context path. Search sends q + mode + sourceId together and uses one-based pagination (20 default, 1–100 page size). The client does not invent ranking or sort a partial page. Wire detail fields are flat; the adapter returns the domain `{summary, storePageUrl, latestRelease}` shape. IDs in detail/release responses must match the requested resource. Missing compatibility or latestRelease stays unknown/absent.

SessionService now exposes a captured profile/account/generation scope. A protected request validates that scope before attaching credentials and again after receiving the response. Requests prepared before logout cannot run under a later login, and late catalog responses cannot become a newer account's result.

The UI has Store/disabled Local Install tabs, server-advertised mode availability, Source chips, 300ms search debounce/Enter submission, pagination, card metadata and skill detail with package/MCP/release information. Changing filters resets the page; returning from detail or Settings preserves the current query. Displayed results are bound to profile/account/generation/query. There is no persistent catalog cache yet; repository cache storage belongs to A07. No cache is used to infer permissions or installed state.

Markdown escapes/omits raw HTML, suppresses remote images and renders links as text. Compatibility on a specific release never falls back to catalog compatibility when that release's declaration is null. Installation remains disabled until the operation review service is connected in A05/A09. Saving an unchanged Store URL preserves the existing session, matching the backend's no-op semantics.

## Verification

| Check | Result |
|---|---|
| `cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --features sds --lib sds::tests` | Exit0; 28 passed, 1 opt-in Windows vault test ignored (separately passed in A03) |
| Catalog subset | 7 tests: query/path/detail mapping; pagination limits/defaults; 422/401/503 versus empty; missing release/compatibility; captured stale scope; late response; Unicode + pageSize100 |
| `npm run test:sds` | Exit0; 17 passed, including query/account stale-response handling, explicit error states, Markdown and release compatibility |
| `npm run lint` | Exit0; UI spec90files and i18n checks pass |
| `npm run build:sds` | Exit0; TypeScript + Vite bundle pass |
| `npm run sds:check` | Exit0; new catalog commands registered and runtime compiles |
| `node --test scripts/sds.test.mjs scripts/sds-bundle.test.mjs` | Exit0; 7 passed |
| Schema/fixture JSON parse | Schema and six fixture envelopes parse; full JSON Schema validation has not run |

Red/green evidence: the first gateway test failed on missing public API before implementation; UI tests failed before CatalogPage existed. Subsequent tests reproduced saving an unchanged URL signing out only the UI, release compatibility incorrectly falling back to catalog metadata, and byte-length limits incorrectly rejecting a 500-character Unicode query. Each was corrected and the relevant checks rerun. Local logs: `a04-gateway-red.log`, `a04-ui-red.log`, `a04-ui-races.log`, `a04-compatibility-red.log`, `a04-unicode-red.log`; final results in `a04-rust-final.log`, `a04-ui-final.log`, `a04-lint-final.log`, `a04-build-final.log`, `a04-check-final.log`, `a04-node-final.log`.

## Open acceptance

A04 source and focused checks are implemented; native visual acceptance remains open. The W02 archived screenshot informed card/toolbar/Source layout, but loading/empty/error and W03 still require rendered native inspection with an explicit fixture banner. The standalone opt-in fixture HTTP runtime has not been implemented; the current fixtures are served only by tests' real loopback HTTP server. Do not label A04 or Phase1 fully accepted from these checks alone.

No real Store facade/source/database change or actual employee authentication has occurred. On2026-10-17 the user explicitly selected the pilot All Time order: name A–Z, skillId tie-break. S02 must implement that server-side before pagination; this is a temporary pilot order, not a popularity ranking. Trending/Hot remain unavailable until their formulas are agreed. A02 release/min-window/keyboard/audit gates, A05–A10 and Store S01–S04 remain required. See phase0-runtime.md for upstream baseline failures and the registered old debug-shell install.
