# Store fixtures

Synthetic development data only. No employee credentials or real MCP tokens.

The committed ZIP archives contain a root SKILL.md; excel-insights.7z contains one matching wrapper. Source text is in sources/. The mock server computes archive SHA-256 and size from the actual committed bytes, and computes an opaque strong ETag from the archive hash and full description. No sample hash is treated as a verified archive hash.

Demo login: employee01 (or employee02), password demo-password. Sessions are in-memory on the mock server and are invalidated by logout or server restart.

Production uses the native Rust HTTP client and the same five API endpoint contracts. The mock server is never launched automatically by the app, and production errors do not switch to fake data.
