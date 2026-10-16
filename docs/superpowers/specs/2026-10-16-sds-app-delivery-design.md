# SDS Skills Manager — thiết kế bàn giao từ pilot đến 1.0

Ngày: 2026-10-16 (Asia/Saigon). Trạng thái: **Người dùng đã duyệt spec qua tin nhắn “Duyệt spec”; implementation plan chưa được duyệt**.

Bản này là spec chính cho lượt triển khai, thay cho draft cục bộ `2026-10-16-sds-app-first-design.md`. Các lựa chọn mang nhãn đề xuất bên dưới đã được chấp nhận trong lần duyệt spec, nhưng không tự mở rộng thành phê duyệt implementation plan hoặc chọn phương thức thực thi.

## 1. Mục tiêu và quyết định từ trao đổi

Phát triển tiếp desktop Skills Manager hiện có để nhân viên SDS sớm sử dụng skill từ Store nội bộ. Bản đầu là pilot AIPro; sau đó phát hành tăng dần đến phạm vi 1.0. “100%” nghĩa là hoàn thành và nghiệm thu phạm vi 1.0 bên dưới, bao gồm tích hợp thật, bộ cài và bàn giao. Mock, màn hình chạy được hoặc test đơn vị riêng lẻ không chứng minh app đã hoàn thành.

Người dùng đã chọn phương án pilot AIPro, yêu cầu làm app trước và cho phép tự bổ sung các API desktop cần vào Skills Store. Store phải chạy local trước. Người dùng đặt các ràng buộc bắt buộc:

- Store: chỉ thêm file/folder/package mới; không sửa, di chuyển hoặc xóa file hiện có, kể cả cấu hình, dependency manifest, security hoặc migration cũ.
- Database Store: chỉ thêm bảng/job/đối tượng mới khi cần. Không sửa hoặc xóa bảng, cột, index hay đối tượng hiện hữu. Không chạy script reset/recreate DB dùng chung.
- Có thể trùng lặp code trong package mới để giữ độc lập với nhóm Store.
- Ưu tiên triển khai app trước; việc làm API thuộc cùng mục tiêu và được thực hiện khi tới mốc tích hợp.

Các lựa chọn thiết kế cần review trong tài liệu này: pilot giới hạn AIPro global; dùng copy deployment; dữ liệu SDS tách khỏi Library cá nhân của bản upstream; giữ mapping Stitch hiện có; Local Install ZIP/7z thuộc phase 4. Đây là đề xuất, không ghi nhận thành quyết định người dùng đã xác nhận.

## 2. Căn cứ source đã kiểm tra

- App: `D:/Documents/Git Project/skills-manager-app/skills-manager`, branch `clone-app`, baseline `e6dcedf`, version 1.42.0; React/TypeScript, Tauri 2/Rust, SQLite.
- Store: `D:/Documents/Git Project/skills-manager-app/skills-store/skill-management`. Bản copy này chưa có Git repository; phải tạo manifest SHA-256 của file hiện hữu trước khi thêm API, rồi so lại sau khi hoàn thành.
- App có `src/lib/tauri.ts`, các views Library/Install/Workspace/Settings/Backup, `core/installer.rs`, `sync_engine.rs`, `path_guard.rs`, `tool_adapters.rs`, `skill_store.rs` và `content_hash.rs` để xem xét tái sử dụng.
- Installer hiện nhận ZIP/`.skill`, chưa nhận 7z. Có test Rust sẵn trong installer và adapters; chưa chạy test/build trong lượt thiết kế này.
- `lib.rs` hiện khởi động CLI bridge, skill updater, auto backup và đăng ký public/Git commands. `AppContext.tsx` có kiểm tra cập nhật nền. Chỉ ẩn menu sẽ không đáp ứng profile SDS.
- Store có `/api/v1/auth/login`, `/refresh`, `/logout`; security chain yêu cầu authenticated cho route ngoài danh sách public. `SkillAccessHelper` có kiểm tra visibility; vẫn phải kiểm tra riêng điều kiện download trong facade mới.
- Store mặc định port 8080, context path `/skm`. Base URL local dự kiến: `http://127.0.0.1:8080/skm`. Đây là cấu hình trong source, chưa chứng minh server đang chạy.
- Login trả `userId`, `userName`, `email`, `role`, `orgs`, `avatarUrl`, `accessToken`, `refreshToken` trong envelope `data`. Refresh có `token`, `accessToken`, `refreshToken`, `tokenType`. Không có `expiresIn` trong các DTO vừa kiểm tra.
- Store dùng Flyway, Hibernate `ddl-auto: validate`. Không thay thành `update/create` để tiện chạy local.

