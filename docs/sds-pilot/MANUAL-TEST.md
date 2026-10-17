# Chạy và manual test Phase 0–1 trên máy Harry

Hướng dẫn dùng môi trường pilot đã chuẩn bị trên máy này, không cần build lại. Chạy các lệnh bằng **PowerShell**, tài khoản Windows hiện tại. DB và BE có thể đang chạy sẵn; kiểm tra trước khi khởi động.

## 1. Khai báo đường dẫn

Copy block này vào **mỗi cửa sổ PowerShell** sẽ dùng:

```powershell
$workspace = 'D:\Documents\Git Project\skills-manager-app'
$appRoot = Join-Path $workspace '.worktrees\sds-phase0-1'
$pilotLedger = Join-Path $appRoot '.superpowers\sdd\2026-10-16-sds-app-phase0-1'
$backend = Join-Path $workspace 'skills-store\skill-management\backend'
$pgCtl = 'D:\Program Files\PostgreSQL\18\bin\pg_ctl.exe'
```

| Thành phần | Địa chỉ / dữ liệu |
| --- | --- |
| Store BE | `http://127.0.0.1:8080/skm` |
| Health BE | `http://127.0.0.1:8080/skm/actuator/health` |
| PostgreSQL pilot | `127.0.0.1:15432`, DB `sds_desktop_pilot_20261017`, user `sds_pilot` |
| Dữ liệu PostgreSQL | `$pilotLedger\postgres-pilot\data` |
| Dữ liệu app | `%LOCALAPPDATA%\SDS\SkillsManager` — SQLite, cache, logs; không cần chạy DB riêng cho app |

## 2. Mở app và đăng nhập

```powershell
Invoke-RestMethod 'http://127.0.0.1:8080/skm/actuator/health'
Start-Process -FilePath (Join-Path $pilotLedger 'installed-shell\sds-skills-manager.exe')
New-Item -ItemType Directory -Force -Path (Join-Path $workspace 'manual-test\skills') | Out-Null
```

Nếu health trả `status: UP`, chuyển thẳng sang cấu hình dưới đây. Nếu không kết nối được, làm mục 3 rồi thử lại.

1. Mở **Settings**, đặt **Store URL** = `http://127.0.0.1:8080/skm`, bấm **Save connection**. Đổi URL sẽ đăng xuất phiên cũ.
2. Đăng nhập tài khoản seed **chỉ dành cho môi trường local**: email `admin@skillmgmt.local`, password `Admin@12345`.
3. Trong Settings, dùng **Choose skills folder**, chọn `D:\Documents\Git Project\skills-manager-app\manual-test\skills` vừa tạo.

Máy này không có AIPro của công ty. Đường dẫn mặc định `%USERPROFILE%\.aipro\skills` có thể báo **Not ready**; dùng thư mục test ở trên. Việc chọn thư mục skills không thay đổi nơi đọc cấu hình MCP. Cảnh báo MCP thiếu/không xác định là kết quả hợp lệ trên máy này; xem cảnh báo rồi dùng **Install anyway** nếu giao diện cho phép. Việc chép file không chứng minh AIPro đã nhận hoặc chạy skill.

Bản đang cài là **development**, hỗ trợ HTTP loopback. Nếu cần cài lại, dùng `$pilotLedger\artifacts\SDS-Skills-Manager-1.42.0-development-eb00287.exe`. Bản **internal** yêu cầu HTTPS nên không dùng với BE local này.

## 3. Khởi động DB và BE khi đã tắt

### PostgreSQL

Sau khi khai báo mục 1, kiểm tra trạng thái:

```powershell
& $pgCtl -D (Join-Path $pilotLedger 'postgres-pilot\data') status
```

**Chỉ khi cluster chưa chạy**, chạy:

```powershell
& $pgCtl -D (Join-Path $pilotLedger 'postgres-pilot\data') `
  -o '-h 127.0.0.1 -p 15432' `
  -l (Join-Path $pilotLedger 'postgres-pilot\manual-test.log') -w start
```

Dùng đúng port **15432** và cluster có sẵn; không chạy `initdb`, không reset DB, không thao tác PostgreSQL service ở port 5432.

### Backend Store

Trong cửa sổ PowerShell đã khai báo mục 1, chạy nếu health chưa UP và port 8080 chưa bị dịch vụ khác chiếm:

