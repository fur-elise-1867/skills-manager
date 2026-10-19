# AX Portal — Gemini Development and Test Implementation Plan

> **For agentic workers:** Gemini thực hiện tuần tự bằng `superpowers:executing-plans` nếu skill khả dụng; dùng checkbox để ghi tiến độ. Codex review độc lập, đánh giá chất lượng và nghiệm thu sau bàn giao. Chỉ tài liệu này được tạo ở bước lập kế hoạch; Gemini chưa được khởi chạy.

**Goal:** Sửa Search trên backend đang chạy, xóa thanh tên nội bộ được đánh dấu trong ảnh và đổi nhận diện bản desktop SDS thành **AX Portal**, có logo/icon mới và bằng chứng kiểm thử.

**Architecture:** Truy vết React → Tauri IPC → Rust StoreClient → Store API trước khi sửa Search. Chỉ sửa lớp gây lỗi đã được chứng minh; nếu cần API Web thì dùng adapter DTO riêng và giữ ranh giới phiên đăng nhập/quyền truy cập. Branding áp dụng theo profile SDS, dùng chung tài sản cho sidebar, favicon và icon Windows.

**Tech Stack:** React 19, TypeScript, Vitest/Testing Library, Vite, Tauri 2/Rust; Store Spring Boot/PostgreSQL đang chạy.

**Spec:** Yêu cầu người dùng trong chat ngày 2026-10-19 và acceptance criteria trong tài liệu này là brief cho lần sửa Search/AX Portal. Các plan customer-review và gemini-codex-work-split cùng ngày là bối cảnh lịch sử; phạm vi/phân công của tài liệu này thay thế chúng **chỉ cho lần sửa này**. Không tự triển khai lại 11 mục review cũ.

## Global Constraints

- Workspace thực hiện: `D:\Documents\Git Project\skills-manager-app\.worktrees\sds-phase0-1`, branch quan sát lúc lập plan: `gemini/sds-uat-ui`, HEAD: `47ac5dd`. Gemini đọc lại HEAD/status trước sửa vì có thể đã thay đổi.
- DB, BE và FE đã được người dùng chạy. Kiểm tra tiến trình/cổng trước mọi lệnh khởi chạy; dùng instance hiện có. Native rebuild/restart chỉ thực hiện cho tiến trình app liên quan khi cần áp dụng thay đổi; ghi rõ instance và thời điểm.
- Có thay đổi sẵn trong `dist-sds/**` và các tài liệu/untracked file, gồm `Customer_Review.docx`. Ghi baseline; giữ nguyên phần của người dùng. Khi build thay output tracked, ghi nhận diff trước/sau và chỉ stage file của task. Không dùng `git add .`.
- Gemini là người sửa code duy nhất trong giai đoạn phát triển. Codex bắt đầu review sau handoff. Làm trực tiếp trong worktree được chỉ định; không tạo hai checkout/process cùng ghi source này.
- Tên hiển thị chính xác: `AX Portal`. Giữ app identifier `com.sds.skillsmanager.desktop`, binary `sds-skills-manager`, runtime profile `sds`, data root `SDS/SkillsManager` và credential keys. Đổi identity/storage cần migration riêng, ngoài phạm vi.
- Giữ profile upstream hoạt động với tên/icon hiện có. Không replace toàn repo các chuỗi `Skill Manager`, `Skills Manager`, `SDS` trong i18n, Rust crate, API, storage, lịch sử tài liệu hoặc generated files.
- Backend/DB và repository Store là read-only cho task này. Không bật `desktop-pilot`, seed/migrate DB, sửa quyền hoặc thay cấu hình BE để né lỗi client. Nếu cần backend change, bàn giao blocker có evidence cho Codex.
- Test ghi dữ liệu dùng temp/fixture biệt lập. Live smoke chỉ đọc catalog/taxonomy/detail và dùng phiên đăng nhập được cấp; không cài/xóa skill thật để chứng minh Search.
- Credentials/token chỉ tồn tại trong session hoặc biến môi trường cần thiết; log/report/screenshot được che thông tin nhạy cảm. Không đọc/xuất credential vault để lấy token.
- Lỗi Search có regression test thực sự bắt lỗi. Branding/CSS/assets kiểm tra bằng build và ảnh native thực tế; tránh test chỉ so khớp chuỗi source.
- Mỗi gói thay đổi có commit riêng khi tách được; commit chỉ gồm file task. Gemini không tự merge/push hoặc tuyên bố nghiệm thu cuối.