Tài liệu đầu vào: `docs/sds-phase1/{README,SRS,SAS,API-DRAFT,DEVELOPMENT-PLAN,STITCH-TRANSFER,STITCH-SYNC-2026-10-16}.md`. Các tài liệu này là lịch sử draft; bản thiết kế mới giải quyết yêu cầu “app trước, Store chỉ thêm mới”. Nội dung “Phase 1 chỉ làm tài liệu” trong draft cũ mô tả đợt khảo sát trước, không phải phase pilot trong roadmap mới.

## 3. Phạm vi 1.0 và tiêu chí kết thúc

Trong phạm vi: Windows desktop; tài khoản Store; catalog/detail; immutable releases; ZIP/7z; AIPro, Claude Code, Gemini CLI, Codex; global/project; Library; tags/search/grid/list; preset/bulk; AIPro MCP warning; cập nhật chủ động; local edits; snapshot/restore; gỡ có phạm vi; offline local management; activity log; settings/onboarding; thông báo và bộ cài nội bộ.

Đề xuất Local Install cho phase 4: nhập một ZIP/7z chứa đúng một skill root. Ghi origin local riêng, không giả Store ID/release/compatibility; MCP requirements chưa khai báo phải hiển thị unknown. Cài qua cùng coordinator. Local package không tự nhận update từ Store. Import folder tùy ý để phiên bản sau.

Ngoài phạm vi 1.0: Git Sync, public marketplace/Git install, CLI điều khiển app, SSO, tự cấu hình/khởi động MCP, tự cài update skill, cloud backup, desktop authoring/approval, macOS/Linux.

Hoàn thành 1.0 khi mọi requirement trong ma trận phase có bằng chứng nghiệm thu, app và API chạy cùng nhau, bộ cài hoạt động trên môi trường Windows mục tiêu, không còn lỗi chặn phát hành, có hướng dẫn vận hành và khôi phục. Ghi rõ hạn chế đã chấp nhận; không tính tính năng chỉ có fixture là đã tích hợp.

## 4. Roadmap theo sản phẩm bàn giao

