# SRS — Samsung SDS Skills Manager

Version 0.3 · 2026-10-16 · Draft for review

Revision 0.3: [đối chiếu Stitch W01–W14 và visual proposals](STITCH-SYNC-2026-10-16.md); giữ business rules C01–C14 và IDs yêu cầu. Lịch sử revision 0.2: [W02](W02-CHANGELOG.md).
Áp dụng quyết định C/P/V trong [README](README.md). Tiêu chí bên dưới mô tả kết quả sản phẩm cần đạt, không ghi nhận testing đã thực hiện.

## 1. Vấn đề, mục tiêu và phạm vi

Nhân viên cần tìm skill hữu ích, cài đúng agent và đúng dự án mà không tự giải nén hoặc tìm đường dẫn. Công ty có hơn 1.000 người, nhiều role và không có truy cập Internet công khai. Store đang phát triển đã là nơi chia sẻ, quản lý quyền và phát hành nội dung.

Desktop là công cụ tìm, tải, quản lý và triển khai skill từ Store xuống máy Windows. Giữ bố cục và thành phần giao diện hiện tại; thêm kết nối Store và trạng thái nghiệp vụ. Thành công được nhận biết qua một luồng hoàn chỉnh: đăng nhập → tìm skill → chọn agent/phạm vi → xử lý MCP nếu AIPro → cài → nhìn thấy đường dẫn và trạng thái từng target. Nhân viên có thể cập nhật/gỡ/khôi phục có chủ đích.

Trong phạm vi: bốn agent, Library, Store, global/project, presets, tags/filters, preview, thao tác nhiều skill, activity log, snapshot cục bộ, settings, ZIP/7z, kiểm tra MCP AIPro, offline local management.

Ngoài phạm vi bản đầu: Git Sync, CLI/agent-control setup, public marketplace, direct Git install, tạo/chia sẻ/duyệt skill trên desktop, tự cài hoặc tự sửa MCP, kiểm tra kết nối MCP, tự cập nhật skill, SSO, triển khai macOS/Linux. W02 có tab Local Install; phạm vi nhập file/folder chưa chốt theo P01 được mở lại. Backup toàn thư viện theo P03 chưa xác nhận.

## 2. Thuật ngữ

| Từ | Nghĩa |
|---|---|
| Store | Dịch vụ nội bộ quản lý catalog, quyền truy cập và gói phát hành. |
| Skill | Một đơn vị có định danh ổn định, SKILL.md và tài nguyên. |
| Release | Một phiên bản nội dung đã phát hành, không thay đổi sau khi publish. |
| Library | Các release đã tải/giải nén và metadata trên máy. Có thể có nhiều release của cùng skill. |
| Deployment / target | Một skill được triển khai vào một agent tại một workspace cụ thể. |
| Global | Thư mục skill dùng chung của agent cho tài khoản Windows hiện tại. |
| Project | Thư mục skill của agent nằm trong project đã chọn. |
| Preset | Nhóm skill để áp dụng cùng lúc; áp dụng là thao tác một lần, không tự đồng bộ. |
| Local modification | Nội dung đang có khác fingerprint sau lần cài/cập nhật gần nhất. |
| MCP configured | Tìm thấy mục cấu hình đúng tên; không có nghĩa dịch vụ đang hoạt động. |

## 3. Actors và user stories

1. Là nhân viên, tôi đăng nhập bằng tài khoản Store để nhận catalog theo quyền của mình.
2. Là nhân viên mới, tôi được hướng dẫn chọn agent và hiểu global/project.
3. Là Dev/BSE/TPM/PM, tôi tìm bằng keyword, chọn All Time/Trending/Hot và Source để chọn skill phù hợp trong Store nội bộ.
4. Là nhân viên, tôi xem mô tả, tác giả, phiên bản, tài nguyên và yêu cầu MCP trước cài.
5. Là người dùng AIPro, tôi biết MCP nào đã có cấu hình và mục nào còn thiếu.
6. Là người dùng AIPro, tôi tự cài MCP, kiểm tra lại hoặc vẫn cài skill sau cảnh báo.
7. Là người dùng bốn agent, tôi chọn một hoặc nhiều agent mà không phải chép file thủ công.
8. Là thành viên dự án, tôi cài riêng trong project để quản lý phạm vi sử dụng.
9. Là nhân viên, tôi tải skill từ Store vào Library để dùng lại khi mất mạng.
10. Là người dùng, tôi được thông báo release mới và tự chọn cập nhật những target cần thiết.
11. Là người đã sửa skill, tôi giữ bản đang dùng hoặc sao lưu trước khi thay thế.
12. Là người quản lý nhiều project, tôi xem được phiên bản khác nhau của cùng skill ở từng nơi.
13. Là người dùng, tôi gỡ skill khỏi một target mà các nơi khác vẫn dùng được.
14. Là nhân viên, tôi gom skill thành preset và áp dụng cho agent/project đã chọn.
15. Là người dùng offline, tôi tiếp tục xem Library và quản lý nội dung đã tải.
16. Là nhân viên, tôi nhận lỗi rõ ràng và thử lại phần thất bại mà không mất phần đã thành công.
17. Là người dùng, tôi khôi phục snapshot đã lưu khi cần quay lại bản cũ.
18. Là nhân viên, tôi xem activity log để biết cài/cập nhật/gỡ đã ảnh hưởng nơi nào.
19. Là nhân viên, tôi nhận bản cập nhật app từ kênh nội bộ.
20. Là đội Store, tôi cung cấp API và package thống nhất để desktop không phải suy đoán description.