## Review Focus

1. Nhấn Search với cùng từ khóa sau lỗi phải tạo request mới, kể cả sau debounce; G2 kiểm tra retry và số lần request.
2. HTTP 401/403/404/5xx, taxonomy lỗi và Store mất mạng phải phân loại đúng; G1/G2 xác minh endpoint đã xác thực, G5 chạy lỗi giả lập có kiểm soát.
3. Response chậm từ query/account cũ không được xuất hiện ở phiên mới; G2 giữ và mở rộng race tests hiện có.
4. Bỏ titlebar phải bỏ cả hàng grid 32px và đặt lại environment label hợp lý; G3/G5 kiểm tra hình native ở 1440×860 và 1100×640, light/dark.
5. Branding phải đến được icon cửa sổ/taskbar/bundle mà giữ profile/storage cũ; G4/G5 kiểm tra config merge, binary thực tế và settings còn giữ sau restart.

## Phân công và thứ tự

| Gói | Người thực hiện | Đầu ra / phụ thuộc |
|---|---|---|
| G0 Baseline | Gemini | HEAD/status, services, checks ban đầu |
| G1 Chứng minh nguyên nhân Search | Gemini | Request/status/contract đã xác thực; sau G0 |
| G2 Sửa Search và regression | Gemini | Source + test; sau G1 |
| G3 Bỏ thanh tên và đổi copy | Gemini | Shell/layout/branding copy; sau G2 |
| G4 Logo/icon và tích hợp | Gemini | Bộ tài sản + cấu hình build; sau G3 |
| G5 Test tích hợp và handoff | Gemini | Checks + native smoke + báo cáo; sau G2–G4 |
| R1 Review chất lượng | Codex | Diff/test/evidence độc lập; sau G5 |
| R2 Nghiệm thu | Codex | Kết luận Accepted / Changes requested / Blocked |

G1 có thể chứng minh sửa cần ít file hơn dự kiến: ưu tiên sửa nhỏ có bằng chứng. Nếu cần adapter Web, giới hạn ở luồng đọc để Search và mở kết quả hoạt động. Download/install/updates engine mới, rich-detail parity đầy đủ và thay DB không thuộc task; ghi rõ limitation và ảnh hưởng thay vì tự mở rộng.

## G0 — Chốt baseline và môi trường

**Read:** `package.json`, `scripts/sds.mjs`, `vite.config.ts`, `src-tauri/tauri.sds*.json`, hướng dẫn repository ở các thư mục cha nếu có.

**Create:** `docs/superpowers/evidence/2026-10-19-ax-portal-gemini-handoff.md` và thư mục evidence `docs/superpowers/evidence/ax-portal/` khi có file cần lưu.

- [ ] Ghi `git status --short`, `git rev-parse HEAD`, branch và diff ban đầu; phân biệt file task với thay đổi sẵn.
- [ ] Xác định native app đang chạy, dev server app (config hiện tại `127.0.0.1:1420`), Store Web và BE. Base tham khảo `http://localhost:8080/skm`, Web `http://localhost:5173/skm/skills`; đối chiếu instance thật và connection trong Settings.
- [ ] Chạy `npm run test:sds`, `npm run lint` và `node --test scripts/sds.test.mjs scripts/sds-fixtures.test.mjs scripts/sds-bundle.test.mjs`. Ghi command, exit code và failure baseline theo tên; lỗi có sẵn phải được phân biệt với regression mới.
- [ ] Chụp native app ban đầu: vùng tên bị khoanh, sidebar, Store/error và Search. Không dùng ảnh mock làm evidence.

**Done:** report có base HEAD, danh sách thay đổi sẵn, service URLs, baseline checks và ảnh trước sửa.

## G1 — Tái hiện và xác minh nguyên nhân Search

**Read:** `src/features/sds/catalog/{CatalogPage.tsx,useCatalog.ts}`, `api.ts`, `session/SessionProvider.tsx`; native `src-tauri/src/sds/{commands.rs,store_client.rs,session.rs,contracts.rs}` và `tests/catalog.rs`.

**Store read-only references:** `D:\Documents\Git Project\skills-manager-app\skills-store\skill-management\backend\src\main\java\com\furelise\skillmanagement\desktop\controller\DesktopCatalogController.java`; `skill/controller/SkillController.java`, `skill/dto/{SkillSearchRequest.java,SkillListItem.java,SkillSearchResult.java}`; frontend callers tương ứng.

