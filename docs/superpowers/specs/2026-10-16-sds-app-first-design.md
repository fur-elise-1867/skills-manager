# SDS Skills Manager — App-first delivery design

Ngày: 2026-10-16 · Trạng thái: Draft for user review

## 1. Mục tiêu và quyết định

Phát triển tiếp desktop React/TypeScript + Tauri/Rust trong repo hiện tại. Đưa bản pilot AIPro dùng thật đến nhóm nhỏ sớm, sau đó hoàn thành phạm vi 1.0 theo các phase có nghiệm thu. “100%” nghĩa là hoàn thành yêu cầu 1.0 được chốt, không phải toàn bộ ý tưởng có thể phát sinh.

Người dùng đã chọn pilot AIPro, yêu cầu làm app trước và giao cả việc bổ sung API Store. Store nằm tại `<workspace>/skills-store/skill-management`. Các tài liệu nền là `docs/sds-phase1/{SRS,SAS,API-DRAFT,DEVELOPMENT-PLAN}.md`.

Ràng buộc Store bắt buộc: chỉ thêm folder/package/file và đối tượng DB mới nếu cần. Không sửa source hiện có; không sửa hoặc xóa table, index hay đối tượng DB hiện có. Chấp nhận duplicate code. Khi đến giai đoạn Store, khảo sát để chọn cơ chế tích hợp bằng phần thêm mới; không giả định có thể sửa security/config/build hiện hữu. Chạy local trước. Thiết kế triển khai Store sẽ được lập riêng theo các ràng buộc này.

## 2. Roadmap đến 1.0

| Phase | Phạm vi | Bằng chứng hoàn thành |
|---|---|---|
| 0 | Kiểm tra build nền, chốt contract app, cấu hình SDS, đường kết nối fixture/HTTP và bộ cài thử | Build/lint/test nền có báo cáo; app mở trên Windows; các khoảng cách được ghi rõ |
| 1 | Đăng nhập, catalog/detail, AIPro global, MCP warning, cài ZIP/7z, Library cơ bản, gỡ có phạm vi | Store local thật → tải gói → cài đúng target → AIPro nhận skill; nhóm pilot cài app và sử dụng được |
| 2 | AIPro project, update chọn lọc, local edits, snapshot/restore, offline local management, recovery hoàn thiện | Không mất local edits; target độc lập; phục hồi được các tình huống gián đoạn đã đặc tả |
| 3 | Claude/Gemini/Codex, global/project, nhiều target, preset/bulk | Đường dẫn và khả năng nhận skill được xác minh cho từng agent trong môi trường đích |
| 4 | Hoàn thiện các yêu cầu 1.0 còn lại, onboarding, settings, log, UI, app update nội bộ | Mỗi yêu cầu trong SRS 1.0 có kết quả nghiệm thu; Local Install chỉ đưa vào sau khi chốt phạm vi |
| 5 | Hồi quy, hiệu năng, phân phối/nâng cấp, hướng dẫn support, rollout | Bộ cài chấp nhận được trong môi trường công ty; không còn lỗi chặn phát hành |

Không cam kết lịch ngày trước khi đo baseline và hoàn thiện các phụ thuộc. Mỗi phase bàn giao một bản có thể kiểm chứng. Kiểm thử diễn ra trong từng phase, không dồn về Phase 5.

## 3. Thiết kế Phase 0–1 đề xuất

Pilot đầu tiên tập trung AIPro global; AIPro project ở Phase 2. Đây là phạm vi đề xuất cần review cùng tài liệu này.

Luồng chính: đăng nhập → tìm/xem skill → chọn AIPro global → xem đường dẫn/MCP → xác nhận → tải và kiểm tra gói → cài → xem kết quả/Library → gỡ khi cần.

Tái sử dụng shell, component, path guards, workspace primitives và SQLite khi phù hợp. Mở rộng bằng module có trách nhiệm rõ; không yêu cầu tách thành nhiều package vật lý ngay.

| Ranh giới | Trách nhiệm và hợp đồng |
|---|---|
| SDS profile | Chọn môi trường, endpoint, data root; chặn đường chạy Git/public marketplace/CLI/updater công khai trong profile SDS |
| Session/Store gateway | Login/refresh/logout; catalog/detail/release/package; ánh xạ lỗi HTTP thành lỗi nghiệp vụ |
| Library repository | Lưu origin, skill/release/deployment và trạng thái thao tác; migration bảo toàn dữ liệu cũ |
| Package service | Checksum, giới hạn giải nén, ZIP/7z, một skill root và tài nguyên; từ chối đường dẫn không an toàn |
| AIPro adapter | Resolve global target, kiểm tra khả dụng và đường dẫn; không tự cài agent |
| MCP preflight | Đọc global `.aipro/mcp.json`, so requiredMcps; không kiểm tra kết nối MCP hoặc sửa cấu hình |
| Operation coordinator | Prepare/review/execute/cancel, conflict check, journal, commit/recovery và kết quả |
| UI | Hiển thị state, lựa chọn và lỗi; gọi Tauri commands; không tự xử lý credential hay cài file |

Mọi entry point cài đặt phải đi qua coordinator. Release đã tải và deployment tại agent là hai đối tượng riêng. Cài bằng copy để từng target có phiên bản độc lập; cần kiểm chứng tương thích với dữ liệu nền.

