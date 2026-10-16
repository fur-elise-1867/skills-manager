# Samsung SDS Skills Manager — Phase 1

Ngày: 2026-10-16 · Phiên bản tài liệu: 0.3 · Trạng thái: **Draft for review**

Bộ tài liệu phân tích và thiết kế cho bản fork Windows phục vụ hơn 1.000 nhân viên Dev, BSE, TPM và PM. Chưa triển khai sản phẩm, chạy build hoặc testing. Kế hoạch là đầu vào cho giai đoạn phát triển sau khi review.

## Tài liệu

1. [SRS — Yêu cầu phần mềm](SRS.md): mục tiêu, phạm vi, user stories, yêu cầu và tiêu chí kết quả.
2. [SAS — Đặc tả kiến trúc](SAS.md): kiến trúc, dữ liệu, agent, trạng thái và luồng xử lý. SAS được hiểu là Software Architecture Specification trong bộ tài liệu này.
3. [API draft](API-DRAFT.md): hợp đồng đề xuất gửi đội Skills Store; không phải mô tả API đã triển khai đầy đủ.
4. [Kế hoạch phát triển](DEVELOPMENT-PLAN.md): các phần việc theo luồng hoàn chỉnh, phụ thuộc, đầu ra và các mốc bàn giao.
5. [Wireframe](wireframes.html): mở bằng trình duyệt; liên kết điều hướng giữa các màn hình và trạng thái minh họa. Không có kết nối thật, đăng nhập thật hoặc thao tác file.
6. [Chỉ dẫn wireframe](WIREFRAME-GUIDE.md): danh mục màn hình, luồng review, ánh xạ yêu cầu.
7. [Bản chuyển sang Google Stitch](STITCH-TRANSFER.md): project chỉnh sửa được, danh mục W01–W14 và giới hạn chuyển đổi.

Thay đổi mới nhất: [Stitch revision 0.3 — W01–W14, before/after và review issues](STITCH-SYNC-2026-10-16.md). [Snapshot versioned: HTML, screenshots, metadata và Git diff](design-snapshots/2026-10-16-stitch-01/README.md). Lịch sử revision 0.2: [W02 — tab và tìm kiếm](W02-CHANGELOG.md).

Revision 0.3 ghi nhận AX Portal và shell compact trong 14 screen IDs chính; project có thêm 14 biến thể và một ảnh tham chiếu. Mapping hiện tại được giữ vì không có timestamp riêng để chọn biến thể mới nhất. Các dòng C01–C14 dưới đây không đổi. Copy SSO, policy offline 72 giờ, retention 30 ngày và MCP blocking không trở thành business rules; xem R01–R10 trong báo cáo đồng bộ.

wireframes.html vẫn là prototype baseline cục bộ trước revision 0.3; dùng screenshots/HTML trong snapshot để review giao diện Stitch mới. Không sửa prototype hoặc source sản phẩm trong lượt này.

Ảnh xem nhanh (revision 0.1, ảnh Store đã cũ; xem W02 mới trong HTML/Stitch): [Skills Store cũ](preview-store.png) · [Cảnh báo MCP AIPro](preview-mcp.png). Đã mở và xem hai bản render này để rà bố cục tài liệu; đây không phải kiểm thử sản phẩm. Wireframe đầy đủ có 14 màn hình, dùng nhãn English và dữ liệu minh họa.

Lưu trữ: thư mục `docs` hiện nằm trong `.gitignore` có sẵn của repo, nên các file này được lưu trên máy nhưng chưa được Git theo dõi. Không sửa quy tắc ignore hoặc commit trong lượt này.

## Cách đọc mức độ quyết định

- **C — Confirmed:** người dùng đã xác nhận trong trao đổi.
- **P — Proposed:** quyết định thiết kế được đề xuất để tạo bản nháp nhất quán; cần review trước triển khai.
- **V — Verify:** thông tin bên ngoài cần đội IT, Store hoặc đầu mối agent xác minh.