| Phase | Đầu ra cho người dùng | Phạm vi và nguồn yêu cầu | Bằng chứng để kết thúc |
|---|---|---|---|
| 0 — Nền tảng | Bản cài nền SDS trên máy thử | Profile, phân vùng dữ liệu, baseline build/test, contract desktop, shell; SET-01 | Build/lint/test baseline được ghi nhận; bộ cài mở được; profile SDS không chạy public updater/Git/CLI; spec được review |
| 1 — Pilot AIPro | Đăng nhập → tìm/xem → cài AIPro global → Library → gỡ | AUTH-01/02, CAT-01..04, LIB-01 cơ bản, INS-01..07 cho một target, MCP-01..04, REM-01, LOG-01 cơ bản; W01..07,W12,W13 | Cài app trên Windows thử nghiệm; login/catalog/download từ Store local thật; AIPro nhận skill; package lỗi và cài gián đoạn không hỏng target; gỡ đúng thư mục |
| 2 — Dùng hằng ngày | AIPro project, update và phục hồi, quản lý offline | UPD-01..04, BAK-01, OFF-01, REM-02; W08..11 | Update target được chọn; bảo vệ local edits; snapshot/restore; restart recovery; phiên hết hạn khác mất mạng; deployment khác không đổi |
| 3 — Mở rộng agent | Claude/Gemini/Codex, nhiều target, preset/bulk | INS-01/02/07 mở rộng, MCP-05, PRE-01; W04,W06,W08 | Xác minh path và nhận skill của cả bốn agent; kết quả từng cặp skill/target; retry không lặp target thành công |
| 4 — Đủ phạm vi 1.0 | Local Install và hoàn thiện quản lý | CAT-05 theo đề xuất §3; LIB-01 đầy đủ; SET-01/LOG-01 đầy đủ; onboarding, dashboard, notification app update | Truy vết đủ yêu cầu SRS; không còn nút dẫn tới luồng chưa làm trong phạm vi phát hành; thông báo/bộ cài nội bộ có dữ liệu thật |
| 5 — Bàn giao 1.0 | Bản phát hành cho môi trường công ty | Hồi quy toàn phạm vi; installer upgrade; network/proxy/CA; quyền Windows; hỗ trợ | Biên bản nghiệm thu, build có thể tái tạo, kiểm tra nâng cấp/rollback, hướng dẫn support và danh sách vấn đề còn lại đã chấp nhận |

Không ấn định lịch tuần trước khi có baseline build và tích hợp đầu tiên. Mỗi phase có một bản dùng thử; phản hồi pilot được xử lý theo mức độ, không tự mở rộng phạm vi 1.0.

Giai đoạn đầu làm app và contract fixtures. Tiếp theo thêm API trong Store để kết thúc phase 1. Việc cần API thật không đẩy toàn bộ tích hợp đến cuối roadmap. Đóng gói bắt đầu ở phase 0; kiểm thử đi cùng từng phase.

## 5. Kiến trúc và ranh giới module

Giữ một desktop app trong repo hiện tại. Không viết lại toàn bộ UI hoặc tách thành nhiều dịch vụ. Các namespace dưới đây là ranh giới đề xuất, tên file cụ thể được chốt trong implementation plan.

| Module | Giao diện công khai | Dữ liệu/quyền sở hữu |
|---|---|---|
| SDS profile | capabilities(), storageRoots(), networkPolicy() | Bật/tắt runtime, địa chỉ Store, roots SDS, định danh bản cài |
| Store session | login(), status(), logout(), authenticatedRequest() | Token chỉ ở Rust/keyring; UI chỉ nhận danh tính và trạng thái |
| Store gateway | taxonomy(), search(query), detail(id), release(id), download(id), updates() | Chuyển DTO Store sang domain model; kiểm tra schema/envelope |
| Library repository | list(), findRelease(), listDeployments(), recordOperation() | SQLite và manifest cục bộ; migration app không ảnh hưởng DB Store |
| Package preparation | prepare(release, bytes) → verified artifact | Checksum, ZIP/7z, giới hạn và skill root; cache bất biến |
| Agent targets | detect(), resolve(agent, workspace), validate(path) | Adapter, override và canonical paths |
| MCP preflight | inspect(requiredMcps) → trạng thái theo tên | Chỉ đọc global AIPro mcp.json; không trả secret |
| Operation coordinator | prepare(request), execute(planId, acknowledgments), cancel(id), status(id) | Review plan, locks, journal, tiến độ và kết quả từng target |
| Deployment/snapshot | stage(), replace(), remove(), fingerprint(), snapshot(), restore() | Filesystem commit/recovery, snapshot manifests |
| UI features | Catalog, detail, review, result, Library, workspace, settings | Tauri IPC typed; không trực tiếp giữ token hoặc tự ghi thư mục agent |

Các điểm bắt đầu cài từ Store, Library, preset, badge, bulk hoặc Local Install đều đi qua coordinator. MCP/conflict review không được triển khai riêng ở từng màn.

