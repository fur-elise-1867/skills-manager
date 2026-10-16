# Wireframe review guide

Version 0.3 · 2026-10-16 · Draft for review

Mở [snapshot Stitch revision 0.3](design-snapshots/2026-10-16-stitch-01/README.md) để xem HTML/screenshots mới nhất đã lấy. [wireframes.html](wireframes.html) giữ prototype baseline trước lần đồng bộ này, có liên kết chuyển trạng thái mẫu; không phải render mới của Stitch. Form/filter chỉ minh họa, không nhập tài khoản thật. HTML export có CDN/font bên ngoài; Ảnh đã lưu là tham chiếu trực quan tự chứa. Link action không chứng minh logic từ input hoặc navigation runtime.

## Nguyên tắc giao diện

Giữ sidebar trái, tiêu đề/toolbar trên, filter pills, card grid, agent badges, preset/workspace và panel chi tiết. Bộ Stitch chính hiện dùng “AX Portal / Samsung SDS”, Inter/JetBrains Mono và shell compact (sidebar 225px, titlebar 32px). Đây là visual proposal; brand chính thức vẫn chờ V04. Named tokens mới dùng surface #f8f9ff, primary #004091, primary-container #1657b8; design-system prose và một số radius HTML khác nhau, chưa coi là bộ token triển khai đã chốt. Xem bảng tokens và R02/R03 trong báo cáo đồng bộ.

Sidebar dùng Skills Store; W02 mới dùng tiêu đề Install Skills và hai tab Store/Local Install. Giữ Library, Dashboard, Preset, Global Workspace, Project Workspace, Backup, Settings. Backup chỉ local snapshots theo P03. Không hiện Git Sync/CLI hay các agent ngoài bốn agent đã chốt. Tên/số liệu/project trong mockup đều là fixtures.

## Màn hình

| ID | Màn hình | Trọng tâm | Requirements |
|---|---|---|---|
| W01 | Sign in | Tài khoản Store, lỗi login, link reset nội bộ | AUTH-01/02 |
| W02 | Skills Store | Store/Local Install; All Time/Trending/Hot, keyword, Source chips/…, grid và pagination | CAT-01/03/04/05 |
| W03 | Skill detail | Release/package/MCP/agent và CTA cài | CAT-02,MCP-02 |
| W04 | Install skill | Agent chips, global/project, path preview | INS-01/02/06 |
| W05 | AIPro MCP check | Missing/found/unverifiable, guide/recheck/skip/cancel | MCP-01…05 |
| W06 | Installation result | Progress, result từng target, retry failure | INS-03…07 |
| W07 | Library | Installed versions, updates/local changes/MCP warning | LIB-01,UPD-01 |
| W08 | Workspace & preset | Scope, agent, Add from Library, preset apply một lần | PRE-01,INS-01 |
| W09 | Update review | Target selection, local edits, backup choice | UPD-02…04 |
| W10 | Offline Library | Last sync, cached actions, retry/sign-in khi hết phiên | OFF-01,AUTH-02 |
| W11 | Backup | Snapshot cục bộ, path/reason, restore preview | BAK-01 |
| W12 | Settings | English, connection, account, agent paths, app update, activity | SET-01,LOG-01 |
| W13 | Remove skill | Scope/path và Library được giữ, cảnh báo local edits | REM-01/02 |
| W14 | Dashboard | Onboarding, trạng thái agent, lối vào Store/Library | C13,INS-02 |

## W02 revision 0.2 — 2026-10-16

Đối chiếu bản Stitch `e921a2c10a034eca80e4af20e760ad1f`; chi tiết nguồn, thay đổi và giới hạn trong [W02-CHANGELOG](W02-CHANGELOG.md). Store/All Time/All sources đang chọn. Source gồm Quality, Security, DevOps, AI/ML, Documentation, Other, Requirements, Design, Development, Testing/QA, Deployment, Operations/Common và … . Không còn dropdown role/agent/sort. Local Install có nhãn tab nhưng chưa có màn nội dung, không xem là một flow import đã hoàn tất.