**Facts đã đọc:** `StoreClient` dùng `/api/v1/desktop/skills` và `/api/v1/desktop/taxonomy`. `DesktopCatalogController` có `@Profile({"desktop-pilot","desktop-test"})`. API Web có DTO query khác (`skillName`, `description`, category/stage...). `CatalogPage` vừa debounce 300ms vừa submit. `useCatalog` có key gồm account/query/revision và guard response cũ. Chưa có bằng chứng request đã xác thực của native đang lỗi ở lớp nào.

- [ ] Tái hiện click Search và Enter với từ khóa tồn tại, từ khóa không tồn tại, rỗng; ghi actual/expected và request thực sự phát sinh.
- [ ] Xác minh base URL có `/skm`, transport/proxy, session, route, HTTP status, envelope và decode. Ghi trace đã che token tại từng ranh giới IPC → native → HTTP; ưu tiên log sẵn hoặc instrumentation tạm được gỡ trước bàn giao.
- [ ] Kiểm tra endpoint **đúng** `/api/v1/desktop/skills`, taxonomy và API Web bằng session hợp lệ. Kết quả 401 của probe chưa xác thực không chứng minh route tồn tại hoặc thiếu; `/desktop/catalog` không phải route catalog của code hiện tại.
- [ ] Đối chiếu DTO/filter/pagination và active backend profile với source/runtime evidence. Kiểm tra offline badge có do `/desktop/me` hoặc retry taxonomy thất bại độc lập với Search hay không.
- [ ] Viết một kết luận root cause: trigger → request → failure → phạm vi fix. Nếu không có session phù hợp, ghi phần live là Blocked, tiếp tục test contract/branding; không tự tạo tài khoản hoặc đoán credentials.

**Done:** có request/status hoặc bằng chứng runtime tương đương đã xác thực, test case tái hiện và quyết định lớp cần sửa. Không mặc định đổi toàn bộ API chỉ vì UI hiển thị offline.

## G2 — Sửa Search, giữ contract và xử lý trạng thái

**Modify khi liên quan:** `catalog/CatalogPage.tsx`, `catalog/useCatalog.ts`, `catalog/CatalogPage.test.tsx`; `api.ts`, `contracts.ts`, `errors.ts`, `session/SessionProvider.tsx` và tests tương ứng nếu G1 chứng minh cần.

**Native conditional scope:** `src-tauri/src/sds/store_client.rs`, `session.rs`, `contracts.rs`, `commands.rs`, `tests/catalog.rs`, `tests/session.rs`. Nếu cần Web adapter: tạo `src-tauri/src/sds/store_web.rs`, đăng ký `mod.rs`, fixture contract đã che dữ liệu trong `contracts/store-web/v1/`.

**Interfaces:** giữ IPC `sds_catalog_search({query: CatalogQuery}) -> Page<SkillSummary>` và `sds_catalog_taxonomy() -> Taxonomy` nếu dữ liệu thật ánh xạ trung thực được. Nếu phải đổi model thì cập nhật TS/Rust/callers/tests cùng commit. `Release`, quyền download, compatibility và MCP không được suy diễn từ dữ liệu Web thiếu trường.

- [ ] Viết regression test nhỏ nhất tái hiện root cause G1; chạy để thấy FAIL đúng nguyên nhân trước khi sửa logic.
- [ ] Sửa lớp được xác định; query trim, Search/Enter và debounce dùng cùng từ khóa, reset về page=1. Submit cùng query phải refresh kết quả/retry sau lỗi; tránh request thừa do effect/debounce.
- [ ] Nếu cần adapter Web, dùng DTO riêng theo response thật, map `totalCount`/numeric IDs/null đúng kiểu. Xác minh semantics tìm tên/mô tả từ BE: các field Web có thể AND, không gửi cùng keyword vào cả hai rồi quảng cáo OR. Chỉ dùng filter được API hỗ trợ; đổi nhãn/placeholder thành hành vi thật nếu không có OR search, ghi khác biệt vào handoff.
- [ ] Khi dùng API Web, allowlist chỉ mở các route cần thiết dưới cùng origin/base path; giữ chống traversal/redirect/token leakage. Giữ generation/account guard xuyên request; lỗi permission/auth không chuyển thành empty state hoặc offline chung.
- [ ] Đảm bảo bấm View kết quả vẫn mở được detail hợp lệ. Nếu Web không có release package metadata thì hiển thị khả năng thật; không tạo release/hash giả để giữ nút install. Ghi mọi thay đổi capability trong report để Codex review.
- [ ] Test các hành vi: click/Enter; từ khóa tiếng Việt/Unicode và `&`, `+`, `%`; rỗng/whitespace; không kết quả; pagination và filters thật; cùng query retry; response cũ; đổi account/logout khi request đang chạy; 401 refresh/expired, 403, 404, 5xx; taxonomy lỗi trong khi search thành công; phục hồi sau lỗi mà không loop request.
- [ ] Test native nếu sửa: route/context `/skm`, encode params, envelope sai/null, page/pageSize bounds, allowlist và phiên cũ. Giữ test desktop fixture hiện có nếu runtime fixture vẫn dùng contract đó; không thay fixture để che sai contract live.
- [ ] Chạy `npm exec -- vitest run src/features/sds/catalog/CatalogPage.test.tsx` cùng session/offline tests bị ảnh hưởng. Chạy Rust filtered tests cho module sửa bằng launcher/env như G5. Ghi RED/GREEN; commit `fix(sds): restore catalog search against configured store`.