Tái sử dụng helper có hành vi phù hợp qua test. Không gọi installer/sync cũ chỉ vì tên gần giống nếu nó làm mất invariant về release, copy hoặc target isolation. Refactor có giới hạn ở seam đang làm.

## 6. Dữ liệu và trạng thái

- Skill: định danh Store ổn định, độc lập displayName. Local import có origin và ID namespace riêng.
- Release: profileId + skillId + releaseId, sequence, version label, installName, package digest, content manifest, requiredMcps, compatibleAgents. Metadata mô tả thay đổi không tạo release mới.
- Deployment: release cụ thể + agent + workspace + canonical target path + baseline fingerprint. Mỗi target là copy độc lập.
- Operation: ID, frozen plan, expected target fingerprint, trạng thái staging/backup/commit, kết quả và mã lỗi. Journal được ghi bền vững trước các bước có thể làm thay đổi target.
- Snapshot: nội dung skill, manifest, target gốc, thời điểm/lý do. Không gồm token, mcp.json hay cấu hình tài khoản.
- Catalog cache: tách theo Store profile/account/query; hiển thị timestamp. Không coi cache là bằng chứng quyền tải hiện tại.

Đề xuất root dữ liệu SDS riêng, có test override để kiểm thử không chạm Library/agent thật. Không tự migrate, adopt, xóa hoặc viết lại dữ liệu upstream khi mở bản SDS. Agent target vẫn là thư mục thật người dùng đã chọn; nội dung chưa được SDS quản lý phải được nhận diện trước thao tác.

Trong pilot, cài mới, cài lại cùng release chưa sửa và gỡ bản chưa sửa được hỗ trợ. Target trùng tên/khác skill hoặc có local edits được giữ nguyên, báo rõ cần xử lý; chưa mở ghi đè/gỡ bản đã sửa trước khi phase 2 có snapshot/review. Điều này là giới hạn pilot, không loại REM-01/UPD-03 khỏi 1.0.

## 7. Luồng pilot và lỗi

1. Nhân viên cấu hình Store local/nội bộ và đăng nhập bằng tài khoản Store.
2. Catalog tìm theo keyword/mode/Source, phân trang server; reset page khi điều kiện đổi, bỏ response cũ tới muộn. Mode chưa được server hỗ trợ hiển thị unavailable, không giả ranking.
3. Detail hiển thị release và MCP metadata; missing khác danh sách rỗng. Skill chưa có installable release không có nút cài hoạt động.
4. Chọn AIPro global, hiển thị đường dẫn `%USERPROFILE%/.aipro/skills/<installName>` và phát hiện xung đột trước xác nhận.
5. Đọc `%USERPROFILE%/.aipro/mcp.json`, trường mcpServers. Match tên exact; có object không rỗng chỉ nghĩa configuration found. Cảnh báo cho phép Check again, Install anyway, Cancel và hướng dẫn nếu có.
6. Download cùng origin, kiểm tra SHA-256/size, giải nén staging, xác minh đúng một root SKILL.md và tài nguyên. Không chạy script. Từ chối traversal, absolute/UNC/drive paths, reparse/symlink, Windows reserved names, archive lỗi, root mơ hồ, encrypted/multipart 7z.
7. Lock và kiểm tra lại fingerprint, stage trên volume target, commit và cập nhật DB. Lỗi giữa filesystem/DB được đối soát qua journal khi restart; không tuyên bố transaction atomic xuyên cả hai.
8. Kết quả hiển thị release, target path và MCP warnings; Library phản ánh file thật. Cancel trước commit không đổi target; trong commit phải kết thúc hoặc rollback target trước khi dừng.

Download, hash và extraction chạy ngoài UI thread. Giới hạn archive bytes/entry/depth phải là cấu hình có giá trị hữu hạn và được kiểm thử; implementation plan xác định giá trị ban đầu theo fixtures, tài liệu ghi rõ đây là giới hạn sản phẩm, không phải số liệu đã đo từ Store.

