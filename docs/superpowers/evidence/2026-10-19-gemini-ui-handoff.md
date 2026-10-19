# Báo cáo Handoff Gemini — SDS UAT Remediation (G1, G2, G3)

**Thời điểm:** 2026-10-19  
**Người thực hiện:** Gemini (Agentic AI)  
**Tài liệu lập kế hoạch:** [docs/superpowers/plans/2026-10-19-gemini-codex-work-split.md](file:///d:/Documents/Git%20Project/skills-manager-app/.worktrees/sds-phase0-1/docs/superpowers/plans/2026-10-19-gemini-codex-work-split.md)  
**Tài liệu tham chiếu:** [docs/superpowers/plans/2026-10-19-customer-review-remediation.md](file:///d:/Documents/Git%20Project/skills-manager-app/.worktrees/sds-phase0-1/docs/superpowers/plans/2026-10-19-customer-review-remediation.md)  

---

## 1. Thông tin Git, Branch và Commits

- **Base HEAD:** `5f60458000c5a13d0885609600c8b6b4fd55c8ef` (nhánh `feature/sds-uat1`)
- **Checkout / Branch:** `gemini/sds-uat-ui`

### Danh sách commits theo thứ tự thực hiện:

1. **G1 — `2ded3c2`** `fix(sds): simplify library controls for UAT`
   - `src/features/sds/library/LibraryPage.tsx`
   - `src/features/sds/library/LibraryPage.test.tsx`
   - `src/features/sds/offline/OfflineFlow.test.tsx`
2. **G2 — `7e34b38`** `fix(sds): separate project workspace selection from global install`
   - `src/features/sds/workspace/WorkspacePage.tsx`
   - `src/features/sds/workspace/WorkspaceFlow.test.tsx`
3. **G3 — `9a9032d`** `fix(sds): consolidate appearance settings and hide deferred navigation`
   - `src/features/sds/SdsShell.tsx`
   - `src/features/sds/catalog/CatalogPage.tsx`
   - `src/features/sds/SdsApp.test.tsx`
   - `src/features/sds/catalog/CatalogPage.test.tsx`
   - `src/features/sds/settings/AppearanceSettings.tsx` (tạo mới)
   - `src/features/sds/settings/AppearanceSettings.test.tsx` (tạo mới)

---

## 2. Bảng Tiêu chí Nghiệm thu (Acceptance Criteria)

| Gói | Tiêu chí nghiệm thu | Trạng thái | Test name / Bằng chứng |
|---|---|---|---|
| **G1** | Bỏ cụm nút All/Installed/Available/Refresh Library | **Pass** | `LibraryPage.test.tsx`: "shows independent cache and target status locally, searches paths and does not expose deferred preview or install controls" |
| **G1** | Giữ ô search và tự tải danh sách local | **Pass** | `LibraryPage.test.tsx`: assert search filter hoạt động |
| **G1** | Bỏ Preview và toàn bộ khung Skill document; không gọi `getLibraryPreview` | **Pass** | `LibraryPage.test.tsx`: assert không có nút Preview, invoke calls chỉ có `["sds_library"]` |
| **G1** | Bỏ Install skill và Use cached copy trong mọi trạng thái session (online, offline, expired, signedOut) | **Pass** | `OfflineFlow.test.tsx`: test 1 & 2 assert các nút bị ẩn trong mọi session |
| **G1** | Giữ Delete, View destinations, Remove from target; giữ `disabled` của Delete khi có destinations | **Pass** | `LibraryPage.test.tsx`: assert Delete disabled khi có destination, `OfflineFlow.test.tsx`: Delete mở modal delete plan |
| **G1** | Sửa copy chỉ dẫn bấm Refresh Library thành chỉ dẫn thao tác còn tồn tại | **Pass** | `LibraryPage.tsx`: cập nhật copy trong reveal ("Check its location and try again") và snapshot ("Check the target and try again") |
| **G1** | Giữ nguyên public props/contracts của `LibraryPage` | **Pass** | `LibraryPage.tsx`: các props `onInstall`, `onCachedInstall` được giữ trong type definition |
| **G2** | Install picker: chỉ hiện Add project folder khi `scope === "project"`; Global ẩn nút | **Pass** | `WorkspaceFlow.test.tsx`: "install picker defaults to Global without Add project folder button and sends global target" |
| **G2** | Install picker: Global vẫn gửi `workspaceId:'global'` và không cần có project đăng ký | **Pass** | `WorkspaceFlow.test.tsx`: click Review gửi `{agentKey:"aipro",workspaceId:"global"}` |
| **G2** | Trang Workspaces: bỏ option Global AIPro khỏi selector Registered | **Pass** | `WorkspaceFlow.test.tsx`: `expect(screen.queryByRole("option",{name:"Global AIPro"})).not.toBeInTheDocument()` |
| **G2** | Trang Workspaces: khi load xong chọn project đầu tiên nếu có; nếu không chọn rỗng | **Pass** | `WorkspaceFlow.test.tsx`: `expect(select.value).toBe("atlas")` |
| **G2** | Trang Workspaces: khi xóa project đang chọn, chọn project còn lại đầu tiên hoặc rỗng, tuyệt đối không fallback về global | **Pass** | `WorkspaceFlow.test.tsx`: "workspace view removal of current project selects remaining project and never falls back to global" |
| **G2** | Bỏ dòng path lặp trực tiếp dưới selector; giữ tên/path trong option và bảng registration/deployment | **Pass** | `WorkspacePage.tsx`: bỏ `{selected && <p className="sds-target-path">...}` |
| **G2** | Empty state không render Library cho global hoặc selected ID không tồn tại | **Pass** | `WorkspaceFlow.test.tsx`: "workspace empty state does not render installed skills or global library" |
| **G3** | Ẩn Backup khỏi navigation và render branch trong `SdsShell` | **Pass** | `SdsApp.test.tsx`: `expect(screen.queryByRole("button", { name: "Backup" })).not.toBeInTheDocument()` |
| **G3** | Bỏ mount `ActivitySettings` trong Settings (không tắt log native) | **Pass** | `SdsApp.test.tsx`: `expect(screen.queryByRole("heading", { name: "Activity logs" })).not.toBeInTheDocument()` |
| **G3** | Bỏ theme toggle ở sidebar; chuyển vào `AppearanceSettings` trong Settings | **Pass** | `SdsApp.test.tsx`: sidebar không có toggle; Settings có `AppearanceSettings` |
| **G3** | Toggle có accessible name `Switch to dark theme` / `Switch to light theme` theo state | **Pass** | `AppearanceSettings.test.tsx`: assert accessible name đổi theo state |
| **G3** | Đổi theme đúng, dataset trên document đổi, provider remount đọc lại `sds.theme`; localStorage throw vẫn đổi được trong session | **Pass** | `AppearanceSettings.test.tsx`: 3 tests với provider thật, verify dataset, remount, và DOMException |
| **G3** | Bỏ entry point Local Install khỏi `CatalogPage`, không ảnh hưởng Store install | **Pass** | `CatalogPage.test.tsx`: `expect(screen.queryByRole("button",{name:"Local Install"})).not.toBeInTheDocument()` |
| **Screenshots** | Ảnh chụp UI thực tế sau sửa (Library, Workspaces, Picker, Settings) | **Not run** | Môi trường CLI không có desktop native display/Tauri window server. Theo đúng quy định trong plan: ghi Not run và lý do, không tạo ảnh mô phỏng. |

---

## 3. Lệnh Thực Thi và Kết Quả Kiểm Tra

### 3.1. Baseline Pre-check
```bash
npm.cmd exec -- vitest run src/features/sds/library/LibraryPage.test.tsx src/features/sds/offline/OfflineFlow.test.tsx src/features/sds/library/Lifecycle.test.tsx src/features/sds/workspace/WorkspaceFlow.test.tsx src/features/sds/install/InstallFlow.test.tsx src/features/sds/SdsApp.test.tsx src/features/sds/catalog/CatalogPage.test.tsx
```
- **Exit code:** 0
- **Kết quả:** 7/7 test files passed (35/35 tests passed)

### 3.2. Targeted Tests từng gói sau khi sửa
- **G1:**
  ```bash
  npm.cmd exec -- vitest run src/features/sds/library/LibraryPage.test.tsx src/features/sds/offline/OfflineFlow.test.tsx src/features/sds/library/Lifecycle.test.tsx
  ```
  - **Exit code:** 0
  - **Kết quả:** 3/3 test files passed (11/11 tests passed)
- **G2:**
  ```bash
  npm.cmd exec -- vitest run src/features/sds/workspace/WorkspaceFlow.test.tsx src/features/sds/install/InstallFlow.test.tsx
  ```
  - **Exit code:** 0
  - **Kết quả:** 2/2 test files passed (14/14 tests passed)
- **G3:**
  ```bash
  npm.cmd exec -- vitest run src/features/sds/SdsApp.test.tsx src/features/sds/catalog/CatalogPage.test.tsx src/features/sds/settings/AppearanceSettings.test.tsx
  ```
  - **Exit code:** 0
  - **Kết quả:** 3/3 test files passed (17/17 tests passed)

### 3.3. Lint Check
```bash
npm.cmd run lint
```
- **Exit code:** 0
- **Kết quả:**
  ```
  UI spec check passed (120 files).
  i18n check passed (es 0 missing, ko 0 missing, zh-TW 0 missing, zh 0 missing).
  0 errors, 0 warnings.
  ```

### 3.4. Build Check
```bash
npm.cmd run build:sds
```
- **Exit code:** 0
- **Kết quả:**
  ```
  tsc -b && vite build --mode sds --outDir dist-sds
  ✓ 2013 modules transformed.
  ✓ built in 3.90s
  ```

### 3.5. Full SDS Test Suite
```bash
npm.cmd run test:sds
```
- **Exit code:** 1
- **Kết quả:**
  - 18 test files passed (62 tests passed)
  - 1 test file failed: `src/features/sds/snapshots/SnapshotFlow.test.tsx` (2 tests)

---

## 4. Báo Cáo Blocker / Điểm Cần Codex Xử Lý

### Blocker: `src/features/sds/snapshots/SnapshotFlow.test.tsx`
- **Nguyên nhân:**
  Trong G3, mục review 11 yêu cầu: *Ẩn Backup bằng cách bỏ navigation và render branch của trang đó khỏi shell*.
  File `SnapshotFlow.test.tsx` mount `<SdsApp />` và cố gắng click `screen.findByRole("button", { name: "Backup" })` để điều hướng vào trang Backup (`SnapshotsPage`).
- **Phạm vi xử lý:**
  Theo bảng phân công `2026-10-19-gemini-codex-work-split.md`:
  - `snapshots/**` được chỉ định là **Read only** đối với Gemini.
  - Gemini bị cấm sửa ngoài write scope đã giao (`SdsShell.tsx`, `CatalogPage.tsx`, `SdsApp.test.tsx`, `CatalogPage.test.tsx`, `AppearanceSettings.tsx`, `AppearanceSettings.test.tsx`).
- **Khuyến nghị cho Codex:**
  Khi Codex tích hợp và xử lý phần Snapshots/Backup (thuộc thẩm quyền Codex sở hữu module native/recovery), Codex có thể điều chỉnh `SnapshotFlow.test.tsx` để mount trực tiếp `<SnapshotsPage />` (component-level integration test) thay vì điều hướng qua sidebar button đã bị ẩn khỏi `SdsApp`.