**Done:** regression xanh và Search trên native có dữ liệu live, hoặc ghi rõ live Blocked với nguyên nhân cụ thể. Mock/fixture xanh một mình chưa đủ nghiệm thu Search.

## G3 — Bỏ thanh tên và đổi nhận diện hiển thị

**Modify:** `src/features/sds/SdsShell.tsx`, `SdsApp.tsx`, `sds.css`, `SdsApp.test.tsx`; `src-tauri/tauri.sds.conf.json`; `vite.config.ts`/`src/main.tsx` khi cần metadata profile.

**Interfaces:** giữ props `SdsShell({profile})`, navigation, session và operation callbacks. Native `productName`/window title của SDS thành `AX Portal`; identity kỹ thuật giữ theo Global Constraints.

- [ ] Xóa header nội bộ chứa icon Box + `SDS Skills Manager` đúng vùng ảnh; bỏ hàng grid 32px trong `.sds-app` để sidebar và main dùng trọn chiều cao. Giữ thanh tiêu đề native cùng nút minimize/maximize/close.
- [ ] Chuyển environment/fixture label sang vị trí nhỏ gọn trong Settings hoặc sidebar footer; vẫn phân biệt development/internal/fixtures được.
- [ ] Sidebar hiển thị `AX Portal`; thay ô chữ SDS bằng tài sản G4. Giữ subtitle hiện tại nếu không cần chỉnh để fit.
- [ ] Đổi startup error/copy của SDS, native title, productName, document title và favicon theo profile. Metadata HTML cần đúng ngay trong output `build:sds`; dùng Vite transform theo mode nếu cần, tránh đổi `index.html` dùng chung thành AX Portal cho upstream.
- [ ] Điều chỉnh test shell đang assert tên cũ/structure khi cần. Verify light/dark, navigation Store/Library/Workspaces/Settings, keyboard focus và min size 1100×640.
- [ ] Commit `feat(sds): rename desktop shell to AX Portal`.

**Done:** hết hàng tên nội bộ và khoảng trống tương ứng; tên AX Portal đồng nhất trong app SDS, profile upstream còn đúng.

## G4 — Tạo logo/icon và tích hợp desktop

**Create:** `public/ax-portal/logo.png`, `public/ax-portal/icon.png`, `public/ax-portal/favicon.png`; `assets/ax-portal/branding.md`; native icons dưới `src-tauri/icons/ax-portal/`.

**Modify:** `SdsShell.tsx`, `sds.css`, SDS metadata transform và `src-tauri/tauri.sds.conf.json` (`bundle.icon`). Kiểm tra `src-tauri/src/sds/runtime.rs`/build config nếu default window icon cần hook riêng. Đọc `scripts/sds*.test.mjs` trước thay config.

**Design brief:** AX Portal là cổng workspace nội bộ. Mark đơn giản gợi chữ AX/cánh cổng, ưu tiên nhận diện ở 16–32px; xanh chính `#1657b8`, điểm nhấn cyan nhẹ; nền ngoài trong suốt. Wordmark chính xác `AX Portal`, không slogan mới. Mark dùng được trên light/dark.

