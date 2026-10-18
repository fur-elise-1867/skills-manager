# Skills Store API — Desktop contract draft

Version 0.2 · 2026-10-16 · **Đề xuất cho đội Skills Store, chưa phải API đã chốt.**

Revision 0.2: đồng bộ W02 từ Stitch; xem [nguồn và phân biệt quyết định/đề xuất](W02-CHANGELOG.md).
## 1. Nguyên tắc và khoảng cách với source hiện tại

Base URL do IT cấp, HTTPS nội bộ. Prefix đề xuất `/api/v1`; auth kế thừa endpoint đang có. Nhóm `/desktop/*` là facade đề xuất để không thay contract web đang phát triển. Store có thể triển khai trên endpoint chung nếu giữ được schema/ngữ nghĩa bên dưới.

Đã quan sát: login/refresh/logout; GET skills/search/detail/taxonomy; download từng file và download-all có auth. Download-all gom file đính kèm và có thể chứa ZIP con, nên **không được dùng mặc định làm install artifact**. File đính kèm và package cài đặt phải phân biệt.

`Skill.version` hiện tăng mỗi lần save. Đề xuất giữ nó làm metadata revision; thêm release ID/sequence/label và package bất biến. Không dùng thay đổi description để kích hoạt thông báo cập nhật skill.

## 2. Quy ước

- ID serialized thành string. Timestamps UTC ISO-8601; client hiển thị timezone địa phương.
- JSON success: `{ "success": true, "data": ... }`. Error: `{ "success": false, "errorCode": "...", "message": "...", "requestId": "..." }`.
- Bearer token từ Store; API bảo vệ trả 401 cho phiên không hợp lệ, 403 cho thiếu quyền, 404 cho không tồn tại/không được lộ. Không trả HTML login thay binary.
- Server phân quyền mọi catalog/detail/release/package request. `canDownload` trên UI là gợi ý, download vẫn re-check.
- Role nghiệp vụ `DEV/BSE/TPM/PM` là taxonomy để lọc. ACL dùng cơ chế Store hiện có.
- Agent keys đề xuất: `aipro`, `claude_code`, `gemini_cli`, `codex`. App có mapping display name.
- requiredMcps là unique array tên chuẩn phân biệt hoa/thường. `[]` = không cần; `null` = chưa khai báo. Trường này thuộc release. Không suy ra từ description.
- Release content, compatibleAgents, requiredMcps, installName và package digest bất biến sau publish. Chỉnh instruction hoặc dependency cần release mới; sửa mô tả catalog có thể chỉ tăng metadataRevision.

## 3. Endpoint matrix

| Method / route | Input | Output / semantics | Trạng thái |
|---|---|---|---|
| POST `/auth/login` | email, password | accessToken, refreshToken, expiresIn, user | Có endpoint; response mapping cần chốt |
| POST `/auth/refresh` | refreshToken | rotated tokens, expiresIn | Có endpoint; giữ semantics server |
| POST `/auth/logout` | bearer + refreshToken | revoke; success data null | Có endpoint |
| GET `/desktop/me` | bearer | userId, displayName, businessRoles, permissions | Facade đề xuất |
| GET `/desktop/skills` | q, mode, sourceId, page, pageSize | paged SkillSummary | Đề xuất trên search hiện có |
| GET `/desktop/taxonomy` | bearer | sources/catalogModes + metadata roles/categories/tags/agents | Đề xuất mapping taxonomy hiện có |
| GET `/desktop/skills/{skillId}` | bearer | SkillDetail + latestRelease | Đề xuất |
| GET `/desktop/skills/{skillId}/releases/{releaseId}` | bearer | immutable Release | Mới |
| GET `/desktop/skills/{skillId}/releases/{releaseId}/package` | bearer | exact archive bytes | Mới; có thể reuse storage |
| POST `/desktop/updates/check` | known skill/release pairs | latest release + per-item status | Mới, metadata only |
| GET `/desktop/mcp-catalog` | bearer | key/displayName/setupGuideUrl | Mới; metadata không chứa secret |
| GET `/desktop/app-release?platform=windows&arch=x64` | bearer | latest version/internal installer metadata | Mới; host IT hoặc Store |

Không có endpoint cài skill trên máy hoặc upload mcp.json. Cài đặt và MCP preflight thuộc desktop.

## 4. Auth example — response shape đề xuất

```json
{
  "success": true,
  "data": {
    "accessToken": "<opaque-access-token>",
    "refreshToken": "<opaque-refresh-token>",
    "expiresIn": 900,
    "user": {
      "id": "1001",
      "displayName": "Example Employee",
      "businessRoles": ["PM"]
    }
  }
}
```

900 chỉ là ví dụ payload, không phải TTL đã quyết định. Điểm danh tính email lấy từ source Store hiện tại; nếu công ty dùng username, đội Store đổi input và UI đồng bộ. Forgot password/account administration mở trang Store nội bộ; không nhân đôi workflow trên desktop.

