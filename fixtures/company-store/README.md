# Store fixtures

Synthetic development data only. No employee credentials or real MCP tokens.

The committed ZIP archives contain a root SKILL.md; excel-insights.7z contains one matching wrapper. Source text is in sources/. The mock server computes archive SHA-256 and size from the actual committed bytes, and computes an opaque strong ETag from the archive hash and full description. No sample hash is treated as a verified archive hash.

7z decoder fixtures use COPY (`excel-insights-copy.7z`), LZMA (`excel-insights-lzma.7z`) and LZMA2 (`excel-insights.7z`). The compressed fixtures contain SKILL.md plus a resource in one solid block. These were created with 7-Zip 19.00 for tests; native installation uses the bundled Rust decoder.

Demo login: employee01 (or employee02), password demo-password. Sessions are in-memory on the mock server and are invalidated by logout or server restart.

Production uses the native Rust HTTP client and the same five API endpoint contracts. The mock server is never launched automatically by the app, and production errors do not switch to fake data.

## Gói cập nhật L5

`packages/jira-project-summary-v2.zip` là ZIP thật của cùng skill Jira, gồm SKILL.md/report đã sửa và tài nguyên mới. Mock chỉ chọn gói này khi revision > 1, SHA/size/ETag được tính từ bytes thật. Dùng `--revision-file` trong integration tests hoặc `server.setRevision(2)` trong native smoke; không có endpoint admin bổ sung trong API draft.