## 4. Yêu cầu chức năng

| ID | Yêu cầu và kết quả cần đạt | Wireframe |
|---|---|---|
| AUTH-01 | Đăng nhập bằng thông tin tài khoản Store. Store quyết định định danh email/username; bản nháp dùng email theo baseline hiện tại. Sai mật khẩu, khóa tài khoản, lỗi kết nối có thông báo riêng. | W01 |
| AUTH-02 | Refresh phiên theo API; hết phiên cần đăng nhập lại để tải. Đăng xuất xóa thông tin phiên và catalog của tài khoản; không xóa file skill đã cài. Chính sách offline theo P07. | W01,W12 |
| CAT-01 | W02 có tiêu đề Install Skills, tab Store/Local Install. Store có search, All Time/Trending/Hot, hàng Source và phân trang. Không còn dropdown role/agent/sort hoặc hàng category cũ; role/agent vẫn hiển thị trong metadata. Store lọc quyền tại server cho mọi endpoint; role nghiệp vụ không thay thế quyền. | W02 |
| CAT-02 | Skill detail có phiên bản, tác giả, mô tả, format/size gói, agent hỗ trợ, changelog và requiredMcps. Danh sách chưa có dữ liệu phải nói rõ, không suy ra tương thích. | W03 |
| CAT-03 | Loading, empty, no results, unavailable và session expired không hiển thị như catalog rỗng thành công. Nút tạo/chia sẻ mở Store web nội bộ theo P08. | W02,W10 |
| CAT-04 | Mode/Source/keyword kết hợp theo API draft; mặc định Store, All Time, All sources. Reset page khi đổi điều kiện, giữ query khi trở về; bỏ response cũ đến muộn. Source không đổi host. Semantics ranking và taxonomy cần V07; hành vi tìm kiếm là P09. | W02 |
| CAT-05 | Hiển thị Local Install theo W02; chưa đặc tả nội dung hay quyền import tùy ý. Chốt P01 trước triển khai tab này; luồng cài agent/scope/MCP hiện có vẫn áp dụng nếu mở nhập local. | W02,W04,W05 |
| LIB-01 | Library hiển thị skill, release đã tải, agent/target đã cài, tag, nguồn, trạng thái cập nhật và sửa cục bộ. Có tìm kiếm, grid/list và chọn nhiều. | W07 |
| INS-01 | Cài từ detail hoặc Library; chọn agent và global hoặc một project trong mỗi lượt. Hiển thị đường dẫn đích trước khi xác nhận. Có thể lặp lại cho project khác. | W04 |
| INS-02 | Agent chưa được nhận diện hiển thị trạng thái chưa sẵn sàng; hướng dẫn kiểm tra/cấu hình path. Không tự cài agent. Chặn target không hợp lệ nhưng vẫn cho chọn target khác. | W04,W12 |
| INS-03 | Gói ZIP/7z phải có một skill root chứa SKILL.md; hỗ trợ root trực tiếp hoặc một folder bọc ngoài. Giữ nguyên tài nguyên. Gói nhiều skill root hoặc chỉ chứa archive lồng nhau bị từ chối có hướng dẫn; không giải nén đệ quy tùy ý. | W06 |
| INS-04 | Tải, kiểm tra checksum, giải nén, kiểm tra root, triển khai và ghi kết quả. Trước khi thay thế nội dung phải chuẩn bị đủ bản mới và phương án khôi phục. | W06 |
| INS-05 | Không chạy script trong gói khi cài. File/path không an toàn, archive lỗi, thiếu SKILL.md, checksum sai không được triển khai. | W06 |
| INS-06 | Cùng skill/release đã cài và không sửa thì hiển thị Installed, không tạo bản trùng. Reinstall phải là thao tác rõ ràng. Folder đích có nội dung chưa quản lý hoặc skill ID khác phải cảnh báo, không tự ghi đè. | W04,W09 |
| INS-07 | Kết quả từng target: completed/failed/cancelled. Nếu một target lỗi, giữ các target thành công và cho retry target lỗi. Cancel không làm mất nội dung cũ. | W06 |
| MCP-01 | Khi target có AIPro, kiểm tra file global mcp.json và đối chiếu tên exact/case-sensitive chuẩn với requiredMcps của release được chọn. Không đọc config.json thay thế. | W05 |
| MCP-02 | `[]` nghĩa không cần MCP; null/không có trường là thông tin chưa được cung cấp. Không tự parse description. | W03,W05 |
| MCP-03 | Có key và cấu hình object không rỗng → Configuration found. Thiếu key → Missing. File thiếu/lỗi JSON/không đọc được/sai shape → Cannot verify hoặc Configuration missing, nêu rõ nguyên nhân. | W05 |
| MCP-04 | Cảnh báo có Open setup guide, Check again, Install anyway, Cancel. Bỏ qua không sửa file MCP. Lưu cảnh báo theo deployment; kiểm tra lại có thể cập nhật trạng thái. | W05,W07 |
| MCP-05 | Claude/Gemini/Codex luôn hiển thị MCP check not supported nếu cần giải thích. Không hiển thị dấu đạt giả. Cài thành công không khẳng định skill hoạt động đầy đủ. | W04,W06 |
| UPD-01 | Kiểm tra metadata release mới khi mở app và khi người dùng bấm Check for updates; tần suất nền cụ thể là đề xuất triển khai. Không tự tải/cài bản mới. | W07,W09 |
| UPD-02 | Chọn từng target để cập nhật; những target không chọn giữ nguyên. Nếu cùng skill có nhiều phiên bản, UI thể hiện riêng. | W09 |
| UPD-03 | Fingerprint khác baseline → Keep current hoặc Back up and update. Snapshot lỗi/hết dung lượng phải dừng thay thế target đó. Không ép cập nhật. | W09 |
| UPD-04 | Cập nhật dùng requiredMcps của release mới; chạy lại cảnh báo AIPro. Release bị rút/quyền tải thay đổi thì báo unavailable và giữ bản đang có. | W05,W09 |
| REM-01 | Remove from target chỉ gỡ thư mục skill được chọn, không xóa toàn skills root/rules/agents/config. Skill đã sửa phải cảnh báo và cho sao lưu. | W13 |
| REM-02 | Delete from Library là hành động riêng. Theo đề xuất, chặn xóa release đang được deployment tham chiếu; chỉ giải phóng cache không dùng. | W07,W13 |
| PRE-01 | Preset giữ nhóm skill để áp dụng một lần vào target đã chọn. Trước áp dụng, resolve release cụ thể, quyền tải và MCP; kết quả theo từng skill/target. Không tự cập nhật khi preset thay đổi. | W08 |
| OFF-01 | Sau đăng nhập thành công trước đó, mất Store vẫn xem Library, triển khai release đã cache, gỡ và xem snapshot cục bộ. Nội dung chưa cache không tải được; không tuyên bố quyền online còn hiệu lực. | W10 |
| BAK-01 | Snapshot cục bộ theo P03 lưu manifest, nội dung và target gốc, không gồm token/MCP config. Restore cần chọn/kiểm tra target và cảnh báo nội dung đã thay đổi từ lúc backup. | W11 |
| SET-01 | Settings giữ cấu trúc cũ: English mặc định, theme/text size, path agent, connection nội bộ, account, app version. Git Sync/CLI/public updater không được kích hoạt từ UI hay tác vụ nền. | W12 |
| LOG-01 | Ghi activity cục bộ: thao tác, skill/release, target, thời điểm, kết quả, mã lỗi. Không log mật khẩu, token hoặc toàn mcp.json. | W12 |