- [ ] Tạo một bộ mark + wordmark nhất quán bằng công cụ tạo ảnh khả dụng; lưu prompt/tool/source vào `branding.md`. Nếu dùng vector làm nguồn, lưu SVG có thể chỉnh sửa và xuất PNG/ICO; không dùng asset tạm, chữ SDS hoặc icon Vite làm bản giao cuối.
- [ ] Xuất icon PNG 32/128/256/512 và ICO Windows có các cỡ 16/24/32/48/64/128/256. Dùng Tauri icon CLI hoặc công cụ conversion cục bộ đã có; ghi command để tái tạo. Logo/sidebar phải vừa kích thước thực, không chỉ đẹp ở ảnh lớn.
- [ ] Lưu asset được tham chiếu trong repo, wiring sidebar + favicon + native bundle/default window icon đồng bộ. Ghi rõ logo và mark khác nhau, dùng mark ở taskbar/favicon.
- [ ] Kiểm tra config merge thực tế qua `makeLaunch` trong `scripts/sds.mjs`; icon SDS không bị kế thừa từ upstream. Giữ binary name và installer process detection hiện tại.
- [ ] Kiểm tra ảnh có alpha, đúng kích thước/format, nét ở 16/32px; chụp light/dark và icon native thật sau rebuild/restart. Nếu Windows cache giữ icon cũ thì ghi rõ cache/instance, xác minh resource của binary mới trước kết luận.
- [ ] Commit `feat(sds): add AX Portal logo and desktop icons`.

**Done:** bộ asset nguồn và output có trong workspace, tham chiếu hợp lệ; hình thực tế và app binary dùng đúng icon.

## G5 — Test tích hợp, native smoke và bàn giao

- [ ] Chạy checks cuối sau thay đổi G2–G4; ghi command, môi trường, exit code, số test và log path:

```powershell
npm run test:sds
npm run lint
node --test scripts/sds.test.mjs scripts/sds-fixtures.test.mjs scripts/sds-bundle.test.mjs
npm run build:sds
npm run build
npm run sds:check -- --environment development
```

- [ ] Chạy native suite với profile/config nhất quán launcher, PowerShell từ workspace:

```powershell
$axPriorConfig = $env:TAURI_CONFIG
$axPriorEnvironment = $env:SDS_ENVIRONMENT
$axPriorFixtures = $env:SDS_FIXTURES
try {
  $env:TAURI_CONFIG = Get-Content -Raw src-tauri/tauri.sds.conf.json
  $env:SDS_ENVIRONMENT = 'development'
  $env:SDS_FIXTURES = '0'
  cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --features sds --lib
  $axNativeExit = $LASTEXITCODE
} finally {
  $env:TAURI_CONFIG = $axPriorConfig
  $env:SDS_ENVIRONMENT = $axPriorEnvironment
  $env:SDS_FIXTURES = $axPriorFixtures
}
# Ghi $axNativeExit vào report; code khác 0 là failure.
```

- [ ] Nếu native build cần env/linker/toolchain bổ sung, dùng cấu hình repository hiện có và ghi lại chính xác. Không gọi native PASS khi chưa chạy được. Lỗi baseline cũng phải có tên và tác động.
- [ ] Build native development bằng `npm run sds:build -- --environment development --debug --no-bundle` để xác minh executable/icon. Không thay instance live bằng artifact lỗi; xác định output theo launcher thực tế. Chỉ build installer nếu thay đổi resource/config ảnh hưởng packaging cần kiểm chứng; không phát hành/cài đè app của người dùng.
- [ ] `scripts/sds-installer.test.mjs` đọc NSIS script sinh tại `src-tauri/target/<profile>/nsis/x64/installer.nsi`; chỉ chạy `node --test scripts/sds-installer.test.mjs` sau khi đã build NSIS cùng revision, đặt `SDS_INSTALLER_PROFILE=debug` hoặc `release` đúng artifact. Nếu không build installer, báo Not run thay vì kiểm tra artifact cũ hoặc báo lỗi thiếu file là regression.
- [ ] Mở native binary đã build, fixture mode tắt, nối BE hiện có. Smoke: tìm keyword thật → xem kết quả → mở detail → quay lại → từ khóa không tồn tại → clear → Search cùng query → restart kiểm tra title/icon/settings. Dùng account hiện tại được cấp.
- [ ] Kiểm tra mất mạng/lỗi server bằng test transport/proxy cô lập; không dừng BE chung. Native live phải có ít nhất bằng chứng Search đọc dữ liệu thật. Đối chiếu keyword với dữ liệu/API thật thay vì chỉ thấy request 200.
- [ ] Chụp `store-search-light.png`, `store-search-dark.png`, `native-title-icon.png`, `minimum-window.png`; lưu cạnh report trong `ax-portal/`. Ảnh có thông tin nhạy cảm được che trước lưu.
- [ ] Review `git diff --check`, diff source/tests/config/assets và status cuối. Loại instrumentation tạm, không stage build output có trước nếu chưa phân biệt ownership được.
- [ ] Hoàn tất handoff theo mẫu bên dưới, commit riêng docs/evidence task. Dừng phát triển sau bàn giao để Codex review trên HEAD cố định.

