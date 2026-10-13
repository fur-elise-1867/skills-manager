# Company Skills Manager — mốc L1–L5

App Windows dùng React/Tauri/Rust; các luồng hiện có: đăng nhập và giữ phiên trong Windows Credential Manager, tìm kiếm/phân trang Store, xem description đầy đủ, cài ZIP/7z vào AIPRO Global hoặc dự án, cảnh báo MCP dừng/bỏ qua, bản cài độc lập, phát hiện tùy chỉnh, cập nhật từng đích có backup và phục hồi thao tác dở.

Catalog đã tải được lưu theo Store/tài khoản/truy vấn với thời điểm lấy. Khi mất mạng, app chỉ hiển thị dữ liệu đã lưu và không cài từ cache; đăng xuất xóa cache tài khoản. Hết phiên giữ skill đang chọn để đọc lại detail sau đăng nhập, không tự chạy lại thao tác cài.

## Chạy demo

Cần Node 22.12+ (hoặc Node 24), Rust stable MSVC, Microsoft C++ Build Tools và WebView2. Cài dependencies bằng `npm ci`, sau đó:

```powershell
npm run company:demo
```

Tài khoản giả lập: `employee01`, mật khẩu `demo-password`. App có nhãn **Dữ liệu giả lập**. Demo dùng profile riêng `%LOCALAPPDATA%\CompanySkillsManagerDemo\profile`, dữ liệu riêng ở `...\data`, Store HTTP tại `127.0.0.1:4318`; không cài vào profile AIPRO thật của người dùng.

1. Đăng nhập và tìm skill Jira/Excel/Rà soát tài liệu.
2. Thêm thư mục ở Dự án nếu cần, rồi mở chi tiết skill và chọn Global hoặc dự án trước khi cài.
3. Demo ban đầu có mcpServers rỗng. Chọn Dừng rồi sửa `profile\.aipro\mcp.json` thủ công và thử lại, hoặc chọn Bỏ qua và cài tiếp.
4. Xem Skill đã cài và đường dẫn đích; đóng/mở app để kiểm tra phiên và dữ liệu cục bộ.

## Dự án và cập nhật

Thêm dự án chỉ ghi nhận thư mục, không tạo `.aipro` hoặc yêu cầu Git. Cài cùng skill cho Global và các dự án tạo các bản sao độc lập. Mở SKILL.md/thư mục để chỉnh sửa bằng công cụ trên máy; Làm mới nội dung phân biệt Nguyên bản, Đã tùy chỉnh, Không tìm thấy và Chưa kiểm tra được. Nếu bản cài bị chuyển/xóa và đích vẫn trống, Cài lại từ Store giữ ID quản lý cũ và kiểm tra MCP lại.

Kiểm tra gói mới so SHA-256 archive với baseline riêng từng bản cài. Chọn một hoặc nhiều đích để xem cập nhật; mỗi đích có kết quả và retry riêng. Tải để xem thay đổi chưa thay file cài. Xem file thêm/sửa/xóa và mở bản sao preview; chỉnh sửa preview không được áp dụng vào bản cài. Bản tùy chỉnh yêu cầu xác nhận rõ trước Backup và cập nhật.

App lưu đầy đủ nội dung và metadata/baseline cũ tại `data\\backups\\<installation-id>\\<backup-id>\\` trước khi đổi đích. Nút Bản lưu hiển thị vị trí và lịch sử; không tự xóa backup. Nội dung sửa sau preview, file đang khóa hoặc backup lỗi làm thao tác dừng, không tự merge. Khi crash giữa rename/DB, startup trả lại bản cũ hoặc giữ bản mới đã commit; dữ liệu không xác định được giữ và đích bị khóa để kiểm tra. Khôi phục thủ công qua UI/gỡ/dọn dung lượng là L6–L7 tiếp theo.

Mock mặc định giữ gói cũ. Để thử cập nhật bằng demo, tạo file có nội dung `1`, chạy Store với `--revision-file <đường-dẫn-file>` rồi đổi nội dung thành `2` sau khi cài gói đầu. Mock chuyển Jira sang fixture `jira-project-summary-v2.zip`; năm endpoint production vẫn giữ nguyên. Smoke L4–L5 tự điều khiển revision và dùng thư mục tạm riêng.

Gói Excel là 7z thật, decoder được đóng cùng app; người dùng app không cần cài 7-Zip. Nguồn fixture và archive đã được commit; 7-Zip chỉ được dùng khi tạo fixture ở giai đoạn phát triển.

## Kết nối Store nội bộ

Bản app bình thường đọc `%LOCALAPPDATA%\CompanySkillsManager\store.json`:

```json
{ "baseUrl": "https://your-internal-store-host" }
```

Thay hostname minh họa bằng dịch vụ nội bộ thật cung cấp năm endpoint của API draft v0.1 dưới `/api/v1`. Rust thực hiện HTTP, token không đưa ra frontend. HTTPS được xác minh bằng hệ thống chứng chỉ Windows; redirect bị từ chối. Bản release không nhận override debug/mock.

Các gói/description dùng hash và strong ETag theo contract. Nếu gói hoặc description thay khi đang tải, thao tác lỗi 412; chọn **Xem thông tin mới** để đọc lại detail rồi quyết định MCP cho lần cài mới. Không tái dùng quyền bỏ qua cũ. App chỉ xét tên MCP trong mcp.json, không kiểm tra kết nối/token/runtime và không sửa file này.

## Kiểm chứng

```powershell
npm run test:company
node --test scripts/company-mock-store.test.mjs
cargo test --manifest-path src-tauri/Cargo.toml --test company_flows
npm run lint
npm run build
npm run tauri:build -- --debug
node scripts/company-native-smoke.mjs
node scripts/company-native-workspace-smoke.mjs
```

API giả lập hỗ trợ `--scenario normal`, `session-expired`, `unavailable`, `package-changed`, `corrupt-download`, `forbidden-download`. Gói hỏng/unsafe, hash/size sai, conflict đích, file khóa hoặc journal không xác định không được báo cài thành công.

Native smoke mở app Windows đã build, dùng WebView2/IPC thật và profile tạm riêng; kiểm tra Credential Manager qua restart, đăng xuất, dừng/bỏ qua MCP, file ZIP/7z và dữ liệu bản cài. Recovery chỉ tự dọn staging hoàn chỉnh khớp manifest; nội dung không xác định hoặc bị sửa được giữ lại và báo đường dẫn cần kiểm tra.

Smoke L4–L5 kiểm chứng tạo dự án, ba bản cài độc lập, tùy chỉnh, preview/backup/update một đích và restart qua WebView2/IPC thật. Schema 4 giữ dữ liệu Global schema 2, chuẩn hóa ownership paths và thêm projects/backups/replacement journals. App không mở database của phiên bản mới hơn để ghi đè.

L6–L7 sẽ bổ sung UI khôi phục/gỡ và quản lý dung lượng. API thật, cách AIPRO nạp/ưu tiên skill, ký/phân phối installer do IT là phần nghiệm thu môi trường L8. Mốc L1–L5 xác nhận đặt file và quản lý bản cài, không chứng minh MCP hoặc AIPRO đã nạp/chạy skill.
