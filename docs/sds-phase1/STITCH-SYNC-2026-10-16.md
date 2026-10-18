# Stitch design synchronization — revision 0.3

Ngày: 2026-10-16 (Asia/Saigon) · Documentation only · Draft for review.

## Nguồn, baseline và giới hạn

Project [9226348915970684263](https://stitch.withgoogle.com/projects/9226348915970684263). MCP `get_project` trả `updateTime=2026-10-16T15:14:10.500363Z`, visibility `PUBLIC`. Chỉ gọi list/get; không sửa Stitch. Snapshot: [2026-10-16-stitch-01](design-snapshots/2026-10-16-stitch-01/README.md).

Đã lấy danh sách 29 screens, get_screen cho cả 29, 28 HTML và 29 screenshot. Một lần get_screen W01 phụ lỗi, lần đọc lại thành công; `screens-retrieved.json` là kết quả đầy đủ, `screens.json` giữ lần đầu để truy vết. `image.png` là ảnh tham chiếu không có HTML. Lưu cả hai bộ W01–W14, project metadata, design systems, checksum SHA-256 và bản tài liệu trước đồng bộ.

Baseline so sánh là tài liệu revision 0.2, `wireframes.html`, `preview-store.png`, `preview-mcp.png`, `stitch-transfer.json` và W02-CHANGELOG. Không có bộ raw HTML/screenshot Stitch cũ đầy đủ trong repo để chứng minh diff theo thời gian của từng screen. “Added/Modified/Removed/Unchanged” dưới đây nghĩa là so với baseline cục bộ, không phải khẳng định thời điểm chỉnh từng màn. W02 asset đã đổi từ `17399203032189606408` sang `12548026123402487532`.

Giữ mapping 14 screen IDs trong STITCH-TRANSFER. Không chọn screen bằng title/order: W08 có hai screen cùng tên, bản chính là `629741bbcca2475fabdd7b80e0f29c77`; bản `b6fe75bbad4240a69b78d5c5d5ec99af` là biến thể. MCP không trả timestamp/version riêng cho screen; chưa xác định được biến thể nào được người dùng muốn thay thế bản chính. Hai bộ được đối chiếu, không gộp nội dung mâu thuẫn thành yêu cầu.

## Before / after — layout, navigation và components

Áp dụng bảng này cho 14 IDs chính; snapshot index có link HTML/screenshots và biến thể tương ứng. “Unchanged” chỉ áp dụng thành phần được nêu, không có nghĩa pixel giống hệt.

| Phạm vi | Class | Trước (local 0.2) | Sau (Stitch đọc hiện tại) | Xử lý tài liệu |
|---|---|---|---|---|
| Shell / branding | Modified | Skills Manager / Samsung SDS; sidebar rộng 225px, titlebar 32px | AX Portal / Samsung SDS trên bộ chính; titlebar 32px, sidebar 225px vẫn có trong HTML, icon Material Symbols, layout gọn hơn | Ghi nhận visual proposal; tên chính thức vẫn cần V04 |
| Navigation grouping | Modified | Presets, Global Workspace, Project Workspaces; count badges và Add project | Group label khác nhau giữa màn: Agents & Presets, Environment, Target Environments…; nhiều count/Add project không còn hiển thị | Chưa chốt một grouping chung; không bỏ chức năng workspace |
| Review navigation | Removed | Thanh review W01–W14 và chú thích fixture ngoài app | Không xuất hiện trong màn app Stitch | Chỉ bỏ trong presentation của export; giữ hướng dẫn review cục bộ |
| Navigation capability | Unchanged | Dashboard, Library, Skills Store, Backup, Settings, preset, 4 agent, Project Atlas | Các đích chính vẫn có | Giữ flow SRS §6; href="#" không chứng minh navigation hoạt động |
| W01 | Modified | Form đăng nhập và details lỗi trong nội dung | Card giữa vùng làm việc; panel error/support riêng bên dưới, nhãn Authentication Error / Connectivity Warning / Account Administration | Email, Password, Sign in, Forgot password và 3 thông báo lõi Unchanged |
| W01 footer | Removed | English default / Shared Store account / No credentials submitted | Không còn footer này trong export chính | Vẫn giữ English mặc định và cảnh báo prototype trong guide |
| W02 | Unchanged | Install Skills; Store/Local Install, All Time/Trending/Hot, Source, 3-column grid, pagination | Cùng cấu trúc và 6 fixture cards; Store/All Time/All sources chọn | Không mở rộng P01/P09/V07; chưa có nội dung Local Install |
| W02 state reference | Modified | No matches, empty catalog, loading, offline, session-expired tách rõ | Disclosure dùng No Matching Skills / Catalog Synchronizing / Store Offline; nhắc agent filters và SSO | Không thay state/error contract bằng copy mâu thuẫn; empty catalog/session expiry vẫn bắt buộc |
| W03 | Modified | Mô tả/MCP bên trái và package bên phải | Các panel viền gọn: description, required MCP trái; package + Install skill phải; quick search Ctrl+K ở titlebar | Metadata và CTA Unchanged; quick search Added nhưng hành vi/shortcut chưa chốt |
| W04 | Modified | Agent chips, scope options và path preview | Agent cards 2×2 với checkbox; AIPro/Claude chọn, Gemini/Codex không chọn; Project radio chọn; thêm Change directory | Giữ agent/scope/path review; không coi màu mờ là đã chứng minh disabled |
| W04 troubleshooting | Modified | Agent/path unavailable và conflict guidance | Disclosure hướng dẫn PATH/CLI và lệnh sds-agent-check | Lệnh và dependency runtime chưa được xác minh, không thành điều kiện cài |
| W05 | Modified | MCP table, warning, variants trong disclosure | Table và trạng thái Missing có panel; found/unverifiable cards cạnh nhau, thêm Check file permissions | Cancel / Check again / Install anyway và config-only check Unchanged |
| W06 | Modified | Package progress + result table + footer actions | Compact progress strip, Target / Result / Action table và in-progress/failure disclosure | Partial completion, No write access, retry target thất bại và giữ target thành công Unchanged |
| W07 | Modified | Toolbar/chips, 3 cards, bảng installed locations | Thanh search/status gọn; Categories label, table header “3 active targets detected” | All/Installed/Available/Updates, local edits/MCP badges và target versions Unchanged |
| W08 | Added | Project path, Add from Library, agent/preset pills, table, preset review | Thêm Rescan, copy path icon, filter workspace input, overflow; context/verification badges và Sync ID | Các control là affordance quan sát; integrity/dependency checks chưa thành bảo đảm runtime |
| W08 preset | Modified | Review panel với danh sách skill, input Preset name | Skill status chips, Preset name và actions trong panel ngang gọn | One-time application / Choose Library skills / Review installation Unchanged; không bật sync |
| W09 | Added | Chọn target, local edits warning, snapshot protection | “1 of 2 candidates selected”, concrete backup path và inspect_mcp_pipeline.json link | Path/link fixture chưa là hợp đồng storage; lựa chọn target và backup-failure stop Unchanged |
| W10 | Added | Offline banner, 2 cached cards, local actions, expired-session variant | Sync Queue Suspended card, retry countdown 48s, package/target labels, policy 72h trong disclosure | Không chấp nhận scheduler/72h policy; P07/V03 vẫn mở |
| W11 | Added | Snapshot table + restore preview | Scan folder, snapshot count, path/size; retention/daemon/compression footer | Restore flow Unchanged; 30-day retention/gzip/daemon chưa được chốt |
| W12 | Modified | General/Store/update và paths/MCP/activity groups | Hai cột panel dày; thêm copy MCP path, connection badges, language options và expanded audit controls | English, internal installer, local read-only MCP Unchanged; SSO/TTL/telemetry/translations chưa chốt |
| W13 | Modified | Centered destructive confirmation, path, checkbox backup | Centered card không có modal backdrop; Reveal, RELEASE 3.4.1, explicit backup archive, Esc hint, rules panel bên dưới | Cancel / Back up and remove, backup checkbox, giữ Library/other targets Unchanged; Esc “discard” không định nghĩa xóa |
| W14 | Modified | Welcome to Skills Manager; 3 metrics, onboarding, agent list, attention | Welcome to AX Portal; metric captions, onboarding links; agent table thêm Protocol; Refresh và diagnostics timestamp | Counts/detection chỉ fixture; protocol/runtime và MCP blocking copy cần review |
| Reference image | Added | Không có asset ảnh rời này trong baseline | image.png: Install Skills / Leaderboard / Local Install / Git Install, public source chips | Chỉ ảnh tham khảo; không thêm Git/public marketplace vào phạm vi |

## Tokens và kích thước

Nguồn: CSS `before/wireframes.html`, HTML từng screen, `project.json.designTheme` và `design-systems.json`. Không dùng kích thước screenshot làm kích thước cửa sổ: phần lớn PNG 2560×2048, canvas instance 1280×1024; W12 phụ 2880×2118, W10 phụ 2560×1800, W07 phụ 2560×1984, W05 có chiều cao lớn hơn. SRS baseline 1440×860/minimum 1100×640 vẫn giữ; design prose đề xuất 1440×900 chưa thay yêu cầu này.

| Token / geometry | Class | Baseline local | Export / metadata mới |
|---|---|---|---|
| Font | Modified | Segoe UI/Arial, body 14px/1.5; h1 26px, h2 19px | Inter cho UI, JetBrains Mono cho path/code; design system body 13px/18px, headings 20/16/14px, captions 11px, titlebar 12px |
| Canvas/text | Modified | #fafafa / #24272b; muted #6b6f79 | Named tokens background/surface #f8f9ff, on-surface #191c20, on-surface-variant #424752 |
| Primary/border | Modified | Blue #1657b8; border #e1e3e7 | Named primary #004091, primary-container #1657b8; outline-variant #c3c6d4 |
| Status tokens | Modified | Green #087b58, amber #92580b, red #ac3535 with tinted backgrounds | Semantic error #ba1a1a / #ffdad6 and tertiary #004e36; screens also contain utility colors. Preserve text/icon status, not color alone |
| Shell sizes | Unchanged | 225px sidebar / 32px titlebar | Same explicit values in mapped HTML; grouping/padding differs |
| Spacing | Modified | Content 28px, grid gap 15px, cards 18px | Metadata scale 4/8/12/16/24px; gutter/margin 16px; inspect individual HTML for actual utility classes |
| Radius | Modified | Cards 11px, buttons 7px, inputs 7–8px, pills 20px | Project ROUND_FOUR; design prose panels/controls 4px, modals 6px. W02 HTML radius DEFAULT 2px, lg 4px, xl 8px, full 12px: conflict flagged |
| Controls/tables | Added | Padding-driven control heights and tables | Design prose: controls 28/32px, table header 28px, rows 32px; selected row pale blue + 2px left marker; button/input focus/hover styles available in HTML |
| Dialog guidance | Added | Inline confirmation/restore panels | Design prose describes centered 480–640px modal, 44px header, 16px body, 48px footer; screenshot W13 is inline card, not evidence of modal focus trapping |
| Shadow/surface | Modified | Outer review window shadow, larger rounded cards | Flatter border-separated shell; design prose has dropdown/modal elevations. Do not infer overlays from prose alone |

Design-system prose retains old #fafafa/#24272b/#1657b8 palette while named tokens differ. Both are archived; do not silently choose one as implementation-wide truth. HTML also loads Tailwind/Google Fonts from public CDNs: these are archival design dependencies, not permission to violate SRS offline/internal-network requirements.

## Labels, validation và state coverage

- W01: retain “Email”, “Password”, “Incorrect email or password. Check your details and try again.”, Store unavailable and account unavailable messages. Preserve identifier and clear password after submit/error per guide; static inputs do not prove validation.
- W02: HTML placeholder still says “Search skills.sh market...”. Approved documentation remains “Search Skills Store...”. No extra public provider or agent dropdown is implied by disclosure copy.
- W04: “Choose agents”, “Where should this skill be available?”, “Project folder”, “Change directory”, “Review requirements”; checked AIPro/Claude + Project are example values. Path-invalid/conflict guidance remains required even when absent from the redesigned disclosure.
- W05: “Missing”, “Configuration found”, “Cannot verify configuration”, “No MCP required.”, “MCP requirements have not been provided.” and “AIPro MCP configuration was not found.” preserve distinct semantics. “Check file permissions” is an added affordance without proven behavior.
- W06: “Partially completed”, “Installed”, “Could not install”, “No write access to the folder”, “Review and retry”; progress and error examples are separate fixtures.
- W07–W09: keep local “Modified” distinct from release “Current”; only selected targets update. “Dependencies verified” / “Workspace integrity check passed” do not prove execution.
- W10: offline network failure remains distinct from “Your session has expired. Sign in again to access Skills Store.” No 72-hour access rule is accepted.
- W11/W13: checkbox/action labels preserve backup-before-replace/remove; snapshot failure must stop the requested destructive action. Specific paths and release labels are examples, not migration instructions.
- W12: language choices Korean/Japanese in HTML are unconfirmed additions. Keep English default and existing scope P05; “View activity / Export logs” must exclude secrets under LOG-01.
- W14: “Endpoint token or bridge socket required before installation can proceed.” conflicts with C07 and W05. Keep warning + manual setup/check again/cancel/install anyway.
- Hover/focus classes and selected states are inspectable in HTML; screenshot only proves captured state. Keyboard navigation, disabled behavior, dialogs, responsive layout and backend validation have not been functionally verified.

## Review issues — không tự đổi business rules

| ID | Cần review | Quy tắc giữ nguyên / owner |
|---|---|---|
| R01 | Chốt 14 mapped IDs hay bộ biến thể làm bản chính; không có per-screen timestamp | Product/design; mapping hiện tại giữ |
| R02 | AX Portal name, sidebar grouping, badge counts và shortcut Ctrl+K/Ctrl+N/⌘N | V04/C13; Windows, không mặc định shortcut macOS |
| R03 | Named tokens vs design prose vs radius HTML; 1440×900 vs SRS 1440×860 | Design/Desktop; cần một token source thống nhất |
| R04 | SSO, 14-day session TTL, mTLS, tenant/admin/policy labels, Korean/Japanese | C09/P05/V03; Store account/password và English vẫn chuẩn |
| R05 | 48s sync, 72h offline limit, SDS-SEC policy IDs | P07/V03/V06; không có policy được xác nhận |
| R06 | 30-day snapshot/log retention, gzip, daemon, execution/token analytics | P03/V06/LOG-01; local snapshots và operation logs, không tự thêm telemetry |
| R07 | CLI troubleshooting, agent protocols, integrity/dependency “verified”, MCP blocking on Dashboard | C06/C07/C08, V01; no CLI feature, config-only AIPro check, install anyway vẫn hợp lệ |
| R08 | Backup paths khác nhau ở W09/W11/W13; fixture versions 3.4.x vs 1.2.0 | Snapshot location/retention chưa chốt; không dùng fixture làm release contract |
| R09 | W02 public placeholder/SSO/agent-filter copy; reference image có Git Install | C02/C08, P01/P09/V07; internal Store, Local Install scope/ranking vẫn mở |
| R10 | Project visibility hiện PUBLIC, transfer cũ ghi PRIVATE | Ghi nhận metadata hiện tại; không đổi visibility trên Stitch |

## Validation và tài liệu thay đổi

Đối chiếu HTML cả 28 wireframes và xem screenshot của cả 29 assets qua 5 contact sheets; ảnh W02/MCP cũ được xem riêng. Đọc snapshot HTML cũ để so sánh form/copy/layout. Có thể xem ảnh gốc đầy đủ từ index; screenshot tổng quan không chứng minh mọi hover/focus. Kiểm tra số asset, image signatures, SHA-256, local links và giữ nguyên các dòng C01–C14. Không chạy build/test sản phẩm vì không đổi source.

Cập nhật tại chỗ: README, WIREFRAME-GUIDE, STITCH-TRANSFER, stitch-transfer.json, W02-CHANGELOG, SRS. Giữ section numbering SRS và cấu trúc tài liệu hiện có. Thêm báo cáo này và snapshot versioned. SAS/API-DRAFT/DEVELOPMENT-PLAN, skill-format-detection-spec và wireframes.html giữ nguyên: không có business/API contract mới được xác nhận; wireframes.html là prototype baseline cũ, không phải render mới.

Git đang ignore docs. Diff trước/sau dùng `git diff --no-index` với bản `before/`; không sửa .gitignore, không stage/commit. [Full diff](design-snapshots/2026-10-16-stitch-01/changes.diff) gồm nội dung tài liệu sửa và file snapshot mới (ảnh thể hiện dưới dạng binary patch); diff không tự chứa chính nó. Manifest ghi file/hash trước khi tạo diff.