### Mẫu handoff bắt buộc

1. Base HEAD, final HEAD, branch, commit SHA từng gói và exact file list; thay đổi sẵn được bảo toàn.
2. Root cause Search cùng evidence đã xác thực; trước/sau route/query/status/response shape; capability/filter nào đổi và lý do.
3. Bảng acceptance criteria AC1–AC8: Pass / Fail / Not run, test name hoặc screenshot/log cụ thể.
4. Checks đã chạy: command, exit code, test counts; baseline failures và failures mới tách riêng. Tests native, live, fixture báo riêng.
5. Asset paths, prompt/source, công cụ xuất icon, config effective và executable được kiểm tra.
6. Hướng dẫn tái hiện ngắn cho Codex; limitations/blockers và những checks chưa chạy. Không ghi password/token.

## R1/R2 — Codex review, đánh giá và nghiệm thu

Codex đọc report và diff thực tế, tự chạy lại regression Search, config/build checks liên quan và smoke native live. Khi báo cáo có failure chưa giải thích hoặc bằng chứng không khớp, yêu cầu Gemini bổ sung/sửa theo file, cách tái hiện và expected/actual.

| AC | Tiêu chí nghiệm thu | Bằng chứng tối thiểu |
|---|---|---|
| AC1 | Search click/Enter hoạt động với Store đang chạy, kết quả đúng keyword và pagination | Live native/API đối chiếu + regression |
| AC2 | Query cũ, account cũ, retry cùng query và lỗi auth/network không làm sai UI | Targeted UI/native tests, state/error rõ ràng |
| AC3 | Không còn thanh tên nội bộ hoặc hàng trống 32px; native controls còn hoạt động | Ảnh native light/dark và min size |
| AC4 | AX Portal nhất quán ở sidebar, window title, SDS HTML và productName | Native screenshot + build/config evidence |
| AC5 | Logo/favicon/window/taskbar icon mới dùng được, có source và output trong repo | Assets + native binary/resource evidence |
| AC6 | Identifier/storage/credential/profile giữ tương thích; upstream không bị đổi branding | Config/source review + restart settings + upstream build |
| AC7 | Test/lint/build phù hợp pass; failure baseline/chưa chạy công khai | Log commands/exit codes, diff tests |
| AC8 | Scope gọn, không đổi DB/BE/dữ liệu người dùng hoặc làm yếu assertions | Diff/commit/status độc lập |

**Đánh giá chất lượng:** Codex ghi từng AC và findings P0–P3, phân biệt lỗi baseline với regression. P0–P2 ảnh hưởng yêu cầu phải được sửa trước nghiệm thu; P3 cosmetic có thể ghi nhận riêng nếu không vi phạm AC. Không lấy điểm trung bình để che một AC bắt buộc chưa đạt.

**Kết luận:** `Accepted` khi AC1–AC8 có evidence đạt; `Changes requested` khi có lỗi sửa được; `Blocked` khi thiếu access/toolchain/live evidence làm không thể nghiệm thu. Fixture-only không đủ Accepted cho Search. Codex lưu kết quả ở `docs/superpowers/evidence/2026-10-19-ax-portal-codex-review.md`, người dùng quyết định phát hành.

## Prompt giao Gemini

> Thực hiện G0 → G5 trong `docs/superpowers/plans/2026-10-19-ax-portal-gemini-development-test.md` tại `D:\Documents\Git Project\skills-manager-app\.worktrees\sds-phase0-1`. Bạn sở hữu phần phát triển và test Search + AX Portal; Codex sẽ review độc lập và nghiệm thu sau handoff. Đọc Global Constraints và từng gói trước khi sửa. Đặc biệt: chứng minh nguyên nhân Search bằng request đã xác thực; backend/DB đã chạy và chỉ được đọc; giữ identity/storage/upstream; bỏ titlebar cả DOM lẫn hàng grid; tạo và tích hợp logo/icon hoàn chỉnh. Ghi baseline, regression RED/GREEN, checks và native live evidence vào report được chỉ định. Mỗi gói commit riêng với file scope rõ ràng; không merge/push. Bàn giao final HEAD, kết quả AC1–AC8 và blockers trung thực, rồi dừng sửa để Codex review.