## 5. Quy tắc gói, tên và phiên bản

Skill ID do Store cấp, ổn định qua đổi tên. `installName` do Store chuẩn hóa, không chứa dấu phân cách path hoặc tên Windows dành riêng. Tên hiển thị không dùng làm khóa định danh. Target là tổ hợp skill ID + agent + workspace/path. Release ID định danh nội dung bất biến; version label để người đọc, release sequence để so sánh. Metadata revision có thể tăng mà không tạo bản cập nhật skill.

Copy là mặc định theo P02. Mỗi target có baseline hash riêng; thay đổi Library không tự đổi target đã cài. Các release đang dùng được giữ lại trong cache. Không so sánh version bằng chuỗi, timestamp chỉnh description hoặc trường version hiện tại của Store.

## 6. Luồng chính và ngoại lệ

**Cài:** Login → Store → Detail → agent/scope/path → MCP preflight nếu AIPro → download & validate → deploy → result. Có thể vào cùng luồng từ Library, bỏ download nếu đã có gói đã xác minh. Nếu chọn hỗn hợp AIPro và agent khác, người dùng có thể bỏ chọn AIPro hoặc xác nhận cảnh báo rồi tiếp tục.

**Cập nhật:** notification → xem release/changelog → chọn target → nhận diện local edits → backup choice → MCP release mới → cài → kết quả. Với target chưa sửa vẫn có bản rollback nội bộ trong quá trình thay thế; snapshot dài hạn dành cho bản sửa cục bộ theo lựa chọn.

