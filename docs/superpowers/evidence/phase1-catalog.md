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

## Native fixture follow-up — 2026-10-17

The standalone development runtime is now implemented. `npm run sds:dev -- --fixtures` starts an ephemeral literal-loopback HTTP server owned by the launcher; both identity flags must agree. Installer modes reject fixtures, including debug installers. Inherited fixture flags/URL cannot opt the app in. Debug-only Rust uses a memory credential adapter, a separate `development-fixtures` data root, an immutable loopback Store profile and no proxy. Normal Store failures cannot select fixtures. The UI banner labels sample data, shows only synthetic sample credentials and locks that development Store address.

Public service tests first failed on the missing fixture API, then reproduced fixture/live data-root collision. After isolation, SDS Rust tests30pass/1opt-in vault test ignored, UI20pass (including2in-progress A05 review tests), Node9pass, lint92files and TypeScript/Vite/check exit0. Logs `a04-fixture-runtime-{red,root-red,green}.log`, `a04-fixture-{node,ui,lint,build}-final.log`, `a04-fixture-check.log`. The opt-in Windows credential test was separately passed in A03; fixtures deliberately never use it.

Native dev build compiled in1m08s and signed in with the public synthetic account through real Tauri IPC and Rust reqwest HTTP. Inspected archived W02/W03 layouts alongside these captures: [catalog loading](screenshots/fixture-catalog-loading.jpg), [six sample cards](screenshots/fixture-catalog.jpg), [detail/release/MCP](screenshots/fixture-detail.jpg), [empty](screenshots/fixture-catalog-empty.jpg), [503 outage without fallback](screenshots/fixture-catalog-outage.jpg). Search for Jira reduced the result to one; Back to results retained that query. Missing compatibility remained unknown and absent releases had no install action. Remote Markdown images were absent and unsafe links were text. Catalog/detail install buttons stay disabled until operation integration.

The native outage inspection exposed a weak generic connection badge. Afterward it was changed to explicit Offline/Session expired text, and a new failing UI test established that a successful request after an offline state must refresh the session badge. That correction passes the final UI suite; a later native pilot run must recheck the updated badge. The five captures otherwise show the inspected runtime before that text-only follow-up.

The fixture app exited normally, launcher session97779 exit0 and port1420 stopped listening. Only its separate logs directory/file was present in the fixture data root; all sample access/refresh tokens and the sample password were absent from that log. No employee credentials, real AIPro folder, Store source or database were used. These checks close the A04 explicit-fixture/native-state gap; production provider/schema parity and real Store integration remain S01–S04/A10 work. JSON Schema validation is still pending that contract integration gate.