```powershell
. (Join-Path $pilotLedger 'store-env.ps1')
& (Join-Path $backend 'desktop-pilot\start.ps1') `
  -DatabaseUrl $env:DESKTOP_PILOT_JDBC_URL `
  -PsqlPath $pilotPsql -MavenPath $pilotMaven
```

Giữ cửa sổ này mở. Script dùng Maven/JDK và dữ liệu đã chuẩn bị, tự kiểm tra đúng DB pilot trước khi chạy. Mở PowerShell khác kiểm tra health; chỉ đăng nhập khi `status: UP`. Mật khẩu DB/JWT được nạp qua file môi trường riêng, không cần nhập hay đưa vào tài liệu.

Nếu muốn xem DB bằng DBeaver/pgAdmin: dùng host/port/database/user ở mục 1; mật khẩu nằm trong `$env:DESKTOP_PILOT_DB_PASSWORD` sau khi nạp `store-env.ps1`. Không xuất biến này vào log hoặc ảnh chụp.

## 4. Checklist manual test

Library đã có dữ liệu nghiệm thu trước đó; không cần xóa để test lại. Chọn đúng thư mục `manual-test\skills` khi cài/gỡ để dễ đối chiếu.

| Thao tác | Kết quả mong đợi |
| --- | --- |
| Mở Store → All Time; tìm kiếm, lọc source | Hiện catalog; sắp tên A–Z, trùng tên dùng skill ID. Trending/Hot báo chưa hỗ trợ. |
| Mở `SDS Pilot ZIP` v1.0.0 | Xem chi tiết, phiên bản, đích cài, yêu cầu `jira-mcp`; có cảnh báo khi thiếu MCP. |
| Cài ZIP vào thư mục test | Có `sds-pilot-zip\SKILL.md` và resources; Library ghi nhận target đã cài. Có thể dùng bản đã tải trong cache. |
| Cài `SDS Pilot 7z` v1.0.2 | Giải nén/cài thành công vào `sds-pilot-unavailable`. Tên thư mục là dữ liệu thử cũ; package hiện có sẵn. |
| Library → Preview, Copy, Reveal | Preview đọc Markdown; đường dẫn copy/reveal trỏ đúng target đã chọn. |
| Đóng rồi mở lại app | Giữ Library, target và cấu hình đã lưu. |
| Remove from target cho ZIP vừa cài | Review đúng đường dẫn `manual-test\skills\sds-pilot-zip`; sau xác nhận, target bị gỡ, bản Downloaded và target 7z còn nguyên. |
| Tùy chọn: sửa `SKILL.md` trong target test rồi refresh/kiểm tra lại | App nhận thay đổi và chặn gỡ/ghi đè nội dung đã sửa; không tự xóa file sửa tay. |

## 5. Dừng và xử lý nhanh

- Đóng app bình thường. Nếu BE do bạn chạy ở mục 3: nhấn **Ctrl+C** trong chính cửa sổ BE. Nếu BE đang chạy sẵn từ phiên trước, chỉ dừng sau khi xác định đúng tiến trình pilot; không kill tùy tiện tiến trình giữ port 8080.
- Sau khi BE đã dừng, có thể dừng riêng DB pilot:

```powershell
& $pgCtl -D (Join-Path $pilotLedger 'postgres-pilot\data') -w stop
```

| Hiện tượng | Cách kiểm tra |
| --- | --- |
| Store không kết nối | Health phải UP; URL có `/skm`; dùng bản app development. |
| Script báo 8080 bị chiếm | Nếu health pilot UP thì dùng BE hiện có. Nếu là dịch vụ khác, xác định dịch vụ trước; script không thay thế nó. |
| Đích cài Not ready | Tạo thư mục test rồi chọn lại trong Settings; chọn thư mục local có quyền ghi. |
| Recovery required | Đóng/mở lại app để chạy recovery; xem logs qua Settings trước khi thử tiếp. |
| Không thấy dữ liệu app tại đường dẫn dự kiến | Codex có thể bị Windows chuyển hướng AppData; dùng đường dẫn app hiển thị và mục mở logs trong Settings. |

Giữ nguyên thư mục `$pilotLedger`: chứa cluster DB, package Store, môi trường riêng và bản app đã đăng ký cài đặt. Tài liệu kỹ thuật chi tiết: [RUNBOOK.md](RUNBOOK.md).