## 4. App trước, Store sau

Contract app lấy API-DRAFT làm đầu vào và được cụ thể hóa bằng DTO/schema, error codes và fixtures trong implementation plan. Các endpoint cần cho pilot: login/refresh/logout, me, taxonomy, catalog/detail, release và package.

Gateway có hai implementation: fixture local và HTTP. UI dùng cùng command/data contract. Chế độ fixture phải hiển thị rõ Development data; chỉ được bật chủ động trong môi trường phát triển. Không tự chuyển từ lỗi HTTP sang dữ liệu giả.

Fixtures bao gồm catalog, release, gói mẫu có checksum thật và lỗi có thể tái hiện: hết phiên, mất mạng, thiếu quyền, package hỏng, MCP thiếu/sai cấu hình. Không dùng credential hay dữ liệu công ty thật trong fixture.

Fixture dùng data root và target sandbox riêng. Kiểm chứng AIPro thật dùng chế độ tích hợp có target hiển thị và được xác nhận qua luồng cài. Bản phát hành không bật fixture bằng mặc định.

Kết thúc phần app với fixture chỉ là mốc phát triển. Pilot hoàn thành sau khi API bổ sung của Store chạy local và luồng HTTP thật được kiểm chứng. Tích hợp Store local là phụ thuộc của Phase 1, không bị đẩy tới cuối dự án.

## 5. Hành vi và bảo vệ dữ liệu

- MCP thiếu: cảnh báo, Check again, Install anyway hoặc Cancel. Chỉ xác nhận có cấu hình, không tuyên bố MCP hoạt động.
- Gói không đúng checksum, archive lỗi hoặc target không hợp lệ: báo lý do; không thay nội dung target.
- Target có nội dung chưa quản lý hoặc có local edits: pilot giữ nguyên và báo conflict; không ghi đè tự động. Update/snapshot UI đầy đủ thuộc Phase 2.
- Cài đặt stage trước commit; có journal và rollback tối thiểu ngay Phase 1. Cancel giữa commit phải kết thúc hoặc rollback target trước khi báo kết quả.
- Gỡ chỉ tác động deployment được chọn. Local edits cần cảnh báo và xác nhận rõ; không xóa Library như tác dụng phụ.
- Phân biệt 401, 403, lỗi mạng, dữ liệu rỗng và lỗi package. Thông báo giữ được ngữ cảnh thao tác để thử lại.
- Token thật thuộc backend/credential storage, không ghi log hoặc SQLite. Fixture auth không đại diện cho cơ chế bảo mật production.
- Dữ liệu SDS và fixture tách khỏi dữ liệu app upstream. Không migrate/xóa dữ liệu người dùng ngoài phạm vi đã xác định.

## 6. UI baseline và giới hạn pilot

Dùng mapping W01–W14 trong STITCH-TRANSFER làm baseline đề xuất; ưu tiên các màn đăng nhập, Store, detail, install, MCP, result, Library và remove. Khi Stitch mâu thuẫn với yêu cầu, giữ business rules đã chốt trong SRS: account/password, MCP warning có Install anyway, không tự thêm SSO hoặc offline TTL.

Giữ bố cục hiện tại và áp dụng thành phần Stitch phù hợp. AX Portal branding và token/biến thể chưa thống nhất không được tự coi là quyết định cuối. Local Install, ranking Trending/Hot và policy retention chưa được chốt; có thể hoãn chức năng tương ứng trong pilot, hiển thị unavailable rõ ràng thay vì bịa semantics.

## 7. Cách giao việc và nghiệm thu

Mỗi ticket phải có yêu cầu SRS, screen, phạm vi sửa, contract, tình huống lỗi, phụ thuộc và bằng chứng hoàn thành. Triển khai từng luồng xuyên UI/backend/data; không hoàn thiện toàn bộ giao diện trước khi thử luồng cài.

Phase 0 kiểm tra baseline frontend build/lint và Rust checks/tests phù hợp. Phase 1 bổ sung test cho contract, session error handling, package/path validation, MCP parsing và thao tác filesystem trên thư mục tạm; kiểm chứng cài/gỡ và recovery qua tích hợp. Bộ cài Windows được cài thử trước khi bàn giao pilot. Không coi source hoặc screenshot là bằng chứng sản phẩm chạy đúng.

Mốc review: người dùng review thiết kế này → implementation plan chi tiết Phase 0–1 → chọn cách thực thi → triển khai và báo cáo bằng chứng. SRS và roadmap tổng thể tiếp tục được cập nhật theo quyết định, các phase sau được lập plan chi tiết khi sắp bắt đầu.

## 8. Các điểm cần xác minh khi thực thi

- Baseline build/toolchain Windows và lỗi tồn tại từ upstream.
- Schema/migration thực tế cho release/deployment và cách tách data root.
- API/auth/security extension của Store khả thi với điều kiện chỉ thêm mới; khảo sát trước khi viết module Store.
- Khả năng AIPro nhận skill đã cài, quyền filesystem và môi trường phân phối.
- Ranking, Local Install, official brand, runtime/signing/CA được chốt trước phase cần chúng; không chặn công việc độc lập bằng fixtures.

Đây là bản thiết kế đề xuất, không phải báo cáo implementation hay kiểm thử đã hoàn thành.