Không có implicit fallback từ Store lỗi sang mock. Tài khoản hết phiên, thiếu quyền, mất mạng và nội dung rút khỏi Store có thông báo khác nhau. Log chỉ gồm ID, path cần thiết, stage, code và thông báo đã lọc secret.

## 8. Contract Store và chế độ local

Base URL bao gồm context path `/skm`. Nối route phải giữ context path: `/skm/api/v1/...`; không ghép URL làm mất `/skm`. HTTP chỉ cho loopback trong cấu hình development. Môi trường nội bộ dùng HTTPS có xác minh chứng chỉ; không cho tắt TLS verification.

Tái sử dụng auth hiện có qua adapter Rust. Chuẩn hóa userId thành string, userName thành displayName. Không yêu cầu Store sửa DTO để thêm expiresIn hoặc user object. Refresh single-flight khi protected request nhận 401, retry tối đa một lần; thất bại chuyển expired. Không refresh trên 403. Tokens không qua frontend IPC, localStorage hoặc SQLite; refresh token lưu Windows keyring. Nếu keyring không dùng được, báo lỗi hoặc dùng phiên không lưu rõ ràng, không ghi plaintext.

Facade mới dùng prefix `/api/v1/desktop`. App contract gồm: me, taxonomy, skills, skill detail, release metadata, package stream, updates/check, mcp-catalog và app-release. Endpoint nào chưa triển khai trả unavailable có chủ đích; không trả dữ liệu giả trong runtime thật.

Package endpoint stream đúng archive bất biến được khai báo trong release; không dùng download-all attachments làm install artifact. Digest metadata và bytes phải khớp. IDs không được tự suy ra từ displayName, timestamp hoặc Skill.version hiện tại.

Fixtures nằm trong app test/development và mô tả cả success/error. Dùng để phát triển UI/backend trước API; test consumer và provider phải dùng cùng contract. Fake credential chỉ thuộc fixtures, không tạo tài khoản trong DB chung.

## 9. Store chỉ thêm mới — thiết kế tích hợp sau phần app

Đề xuất package `com.furelise.skillmanagement.desktop` với controller/DTO/service/repository/model riêng, cùng tests/resources mới. Không chỉnh SecurityConfig: route desktop mới vẫn phải được filter chain hiện tại bảo vệ. Nếu cần cấu hình riêng, thêm configuration class/profile file có tên riêng; không override bean hiện hữu hoặc thay policy auth của route cũ.

Đề xuất bảng `desktop_release`, `desktop_package`, `desktop_skill_source`, `desktop_operation_event` khi chức năng thật sự cần. Index/constraint chỉ thuộc bảng mới. Không thêm FK/index/trigger lên bảng cũ; Store IDs được tham chiếu bằng giá trị và kiểm tra ở service. Không ghi vào bảng nghiệp vụ cũ khi tạo desktop release.

Đọc visibility và quyền download từ cơ chế Store đang có; nếu tái sử dụng service tạo side effect thì chọn implementation đọc riêng trong package desktop và kiểm thử parity. Catalog, detail, release và package đều kiểm tra quyền; không chỉ kiểm tra ở UI.

Migration mới có prefix/version không đụng nhóm khác hoặc migration location/history riêng cho desktop. Cách lựa chọn phải dựa trên cấu hình Flyway thực tế lúc triển khai. Chỉ chạy trên DB local chuyên dụng với fixture data; không chạy toàn bộ migration upstream lên DB dùng chung. Nếu chạy bootstrap upstream ở DB local trống, ghi rõ đó là tạo môi trường thử độc lập, không thay schema hiện hữu của nhóm.

Manifest trước/sau phải chứng minh mọi file Store có từ trước giữ nguyên bytes. So sánh schema DB trước/sau chứng minh không có thay đổi/xóa đối tượng hiện hữu. Scripts local mới phải có tên riêng và khởi động bằng tham số/profile mới; không sửa start-all.bat, pom.xml, application.yml hoặc docker-compose.yml hiện tại.