## 5. Search và pagination

GET `/desktop/skills?q=jira&mode=all_time&sourceId=quality&page=1&pageSize=20`

Theo W02 revision 0.2: `q` match name/description (đề xuất); `mode` enum `all_time|trending|hot`, mặc định `all_time`; `sourceId` tùy chọn, bỏ trường nghĩa All sources. Keyword và Source kết hợp AND trong mode đang chọn. Page bắt đầu 1, pageSize mặc định 20, giới hạn 1–100. Client reset page khi đổi điều kiện. Không còn tham số role/agent/categoryId/tag/sort trong contract W02; metadata tương ứng vẫn dùng ở card/detail.

Ranking do server thực hiện trước pagination, secondary sort skillId để xử lý bằng điểm. Công thức, cửa sổ thời gian Trending/Hot và thứ tự All Time là V07 chưa chốt; không tự ánh xạ sang updated_desc/download count/rating. Mode chưa hỗ trợ trả `422 CATALOG_MODE_UNAVAILABLE`; client báo rõ, không âm thầm đổi mode. Unknown mode/sourceId trả 400. `totalItems` là tổng sau lọc ACL và điều kiện tìm.

Taxonomy draft bổ sung `sources: [{id, label}]` và `catalogModes: [{id, label, available}]`. Sources là nhóm nội dung nội bộ; không chứa URL nguồn bên ngoài. Ví dụ ID `quality` chỉ minh họa. Labels quan sát: Quality, Security, DevOps, AI/ML, Documentation, Other, Requirements, Design, Development, Testing/QA, Deployment, Operations/Common. Đội Store chốt mapping; không mặc định nhóm này chính là category hiện có. `All sources` là lựa chọn UI không có ID. Nút … hiển thị thêm taxonomy.

Local Install là tab local, không thêm endpoint upload/import vào Store khi P01 chưa chốt. Phần search cũ v0.1 được thay bằng contract này; không yêu cầu xóa API web hiện có.

```json
{
  "success": true,
  "data": {
    "items": [{
      "id": "skill-123",
      "name": "Jira Weekly Report",
      "description": "Summarize sprint progress and blockers.",
      "metadataRevision": 8,
      "roles": ["PM", "TPM"],
      "tags": ["jira", "reporting"],
      "categoryIds": ["project-management"],
      "sourceIds": ["quality"],
      "creator": {"id": "2001", "displayName": "Delivery Enablement"},
      "latestRelease": {"id": "rel-003", "sequence": 3, "version": "1.2.0"},
      "compatibleAgents": ["aipro", "claude_code", "gemini_cli", "codex"],
      "canDownload": true,
      "updatedAt": "2026-10-15T02:00:00Z"
    }],
    "page": 1,
    "pageSize": 20,
    "totalItems": 1
  }
}
```

Ví dụ tên/ID/số liệu là dữ liệu minh họa. Store không nên tự gán compatibleAgents cho cả bốn khi tác giả chưa khai báo. Unknown = null, client thể hiện chưa xác nhận.

## 6. Detail và release

SkillDetail gồm các trường Summary và `description`, `storePageUrl`, `latestRelease` đầy đủ theo schema sau. Description có thể là Markdown, client sanitize/không tải ảnh public tự động. Đường dẫn có host là placeholders do IT thay.

```json
{
  "success": true,
  "data": {
    "skillId": "skill-123",
    "id": "rel-003",
    "sequence": 3,
    "version": "1.2.0",
    "installName": "jira-weekly-report",
    "publishedAt": "2026-10-15T02:00:00Z",
    "changelog": "Add blocked-task summary.",
    "compatibleAgents": ["aipro", "claude_code", "gemini_cli", "codex"],
    "requiredMcps": ["jira-mcp"],
    "package": {
      "format": "zip",
      "fileName": "jira-weekly-report-1.2.0.zip",
      "sizeBytes": 42112,
      "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "downloadPath": "/api/v1/desktop/skills/skill-123/releases/rel-003/package"
    }
  }
}
```

Digest toàn chữ a chỉ minh họa format 64 hex; server phải tính digest thật từ đúng artifact. `format` enum zip/7z. Một release chỉ có một package cài đặt trong bản đầu; file tham khảo giữ riêng ở Store. installName là basename Windows hợp lệ; thay installName giữa release cần desktop xử lý rename/migration rõ ràng, bản đầu đề xuất giữ ổn định trong một skill.

Release sequence số nguyên tăng theo skill, không giảm/tái dùng. Không so sánh labels semver bằng chuỗi. Release cũ vẫn tải được khi còn được phép; nếu bị rút thì 410 RELEASE_WITHDRAWN, không âm thầm trả latest bytes tại URL cũ.

## 7. Binary package

200: `Content-Type: application/zip` hoặc `application/x-7z-compressed`; `Content-Length`; `Content-Disposition`; `ETag` ổn định theo archive digest. Binary body, không bọc JSON. Client xác minh sha256 từ release JSON; ETag là cache hint.