**Offline:** lỗi kết nối Store → banner thời điểm dữ liệu cập nhật gần nhất → Library/local actions. 401 hết phiên được phân biệt với lỗi mạng; 403/404 không tự fallback sang download cache để giả lập quyền tải online. Khả năng thu hồi skill đã tồn tại trên máy không được cam kết; V03 cần chính sách công ty.

## 7. Yêu cầu phi chức năng và vận hành

- Windows là nền tảng mục tiêu. Version/architecture, quyền cài, WebView2 và signing cần V02; chưa cam kết bộ cài offline khi chưa xác minh runtime.
- UI giữ sidebar, toolbar, grid/list cards, preset pills, detail sheet và agent badges. Baseline cửa sổ app là 1440×860, minimum 1100×640 theo repo; nội dung dài được cuộn, dialog không che nút hành động.
- Không phụ thuộc CDN/public network cho font, icon, marketplace, update hoặc login. Link trong Markdown không tự tải ảnh ngoài; chỉ nội dung/resource được phép.
- Network và giải nén không khóa UI; có progress/cancel. Search phân trang 20, tối đa 100 theo draft API. Không giả định tải toàn bộ catalog cho 1.000 người dùng.
- Mật khẩu chỉ dùng để login; refresh token lưu qua Windows credential facility theo thiết kế SAS; không để token trong SQLite hoặc log.
- Có retry có giới hạn; lỗi thao tác phải nêu target, nguyên nhân, việc cần làm tiếp theo. Không mất bản cũ nếu thay thế thất bại.
- Mốc hiệu năng đề xuất: tương tác UI phản hồi trong 200 ms, trang catalog 20 mục trong 2 giây ở điều kiện mạng nội bộ được IT thống nhất. Đây là mục tiêu thiết kế, chưa đo hoặc xác nhận SLA.
- Hỗ trợ keyboard focus, label rõ, trạng thái ngoài màu sắc. English là mặc định; không tự dịch nội dung skill.

### Tham chiếu giao diện Stitch revision 0.3

Snapshot 2026-10-16 mô tả shell AX Portal với sidebar 225px/titlebar 32px, typography Inter/JetBrains Mono, controls/table compact; xem [tokens và bố cục từng màn](STITCH-SYNC-2026-10-16.md). Đây là đặc tả visual quan sát, chưa thay brand V04 hoặc baseline kích thước cửa sổ bên trên. Token prose/HTML chưa thống nhất cần R03.

Các controls mới (Rescan, Scan folder, Reveal, quick search, copy path, overflow) được ghi nhận để review, không bổ sung command/API contract. Không suy ra SSO, TTL 14 ngày, offline 72 giờ, retention 30 ngày, telemetry hoặc MCP blocking từ fixture. AUTH/MCP/OFF/BAK/LOG và C07 vẫn là chuẩn hành vi; thiếu trạng thái trong screenshot không có nghĩa yêu cầu bị xóa. Local Install vẫn chờ P01.

## 8. Điều kiện bàn giao cho phát triển

Review P01–P08, thống nhất API release/package/auth, xác minh đường dẫn và phiên bản agent, IT cung cấp deployment profile. Thiếu URL/CA thật không cản viết wireframe/spec, nhưng là phụ thuộc tích hợp thật. Kế hoạch [DEVELOPMENT-PLAN](DEVELOPMENT-PLAN.md) mô tả thứ tự. Testing không nằm trong hoạt động Phase 1 và không có kết quả kiểm thử sản phẩm trong tài liệu này.
