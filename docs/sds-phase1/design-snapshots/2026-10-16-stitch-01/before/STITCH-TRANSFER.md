# Google Stitch — wireframe transfer

Ngày: 2026-10-16 · Bộ nguồn: [wireframes.html](wireframes.html)

[Mở project Samsung SDS Skills Manager — Phase 1 Wireframes](https://stitch.withgoogle.com/projects/9226348915970684263)

Project ID: `9226348915970684263`. Visibility được API xác nhận là PRIVATE.

## Phương thức và phạm vi

Kết nối Stitch hiện không có import HTML trực tiếp. Đã dùng HTML wireframe làm tham chiếu để tạo các màn hình Stitch có thể chỉnh sửa. Đây là chuyển đổi bằng công cụ sinh thiết kế, không phải import giữ nguyên từng pixel. Không đưa source backend, credential thật hoặc API nội bộ thực tế lên Stitch; input là wireframe với dữ liệu minh họa.

## Bộ chính W01–W14

Các màn hình chính có hậu tố `(#...)` trong tên, nằm trên hàng chính của canvas. Dùng các màn này khi chỉnh sửa và review.

| Screen | Stitch screen ID |
|---|---|
| W01 Sign in (#login) | `ca797a30d9a14f298953483347c4546f` |
| W02 Skills Store (#store) | `e921a2c10a034eca80e4af20e760ad1f` |
| W03 Skill detail (#detail) | `7dcd2666d80d43dd8ae20556de775abc` |
| W04 Install skill (#install) | `a3cca1f7d09646bca0f0a74a0fbbc1c2` |
| W05 AIPro MCP check (#mcp) | `e297f1843ce04ab9a2189dd690f7f7ee` |
| W06 Installation result (#result) | `727ccba2e7f14f37abce39cfcb41ad8d` |
| W07 Library (#library) | `c4c7355ac0aa416eb275e610be3e916a` |
| W08 Workspace and preset (#workspace) | `629741bbcca2475fabdd7b80e0f29c77` |
| W09 Update review (#update) | `47e723baede14b0cb331426b055b6f8f` |
| W10 Offline Library (#offline) | `9c91f3d8eaae489e887790fa7bfd3113` |
| W11 Backup - local snapshots (#backup) | `9f7fa1d49a4a468bb88a18dc58def5f1` |
| W12 Settings (#settings) | `662d6089aed24f248081b81a4a9fb2ed` |
| W13 Remove skill (#remove) | `7f90de67b26746b3bb047c078b4e9fbe` |
| W14 Dashboard (#dashboard) | `da58f733eb074f4aa714cb22457e12c8` |

## Bản bổ sung

Tại thời điểm chuyển ban đầu project có thêm 5 màn W01–W05 không có hậu tố, do yêu cầu chuyển hàng loạt ban đầu hoàn thành muộn sau timeout. Không xóa tự động các bản này. Đây là các bản tham khảo, không phải thêm yêu cầu sản phẩm.

## Đồng bộ ngược W02 — 2026-10-16

Đã đọc HTML asset `17399203032189606408` của W02 chính sau người dùng chỉnh. Đồng bộ tab và search sang bộ tài liệu revision 0.2; xem [W02-CHANGELOG](W02-CHANGELOG.md). Không sửa màn trên Stitch. Danh mục phía trên là mapping lúc bàn giao ban đầu; project hiện có thêm màn ngoài mapping, không tự xóa hoặc chọn chúng thay W02 chính.

## Rà độ trung thực

Đã xác nhận qua list_screens/get_project có đủ 14 màn chính và thành phần HTML/screenshot cho từng màn. Stitch tự bổ sung một số nhãn phiên bản, SSO, runtime và countdown ngoài nguồn. Đã gửi edit_screens trên 14 màn chính để loại các nhãn này; tool trả thành công cùng các DOM correction events. Các event được lưu tại [stitch-transfer.json](stitch-transfer.json).

Lưu ý: URL HTML xuất ban đầu vẫn trả asset cũ tại lần đối chiếu sau edit; chưa xác minh trực quan bản sửa trong canvas đăng nhập. Người dùng cần rà lại các nhãn này khi chỉnh trên Stitch. Các phiên bản skill thật trong wireframe (1.0.0/1.1.0/1.2.0/1.3.0) cần được giữ. Prototype navigation giữa các screen chưa được xác nhận; mục tiêu bàn giao hiện tại là các màn chỉnh sửa được.

Không sửa code sản phẩm hoặc thực hiện testing trong bước chuyển thiết kế này. Các file trong docs vẫn theo quy tắc ignore hiện có của repo.