Chưa khởi động Store trong lượt thiết kế. Tới mốc tích hợp, phải kiểm tra process/port và kết nối DB thật trước khi start để không chạy trùng hoặc tác động môi trường khác.

## 10. UI baseline

Dùng 14 mapped screen IDs trong `docs/sds-phase1/STITCH-TRANSFER.md`, project `9226348915970684263`; snapshot `2026-10-16-stitch-01` là baseline cố định ban đầu. Không tự đổi sang variant chỉ vì title giống nhau hoặc vị trí canvas.

Giữ bố cục sidebar, detail sheet và trạng thái từ SRS. English mặc định. Dùng tên SDS Skills Manager cho pilot; AX Portal là đề xuất branding chưa chốt. Không biến các label SSO, TTL 14 ngày, offline 72 giờ, retention 30 ngày hoặc MCP blocking trong Stitch thành business rule.

Các màn phase sau không được tạo nút có vẻ dùng được nhưng không hoạt động. UI pilot chỉ mở capability đã hoàn thành; source upstream giữ được để tham khảo. Tất cả profile gates cũng có ở command/background/tray boundary.

## 11. Kiểm chứng và workflow giao việc

Mỗi phase chia thành vertical slices nhỏ có kết quả người dùng quan sát được. Mỗi ticket phải liên kết requirement ID, module/seam, input/output, error states, phụ thuộc, acceptance và bằng chứng. Xây UI + Rust + storage phù hợp cùng một slice; tránh hoàn tất toàn bộ UI rồi mới làm backend.

Áp dụng test-first ở các seam: auth response/refresh, URL/context path, package validation, MCP states, target resolution, update selection và filesystem recovery. Dùng temp roots, fake HTTP server và database thử; test không chạm agent folders thật.

Các lớp kiểm chứng:

- Typecheck/build/lint và test hiện có: ghi baseline để phân biệt lỗi cũ với regression.
- Contract: payload Store thật, lỗi auth/ACL/ranking/package; binary digest, immutable release và context path.
- Filesystem: archive độc hại, target collision, Windows junction/case, local edits, cancel, snapshot failure, restart giữa commit/DB.
- UI: loading/empty/error/expired/offline, focus, overflow, progress và thao tác thực tế qua Tauri.
- Packaging: cài mới, chạy không admin theo chính sách môi trường, mở lại, upgrade, dữ liệu được giữ; WebView2/runtime và không phụ thuộc public network trong cấu hình SDS.
- Store additive audit: file manifest, schema diff, tests route cũ, ACL parity.

Test đầy đủ được chạy ở mốc hoàn thành phase, có review code theo Matt Pocock trước bàn giao. Lệnh cụ thể và trình tự từng ticket thuộc implementation plan tiếp theo, sau review spec này.

## 12. Thứ tự tài liệu và điều kiện tiếp tục

Bản này là design spec cho chương trình bàn giao và pilot đầu tiên, không phải báo cáo code đã viết hoặc build đã pass. SRS IDs cũ tiếp tục dùng; các phase sau sẽ có spec riêng ở mức chi tiết cần triển khai, giữ toàn bộ mục tiêu 1.0.

Sau khi người dùng review spec này: viết implementation plan cho phase 0–1, trình bày cách thực hiện để người dùng chọn theo workflow brainstorming, rồi triển khai app. Store được bổ sung API theo §9 tại mốc tích hợp thật. Các câu hỏi môi trường chưa có bằng chứng được giải quyết qua kiểm tra local và ghi quyết định; không tự báo pilot đạt khi chỉ chạy fixtures.

Kiểm tra nội bộ tài liệu: phạm vi 1.0 không thu nhỏ thành pilot; Store-only-add là invariant; có phân biệt quyết định người dùng/đề xuất; không nhận mock là tích hợp; không nhận cấu hình source là bằng chứng runtime; mọi phase có điều kiện nghiệm thu.