## Quyết định đã xác nhận

| ID | Quyết định |
|---|---|
| C01 | Windows; giữ giao diện và các chức năng nền của Skills Manager, điều chỉnh theo phạm vi bên dưới. |
| C02 | Tìm và cài trực tiếp trong app qua Skills Store nội bộ; tương lai Store có thể dùng GitHub nội bộ làm nguồn. |
| C03 | Hiển thị và tích hợp AIPro, Claude, Gemini, Codex; ưu tiên AIPro. |
| C04 | AIPro đọc skill global ở `%USERPROFILE%\.aipro\skills` và project ở `<project>\.aipro\skills`. |
| C05 | Gói ZIP/7z chứa một folder có `SKILL.md` và tài nguyên; giữ cấu trúc bên trong khi cài. |
| C06 | Chỉ kiểm tra MCP cho AIPro: `%USERPROFILE%\.aipro\mcp.json`, trường `mcpServers`; `config.json` là cấu hình cài đặt khác. Tên MCP thống nhất trong công ty. |
| C07 | Store trả `requiredMcps` riêng. Thiếu MCP thì cảnh báo; người dùng tự cấu hình rồi kiểm tra lại, hủy, hoặc vẫn cài. |
| C08 | Git Sync và CLI tạm vô hiệu hóa, giữ source để mở lại sau. |
| C09 | Đăng nhập bằng tài khoản/mật khẩu hiện có của Store. |
| C10 | Chỉ thông báo phiên bản skill mới, người dùng chủ động cập nhật; bản sửa cục bộ được giữ lại hoặc sao lưu trước cập nhật. |
| C11 | Gỡ ở đâu chỉ ảnh hưởng nơi đó; Library riêng. Chọn agent và global/project khi cài. |
| C12 | Phân phối bộ cài nội bộ; kết nối nội bộ; mất Store vẫn quản lý skill đã tải. |
| C13 | Giữ bố cục, thêm hướng dẫn lần đầu, tìm/lọc và trạng thái cài đặt/MCP rõ ràng. English là ngôn ngữ mặc định. |
| C14 | Phase 1 chỉ phân tích, tài liệu, wireframe, kế hoạch; bỏ coding sản phẩm và testing. |

## Sổ giả định và xác minh

