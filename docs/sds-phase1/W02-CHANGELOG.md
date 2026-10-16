# W02 — Đồng bộ thay đổi từ Google Stitch

Ngày 2026-10-16 · Revision 0.2 · Chỉ cập nhật thiết kế/tài liệu.

Nguồn: [project Stitch](https://stitch.withgoogle.com/projects/9226348915970684263), màn `W02 Skills Store (#store)`, screen `e921a2c10a034eca80e4af20e760ad1f`, HTML asset `17399203032189606408`. Đọc từ API Stitch và HTML export ngày 2026-10-16; không sửa project Stitch. Bản W02 không có hậu tố là bản khác, không dùng cho lần đồng bộ này.

## Thay đổi quan sát được

| Thành phần | Trước | W02 mới |
|---|---|---|
| Tiêu đề nội dung | Skills Store | Install Skills; sidebar vẫn Skills Store |
| Tab | Không có | Store (đang chọn), Local Install |
| Chế độ khám phá | Recently updated / Name A–Z | All Time (đang chọn), Trending, Hot |
| Tìm kiếm | Search + dropdown role/agent/sort | Ô search cạnh nhóm chế độ; không còn dropdown role/agent/sort |
| Bộ lọc | All categories và category pills | Source: All sources, Quality, Security, DevOps, AI/ML, Documentation, Other, Requirements, Design, Development, Testing/QA, Deployment, Operations/Common; nút … |

Card grid, metadata skill, agent badges, link detail và pagination vẫn hiện diện. Role/agent còn là metadata, không còn là bộ lọc hiển thị trên W02.

## Đối chiếu tiếp revision 0.3 — 2026-10-16

Screen chính vẫn là `e921a2c10a034eca80e4af20e760ad1f`; HTML asset hiện là `12548026123402487532`. [HTML và screenshot đã lưu](design-snapshots/2026-10-16-stitch-01/README.md). Tab/mode/Source/grid/pagination **Unchanged** so với revision 0.2. Shell/brand AX Portal, typography, tokens và density **Modified**. Nội dung disclosure empty/loading/offline **Modified**, vẫn chứa agent-filter/SSO copy ngoài phạm vi. Search public placeholder vẫn chưa được sửa trong export; tài liệu tiếp tục dùng “Search Skills Store...”.

Không có HTML Stitch cũ đã lưu để diff byte; so sánh với changelog/local HTML. Bản W02 phụ cũng được lưu nhưng không thay mapping chính. P01/P09/V07 và các đề xuất bên dưới giữ nguyên. Xem [báo cáo toàn bộ](STITCH-SYNC-2026-10-16.md) cho before/after và review issues.

## Ngữ nghĩa đề xuất để triển khai, chưa được xác nhận bằng wireframe

- Store là tab mặc định. Giữ keyword/chế độ/Source khi quay về từ detail hoặc Local Install; đổi điều kiện đưa pagination về trang 1. Đề xuất tìm khi Enter hoặc debounce 300 ms, chỉ hiển thị kết quả của request mới nhất.
- All Time / Trending / Hot là enum chế độ catalog, không phải bộ lọc khoảng ngày do client tự tính. Mặc định All Time. Đội Store cần chốt metric, cửa sổ thời gian, công thức và thứ tự All Time; chưa gán chúng thành download count/rating.
- Source là nhóm taxonomy trong Store nội bộ, không phải chọn host public/Git. Đề xuất chọn một Source; All sources bỏ điều kiện. Nút … mở các lựa chọn còn lại từ taxonomy. Tên và ID do API cung cấp; cần đội Store xác nhận mapping với taxonomy hiện tại.
- Local Install xuất hiện trong thiết kế nhưng nội dung tab chưa được cung cấp. P01 được mở lại: cần chốt chọn ZIP/7z, folder hay chỉ gói đã tải từ Store, metadata MCP và cách nhận diện update. Không suy ra quyền import tùy ý hoặc tự dựng luồng mới từ một nhãn tab.
- Export còn placeholder `Search skills.sh market...` và đoạn trạng thái nhắc SSO/agent filter. Đây là nội dung lệch với phạm vi đã chốt. Bản tài liệu/local dùng `Search Skills Store...`, tài khoản Store và keyword/Source/mode; không thêm truy cập skills.sh hoặc SSO. Stitch được giữ nguyên để người dùng chỉnh.

## Tài liệu đồng bộ

SRS CAT-01/04/05, SAS catalog query/cache, API draft search/taxonomy, T00/T02 trong kế hoạch, README P01/P09/V07, wireframe guide, W02 trong HTML cục bộ và transfer metadata. Ảnh preview-store.png cũ không đại diện revision này; xem HTML hoặc Stitch.