Download phải xác thực ở thời điểm tải. Nếu signed URL được dùng về sau, chỉ host nội bộ đã được cấu hình; không chuyển Bearer token tới host khác. Bản đầu ưu tiên authenticated same-origin stream, không cần signed URL hoặc range resume. Retry tải lại file staging; không tự giải nén phần file cũ.

Store publish pipeline cần xác định install artifact có đúng một SKILL.md root và tài nguyên. Download-all hiện có là tiện ích tải attachments, không đảm bảo contract này. Với ZIP/7z đính kèm sẵn, endpoint package stream đúng file đã chọn làm release, không bọc thêm ZIP nữa.

## 8. Update check

Input tối đa 100 cặp unique skillId/releaseId mỗi request; desktop chunk khi nhiều hơn. Store deduplicate skill nhưng trả tương ứng từng cặp, vì nhiều target có thể dùng release khác nhau.

```json
{
  "items": [
    {"skillId": "skill-123", "releaseId": "rel-002"},
    {"skillId": "skill-123", "releaseId": "rel-003"}
  ]
}
```

```json
{
  "success": true,
  "data": {
    "checkedAt": "2026-10-15T03:00:00Z",
    "items": [
      {"skillId": "skill-123", "currentReleaseId": "rel-002", "status": "update_available",
       "latestRelease": {"id": "rel-003", "sequence": 3, "version": "1.2.0"}},
      {"skillId": "skill-123", "currentReleaseId": "rel-003", "status": "up_to_date",
       "latestRelease": {"id": "rel-003", "sequence": 3, "version": "1.2.0"}}
    ]
  }
}
```

Status: update_available, up_to_date, unavailable, unknown_release. Không tiết lộ metadata skill user mất quyền; dùng unavailable và không latestRelease. Không gửi đường dẫn project/file local, hash bản sửa hoặc token MCP. Chưa bao gồm pinned target preference phía server; desktop quản lý target selection.

## 9. MCP catalog và app update

MCP catalog items: `{ "key": "jira-mcp", "displayName": "Jira MCP", "setupGuideUrl": "https://<internal-host>/guides/jira-mcp" }`. Entry không chứa URL endpoint có credential hoặc cấu hình token nhân viên. Nếu catalog chưa có tên yêu cầu, app vẫn hiển thị raw standardized key; thiếu hướng dẫn không chặn cài.

App release data: `version`, `releaseId`, `publishedAt`, `releaseNotes`, `installerUrl`, `sha256`, `sizeBytes`, `platform`, `arch`. Unknown platform/arch trả 404. User click link cài đặt nội bộ theo P04; không auto-download/restart. Windows architecture cuối cùng theo V02.

## 10. Error matrix và cache

| HTTP / code | Xử lý desktop |
|---|---|
| 400 INVALID_REQUEST | Nêu lỗi input/schema; không retry vòng lặp |
| 401 SESSION_EXPIRED | Refresh một lần, sau đó login lại; giữ thao tác đang chuẩn bị |
| 403 ACCESS_DENIED | Báo thiếu quyền, không thay bản đã cài |
| 404 NOT_FOUND | Không thấy nội dung/không được lộ; giữ local state |
| 409 RELEASE_CHANGED | Refresh metadata và yêu cầu review lại, không thay release ngầm |
| 410 RELEASE_WITHDRAWN | Không tải; thông báo Store đã rút release |
| 422 CATALOG_MODE_UNAVAILABLE | Nêu mode chưa có dữ liệu/hỗ trợ, cho user chọn mode khác; không giả kết quả |
| 429 RATE_LIMITED | Tôn trọng Retry-After, nút retry |
| 503 STORE_UNAVAILABLE / timeout | Banner offline; Library vẫn dùng theo P07 |

Catalog cache theo account và lastFetchedAt, không dùng cache để khẳng định quyền tải. Release/package cache theo profile/skill/release/digest. Không cache password hoặc auth error body nhạy cảm. Các endpoint protected phải có giới hạn tài nguyên server; số lượng employee >1.000 không đồng nghĩa 1.000 request đồng thời đã được xác nhận.

## 11. Checklist trao đổi với đội Store

1. Giữ auth hiện có và thống nhất response adapter cho desktop.
2. Chốt release model độc lập metadata version, publish/withdraw và retention release cũ.
3. Chốt install artifact, ZIP/7z, digest và immutable bytes.
4. Thêm requiredMcps và agent compatibility vào release; chốt sources/catalogModes, ranking và availability theo V07. Business role vẫn là metadata.
5. Quyền catalog/detail/package và cách trả unavailable sau thu hồi.
6. Cung cấp endpoints cùng fixtures contract nội bộ cho app team.
7. Chốt TTL token, URL/CA/proxy/host allowlist và nơi host bộ cài với IT.

Chưa gửi tài liệu này tới đội Store hoặc tạo issue. Đây là file bàn giao để người dùng review và chuyển cho đội dự án.