| ID | Loại | Quyết định bản nháp / câu cần xác minh | Đầu mối / thời điểm |
|---|---|---|---|
| P01 | P | Mở lại theo W02 ngày 2026-10-16: hiển thị tab Local Install, nhưng nội dung/phạm vi chưa chốt (ZIP/7z, folder hoặc gói Store đã tải). Đề xuất cũ ẩn import không còn là baseline UI. Git/public marketplace vẫn tắt; chưa coi nhãn tab là chấp thuận import tùy ý. | Product, trước chốt phạm vi |
| P02 | P | Dùng copy để triển khai; mỗi target độc lập về phiên bản. Không mặc định symlink vì cập nhật Library có thể đổi nhiều target ngoài lựa chọn. | Desktop |
| P03 | P | Backup bản đầu là snapshot cục bộ trước ghi đè và chức năng phục hồi; giữ vị trí màn hình Backup, không có cloud/Git sync. Backup toàn thư viện có thể bổ sung sau. | Product/Desktop |
| P04 | P | App update bản đầu: thông báo từ endpoint nội bộ và liên kết tới bộ cài nội bộ; người dùng tự cài. Không bật updater công khai. | IT/Desktop |
| P05 | P | English cho nhãn mới; không cam kết thêm bản dịch Việt/Hàn. Giữ cơ chế đa ngôn ngữ sẵn có, fallback English. | Product |
| P06 | P | Chỉ hiển thị bốn agent đã chốt. Claude được hiểu là Claude Code; Gemini là Gemini CLI tương ứng adapter có sẵn. | Product/đầu mối agent |
| P07 | P | Sau lần đăng nhập thành công, cho quản lý dữ liệu cục bộ khi offline; lần đầu cần online. Đăng xuất xóa phiên/cache catalog riêng của tài khoản, không xóa skill trên ổ đĩa. | Product/IT |
| P08 | P | Bản đầu không triển khai tạo/duyệt/upload skill trong desktop; dẫn sang Store web nội bộ. Dev/BSE/TPM/PM là phân loại nghiệp vụ, không tự biến thành quyền hệ thống. | Product/Store |
| P09 | P | W02: Store mặc định; một Source; query + mode + Source kết hợp; reset trang khi đổi điều kiện, giữ trạng thái khi quay về; debounce 300 ms/Enter. Xem W02-CHANGELOG. | Desktop/Product |
| V07 | V | Chốt semantics All Time/Trending/Hot, ranking/window/tie-break và mapping Source taxonomy, dữ liệu hỗ trợ từ Store. | Store/Product, trước T02 |
| V01 | V | Đường dẫn/phiên bản cụ thể của Claude Code, Gemini CLI, Codex tại SDS; cơ chế reload skill và ưu tiên global/project. Đường dẫn đọc từ repo chỉ là baseline, chưa chứng minh agent bản công ty hỗ trợ. | Agent owners, trước phát hành |
| V02 | V | URL Store thật, proxy, chứng chỉ CA, quyền cài app, kiến trúc Windows, WebView2 có sẵn hay phải đóng gói offline, chữ ký bộ cài. | IT, trước đóng gói |
| V03 | V | Schema auth, token TTL, phân quyền tải, API release/package mới và chính sách thu hồi quyền khi offline. | Store/IT, trước tích hợp thật |
| V04 | V | App brand/identifier chính thức; SAS template công ty nếu khác cách dùng tại đây. | Product, review tài liệu |
| V05 | V | MCP ở project có override global hay không. Bản đầu chỉ kiểm tra file global đúng yêu cầu đã chốt, hiển thị rõ đường dẫn kiểm tra. | AIPro owner |
| V06 | V | Thời gian phản hồi, quy mô catalog/gói, quota cache/snapshot, lịch retention phù hợp môi trường thực tế. | IT/Store |

## Căn cứ khảo sát source

Baseline desktop: package version 1.42.0, Git HEAD `e6dcedf` tại thời điểm đọc. React 19/TypeScript/Tauri 2/Rust. Store: Java/Spring Boot, Vue, PostgreSQL theo source; README Store cũ hơn chức năng thực tế nên ưu tiên controller/model.

| Nội dung | Căn cứ trong source |
|---|---|
| Sidebar, cards, agent badges, preset, workspace | `src/components/Sidebar.tsx`, `src/views/*`, `assets/demo/library.png` |
| Installer chỉ xử lý ZIP/skill | `src-tauri/src/core/installer.rs` |
| Adapter và đường dẫn hiện tại | `src-tauri/src/core/tool_adapters.rs` |
| Copy/symlink và chặn đường dẫn chồng lấn | `src-tauri/src/core/sync_engine.rs` |
| SQLite, metadata, locks, snapshot | `src-tauri/src/core/skill_store.rs`, `repo_lock.rs`, `git_backup.rs` |
| Endpoint công khai và tác vụ nền | `src-tauri/tauri.conf.json`, `core/skillssh_api.rs`, `core/github_api.rs`, `src-tauri/src/lib.rs` |
| Store auth | `backend/.../auth/controller/AuthenticationController.java` |
| Store tải file và download-all | `backend/.../skill/controller/SkillController.java` |
| Store version tăng mỗi lần save | `backend/.../skill/model/Skill.java` |

Các đường dẫn Store trong bảng được rút gọn từ `D:\Documents\Git Project\skills-store\skill-management`; đây là tài liệu tham chiếu, không thay đổi source Store. Chưa có tracker được cấu hình cho phần việc này; kế hoạch được giao bằng file cục bộ, chưa tạo issue hay gửi thông tin ra dịch vụ bên ngoài.
