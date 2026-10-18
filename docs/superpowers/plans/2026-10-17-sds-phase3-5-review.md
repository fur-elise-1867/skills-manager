# Review và chỉ mục kế hoạch Phase 3–4–5

Ngày: 2026-10-17 · Phạm vi: review source/tài liệu/Stitch, lập develop & test plan. Chưa triển khai Phase 3–5.

## Kết luận

Đã có roadmap Phase 0–5 và plan chi tiết Phase 0–1; chưa có implementation plan riêng cho Phase 3–5 trong hai cây tài liệu được kiểm tra. `docs/sds-phase1/DEVELOPMENT-PLAN.md` là draft lịch sử T00–T13/D0–D6, không dùng số D3/D4/D5 làm số phase mới. Các plan dưới đây cụ thể hóa roadmap đã duyệt, không đổi mục tiêu 1.0.

- [Phase 3 — nhiều agent, target và preset](2026-10-17-sds-phase3.md)
- [Phase 4 — Local Install và hoàn thiện sản phẩm](2026-10-17-sds-phase4.md)
- [Phase 5 — kiểm chứng và bàn giao](2026-10-17-sds-phase5.md)

Trạng thái: **draft triển khai để review**, không phải bằng chứng các tính năng đã hoàn thành. Thực thi tuần tự là khuyến nghị vì các task cùng thay contracts/repository/coordinator. Không mở worker hoặc bắt đầu coding trong lượt lập plan này.

**Bổ sung theo yêu cầu tiếp theo ngày 2026-10-17:** người dùng xác nhận Phase 2 chưa hoàn thành. Đã thêm [Phase 2 — daily lifecycle, develop & test](2026-10-17-sds-phase2.md), gồm 9 task và checklist bàn giao G2. Có plan không có nghĩa G2 đã đạt. Các nhận xét baseline bên dưới mô tả lần khảo sát ban đầu; dùng bổ sung này cho trạng thái kế hoạch hiện tại.

## Căn cứ và thứ tự ưu tiên

1. [Delivery spec đã duyệt](../specs/2026-10-16-sds-app-delivery-design.md), nhất là §3–11.
2. Amendments và acceptance mới nhất trong `.worktrees/sds-phase0-1/docs/superpowers/plans/2026-10-16-sds-app-phase0-1.md` và `docs/superpowers/evidence/phase1-pilot.md` của worktree đó. Chúng mới hơn bản plan ở checkout tài liệu.
3. [SRS](../../sds-phase1/SRS.md), [SAS](../../sds-phase1/SAS.md), [API draft](../../sds-phase1/API-DRAFT.md), [mapping Stitch](../../sds-phase1/STITCH-TRANSFER.md), [báo cáo visual](../../sds-phase1/STITCH-SYNC-2026-10-16.md). Các dòng draft cũ về Local Install chưa chốt đã được delivery spec §3 thu hẹp thành một ZIP/7z chứa một skill root.
4. Source thực tế: `D:/Documents/Git Project/skills-manager-app/.worktrees/sds-phase0-1`, HEAD `be1cd179ca3cba957317a8a1c877716fcb44ad0c`. Checkout `skills-manager` vẫn HEAD `161f6d4`; không dùng checkout này làm code baseline Phase 3.

Không tìm thấy AGENTS.md trong workspace app qua inventory. Checkout tài liệu có thay đổi `.gitignore`, spec, docs và logo từ trước; worktree có `.gitignore` thay đổi và `dist-sds/` untracked. Lượt này chỉ thêm bốn tài liệu plan, không stage/commit hoặc sửa các thay đổi đó.

## Findings và cách xử lý