HTML cục bộ đồng bộ bố cục W02 với nhãn tìm nội bộ `Search Skills Store...`; các tab/filter chỉ minh họa. Hành vi P09 và ranking/taxonomy V07 chưa được chứng minh bởi mockup. Không thêm skills.sh hoặc SSO từ các nhãn thừa trong export. preview-store.png là ảnh revision 0.1, không dùng làm baseline mới.

## Đồng bộ toàn bộ revision 0.3 — 2026-10-16

[Bảng Added / Modified / Removed / Unchanged cho W01–W14](STITCH-SYNC-2026-10-16.md) là đặc tả chênh lệch so với local 0.2. Giữ 14 IDs chính trong STITCH-TRANSFER; lưu cả 14 biến thể và ảnh tham chiếu, không chọn bản mới chỉ dựa vào tên. W08 có hai title giống nhau. Màn không có timestamp riêng; không khẳng định thứ tự revision.

W01 dùng login card + panel lỗi; W03 tách description/MCP và package sidebar; W04 agent cards 2×2, chọn AIPro/Claude + Project; W06 giữ result table theo target; W07 cards + installed-locations table; W08 thêm Rescan/copy path/filter; W09 chọn target + bảo vệ local edits; W10 thêm sync placeholder; W11 table + restore preview và Scan folder; W12 hai cột settings; W13 confirmation card với Reveal + backup checkbox; W14 metrics/onboarding + agent table. Đây là bố cục quan sát, không phải xác nhận hành vi các control mới.

Giữ nguyên trạng thái lỗi trong mục State coverage. Copy SSO, CLI troubleshooting, policy offline, retention, telemetry và Dashboard yêu cầu token trước cài cần review R04–R09; C07 vẫn cho Install anyway. Nhãn/shortcut và dữ liệu release/path/tenant/runtime trong mockup không thay các quy tắc C/P/V.

## Các đường review

1. W01 → W14 → W02 → W03 → W04 → W05 → W06 → W07: cài có cảnh báo MCP.
2. W07 → W09 → W05 → W06 → W11: update có local edits và snapshot (màn kết quả là fixture chung, implementation phải giữ context thao tác).
3. W08 → W13: gỡ đúng AIPro trong Project Atlas.
4. W10 → W07: offline vẫn thấy Library; retry link chỉ chuyển sang state online mẫu.
5. W12: review path override, không coi path ba agent là đã xác minh deployment công ty.

## State coverage ngoài khung minh họa chính

- Login: wrong credentials, disabled account, unreachable Store, expired session; giữ input định danh nhưng xóa password sau submit/error.
- Catalog: mode unavailable; query/Source/mode không có kết quả; loading skeleton, empty catalog, no matching results, 403, 503. Không dùng màn rỗng thay lỗi. Mỗi state có Retry/Clear filters/Sign in phù hợp.
- Install: agent missing, path invalid, folder conflict, encrypted/multi-root archive, missing SKILL.md, checksum mismatch, access denied, disk full, cancel, partial result. SRS INS là nguồn hành vi chuẩn.
- MCP: yêu cầu unknown hoặc [] khác nhau; config missing/unreadable/invalid khác missing key; configured không phải connected. W05 có các khối biến thể để review nội dung.
- Update: no updates, withdrawn release, metadata-only change, target changed since review, backup failed. Nút Back up and update phải dừng nếu snapshot không hoàn thành.
- Remove/restore: path modified, destination missing, local edits, snapshot không đủ dung lượng; không xóa ngoài target.

## Review cần tập trung

Xác nhận Store thay marketplace phù hợp; đường chọn agent/scope đủ rõ cho PM/BSE/TPM; từ ngữ MCP không khiến nhân viên hiểu nhầm là dịch vụ đã chạy; snapshot khác cloud backup; P01/P03/P07 có đáp ứng cách dùng công ty. Wireframe không chứng minh khả năng hoạt động hoặc độ chính xác runtime của app.