| Mức | Bằng chứng source/tài liệu | Hệ quả | Task xử lý |
|---|---|---|---|
| Chặn bắt đầu scope đầy đủ | Phase 2 chưa triển khai; đã bổ sung plan nhưng chưa có acceptance evidence, coordinator vẫn pilot | Không thể coi project/update/snapshot/offline đã sẵn sàng | P2.1–P2.9 → Gate G2 → P3.1 |
| Cao | `targets.rs` chỉ `resolve_aipro_global`; `contracts.rs::InstallRequest` chỉ skill/release; `repository.rs::validate_operation` chỉ chấp nhận aipro/global | Chỉ thêm checkbox agent sẽ lỗi prepare/persist/restart | P3.1–P3.2 mở rộng cả DTO, target, journal validation |
| Cao | `PreparedOperation` và `OperationResult` đơn target; `execute(id, ack_mcp)` có một acknowledgement | Bulk cần kết quả/retry/cancel từng cặp, không dùng một boolean cho cả batch | P3.3 |
| Cao | `Release`, cache key, LibraryItem và frozen plan bắt buộc Store identity/context | Local Install không thể giả release Store hay bypass toàn bộ guard phiên | P4.1: origin tường minh, migrate và local context riêng |
| Cao | `Repository::open` đã đọc user_version, chỉ chạy001 khi version=0 và từ chối version>1 | Mở rộng dispatcher hiện có để upgrade; không sửa migration cũ hoặc hạ version | P2.1–7, P3.1, P4.1, P5.2 |
| Trung bình | W08 có hai màn cùng title; W14/12 chứa copy policy/SSO/TTL giả định | Không lấy variant/title hoặc fixture làm business rules | Mapping cố định; P3.4, P4.2–P4.4 |
| Cao | Evidence pilot chỉ xác minh AIPro directory theo tiêu chí người dùng điều chỉnh | Không tuyên bố đã chạy AIPro; các agent khác cũng cần evidence riêng | P3.5, P5.1 |
| Cao | Internal installer pilot unsigned, WebView2 đã có sẵn; full Rust suite có 3 lỗi upstream | Chưa phải bằng chứng phân phối công ty hoặc toàn suite xanh | P5.1–P5.3 |

## G2 — phụ thuộc Phase 2, không tự bỏ qua

Người dùng đã yêu cầu lập plan Phase 2 sau review Phase 3–5; thực hiện [P2.1–P2.9](2026-10-17-sds-phase2.md) và nhận branch/commit + evidence trước khi mở capability project/update/restore/offline cho nhiều agent. Có thể khảo sát contract P3.1 khi G2 chưa đạt; không đóng Phase 3 khi G2 còn thiếu.

G2 cần chứng minh:

- Registry project ổn định, canonical root, AIPro project/global độc lập; không dùng display path làm workspace ID.
- Update chỉ target được chọn; metadata edit không tạo update; release bị rút/403 giữ target hiện tại.
- Local edits có Keep current/Back up and update; snapshot failure chặn thay thế; restore bảo vệ nội dung mới.
- Offline có xem/cài cache đã xác minh/gỡ/snapshot; 401/403 khác lỗi mạng; đổi account không trộn catalog.
- Cache delete chặn release còn referenced; recovery sau commit/DB interruption không mất dữ liệu.
- IPC, migration và tests thật được công bố. Tên interface Phase 2 cần được đối chiếu ở P3.1; tên mới trong plan là đề xuất triển khai, không khẳng định đã có.

## Đối chiếu Stitch trực tiếp

Đã gọi `list_screens` project [9226348915970684263](https://stitch.withgoogle.com/projects/9226348915970684263): 29 screens; đủ 14 IDs chính. Ngày 2026-10-17 tải HTML live và so sánh nội dung exact với snapshot `2026-10-16-stitch-01`: cả bảy màn sau khớp. Không sửa Stitch và không chọn các variant thay thế.

| Màn | ID chính | Plan |
|---|---|---|
| W02 | e921a2c10a034eca80e4af20e760ad1f | P4.1 Local Install |
| W04 | a3cca1f7d09646bca0f0a74a0fbbc1c2 | P3.2–3 chọn agent/scope |
| W06 | 727ccba2e7f14f37abce39cfcb41ad8d | P3.3 kết quả từng target |
| W07 | c4c7355ac0aa416eb275e610be3e916a | P4.2 Library |
| W08 | 629741bbcca2475fabdd7b80e0f29c77 | P3.4 workspace/preset |
| W12 | 662d6089aed24f248081b81a4a9fb2ed | P4.3 settings/activity |
| W14 | da58f733eb074f4aa714cb22457e12c8 | P4.4 dashboard/onboarding |

Đây là kiểm tra HTML và inventory, không phải visual/function QA của app Phase 3–5. W01/03/05/09/10/11/13 dùng snapshot được lưu, không tuyên bố đã tải lại HTML của chúng trong lượt này. W02 chỉ có tab Local Install, chưa có thiết kế chi tiết import; P4.1 phải trình review import trong shell hiện có trước coding UI đó.

## Verification đã thực hiện trong lượt review

- Chạy lại `npm run test:sds` tại worktree HEAD trên: **11 test files, 36 tests passed**, exit 0. Đây là tests pilot, không chứng minh Phase 3–5.
- Có warning Browserslist data cũ; không tự update dependency trong task tài liệu.
- Không chạy lại Rust/build/installer/live Store trong lượt này. Evidence cũ ghi SDS Rust 106 passed/4 ignored; full default 548 passed/3 failed/7 ignored; full features sds 654 passed/3 failed/11 ignored. Ba lỗi setup upstream gồm hai symlink Windows 1314 và partial-clone diagnostic assertion; giữ nguyên trạng thái fail, không gọi suite pass.
- Đã kiểm tra liên kết local mới, task/interface consistency và coverage SRS sau khi viết plan. Các checklist triển khai vẫn để trống.

## Ma trận sở hữu yêu cầu

| Yêu cầu | Nơi phát triển chính | Nơi hồi quy/bằng chứng |
|---|---|---|
| AUTH-01/02, CAT-01..04 | Phase 1 + Phase 2 | P4.2/P4.4, P5.1 |
| INS-01/02/06/07, MCP-05 | P3.1–P3.5 | P5.1 |
| INS-03..05, MCP-01..04 | Phase 1; mở rộng local/batch P3.3/P4.1 | P3.5, P5.1 |
| PRE-01 | P3.4 | P3.5, P5.1 |
| UPD-01..04, BAK-01, REM-02, OFF-01 | G2/Phase 2 bắt buộc | P3.5, P4.1–2, P5.1–2 |
| REM-01 | Phase 1 + G2 bảo vệ bản sửa | P3.5, P4.1, P5.1 |
| CAT-05 | P4.1 | P5.1 |
| LIB-01 | P4.2, batch selection P3.4 | P5.1 |
| SET-01, LOG-01 | P4.3–4 | P5.1–3 |
| Packaging/network/accessibility/performance | P5.1–4, kiểm tra UI mỗi phase | Biên bản P5.4 |

## Thứ tự và ước lượng

`P2.1 → … → P2.9 → G2 → P3.1 → P3.2 → P3.3 → P3.4 → P3.5 → P4.1 → P4.2 → P4.3 → P4.4 → P5.1 → P5.2 → P5.3 → P5.4`.

Ước lượng lập kế hoạch, không phải cam kết: Phase 3 12–18 ngày công; Phase 4 12–18; Phase 5 8–12. Tổng 32–48 ngày công, chưa gồm Phase 2, thời gian chờ IT/agent owners và sửa lỗi lớn phát hiện khi tích hợp. Re-estimate sau P3.1 và P4.1. Không gán ngày phát hành khi chưa có G2 và deployment profile công ty.

Phase 2 mới có ước lượng sơ bộ15–23 ngày công; tổng Phase2–5 là47–71 ngày công trước thời gian chờ và sửa lỗi lớn. Các số này là allowance lập kế hoạch, chưa đo velocity thực tế; re-estimate sau P2.3.

Các quyết định còn cần evidence ở đúng mốc: agent versions/flavors và Codex root (P3.2), chi tiết Local Install UI (P4.1), API app-release/kênh tải nội bộ (P4.4), Windows/architecture/WebView2/signing/proxy/CA và owner nghiệm thu (P5.2–4). Có thể làm fixtures trước; thiếu evidence thật phải giữ gate tương ứng mở.
