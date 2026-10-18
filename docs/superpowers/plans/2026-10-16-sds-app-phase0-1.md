# SDS App Phase 0–1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bàn giao pilot Windows cho luồng Store local thật → AIPro global → Library → gỡ, có MCP warning, bảo vệ filesystem và bộ cài.

**Architecture:** Mở rộng repo hiện tại bằng profile SDS và các module Rust/React riêng có giao diện nhỏ. Tái sử dụng component/helper phù hợp, nhưng không chạy startup, command hoặc migration dữ liệu upstream trong profile SDS. Làm app bằng contract fixtures trước; API Store chỉ thêm mới theo companion plan là điều kiện kết thúc pilot.

**Tech Stack:** React 19, TypeScript, Vite, Tauri 2, Rust, SQLite, Windows keyring; Spring Boot 3.4.2/Java 21/PostgreSQL cho provider.

**Spec:** [Thiết kế đã duyệt](../specs/2026-10-16-sds-app-delivery-design.md). [Store companion plan](2026-10-16-sds-store-additive-pilot.md).

Trạng thái: **Chờ người dùng review plan và chọn Native/Subagent-driven.** Chưa thực hiện các checkbox dưới đây. Duyệt spec không tự đánh dấu task hoàn thành.

## Global Constraints

- Phát triển tiếp repo app hiện tại; Windows; English mặc định; giữ toàn bộ roadmap 0–5 trong spec.
- Store: chỉ thêm file/folder/package mới; không sửa, di chuyển hoặc xóa file hiện có, kể cả cấu hình, dependency manifest, security hoặc migration cũ.
- Database Store: chỉ thêm bảng/job/đối tượng mới khi cần. Không sửa hoặc xóa bảng, cột, index hay đối tượng hiện hữu. Không chạy script reset/recreate DB dùng chung.
- Có thể trùng lặp code trong package mới để giữ độc lập với nhóm Store.
- Ưu tiên triển khai app trước; việc làm API thuộc cùng mục tiêu và được thực hiện khi tới mốc tích hợp.
- Copy deployment; AIPro global ở pilot, project ở phase 2. Không ghi đè nội dung chưa quản lý hoặc local edits trong pilot.
- Không tự bật public updater/Git/CLI; không chạy script từ package; không ghi token vào frontend/SQLite/log.
- HTTP chỉ cho loopback trong cấu hình development. Môi trường nội bộ dùng HTTPS có xác minh chứng chỉ; không cho tắt TLS verification.
- Fixture chỉ trong debug/test, bật chủ động, tách data/target roots, không fallback tự động khi HTTP lỗi.
- Snapshot/restore, cập nhật chọn lọc, project, offline đầy đủ, ba agent khác và preset vẫn thuộc phase 2–5; hoàn thành plan này không hoàn thành toàn bộ goal 1.0.
- Không sửa `.gitignore` và `logo_samsung.svg` đang có thay đổi của người dùng. Git chỉ stage file của task. Store chưa có Git: giữ manifest và change inventory, không tự `git init`.

## Review Focus

1. Profile mismatch: frontend SDS chạy nhầm backend upstream hoặc ngược lại phải dừng với lỗi rõ; không âm thầm mở Library cá nhân. Test A01/A02.
2. Race account/profile: response của account cũ hoặc refresh tới sau logout không được phục hồi phiên/cache cũ. Test A03/A04.
3. Windows path aliases: junction, drive/UNC, ADS, trailing dot/space và case collision phải bị từ chối trước ghi file. Test A05/A06.
4. Crash ở ranh giới filesystem/DB: mất điện sau rename nhưng trước DB commit phải phục hồi đúng target, không dọn evidence cần thiết. Test A07/A08.
5. Gói/target đổi sau review: khác digest, fingerprint hoặc parent junction khiến plan hết hiệu lực; yêu cầu review lại. Test A06/A07/A09.

## Phạm vi, baseline và trình tự

App root: `D:/Documents/Git Project/skills-manager-app/skills-manager`. Đường dẫn trong plan tương đối với root này. `R = src-tauri/src/sds`, `F = src/features/sds`; ký hiệu R/F bên dưới chỉ là tiền tố đường dẫn, không phải file cần tạo thêm.

Baseline code `e6dcedf`, spec commit `161f6d4`. Đã kiểm tra: node 24.20.0, npm 11.4.1, Rust/Cargo 1.99.0, Java 21.0.7 có trên máy; chưa có node_modules hoặc src-tauri/target. Chưa chạy build/test. Store có mvnw.cmd nhưng thiếu `.mvn/wrapper/maven-wrapper.properties`, Maven chưa thấy trên PATH. Không suy ra toolchain build hoàn chỉnh chỉ từ các binary này.

Thực hiện tuần tự A00 → A01 → A02 → A03 → A04 → A05 → A06 → A07 → A08 → A09 → Store S01–S04 → A10. Có thể review từng task độc lập nhưng không mở worker song song sửa cùng runtime/contracts. Method do người dùng chọn ở cuối plan.

Phase 0 kết thúc sau A02 với shell/bộ cài nền. Phase 1 kết thúc sau A10. Các bản app dùng fixtures giữa hai mốc là development builds, không phải pilot đã nghiệm thu.

## File structure và contracts chung

- `R/mod.rs`, `profile.rs`, `runtime.rs`, `error.rs`: profile, startup/exit và public IPC allowlist.
- `R/contracts.rs`, `R/fixtures.rs`: DTO wire/domain và fake HTTP fixtures chỉ debug/test.
- `R/session.rs`, `credentials.rs`, `store_client.rs`: auth, OS credential store, network.
- `R/repository.rs`, `migrations/001_pilot.sql`: SQLite riêng SDS; dữ liệu release/deployment/journal/cache.
- `R/targets.rs`, `mcp.rs`: AIPro target và cấu hình MCP chỉ đọc.
- `R/packages/{mod,paths,zip,seven_zip}.rs`: chuẩn bị artifact; hai format dùng chung validator/limits.
- `R/deployment.rs`, `journal.rs`, `operations.rs`, `commands.rs`: thao tác có review/lock/recovery và Tauri commands.
- `R/tests/{mod,support,profile,session,catalog,targets,mcp,packages,deployment,recovery,operations}.rs`: unit/integration qua các seam trên.
- `F/{SdsApp,SdsShell,SdsThemeProvider}.tsx`, `api.ts`, `contracts.ts`, `sds.css`: app profile UI và boundary; `F/session/`, `catalog/`, `install/`, `library/`, `settings/` chứa components theo luồng.
- `contracts/sds-desktop/v1/{README.md,types.json,fixtures/}`: contract versioned cho cả consumer/provider; không đặt fixtures dùng build trong docs vì docs hiện bị ignore.
- `src-tauri/tauri.sds.conf.json`, `tauri.sds.dev.conf.json`, `capabilities/sds.json`, `scripts/sds.mjs`: launch/build SDS đồng bộ feature/profile.

Kiểu Rust serde camelCase và TS tương ứng (không export secret):

```text
SdsError { code:String, message:String, retryable:bool, operationId:Option<String> }
ProfileInfo { kind:upstream|sds, environment:development|internal, fixtureMode:bool }
StoreProfile { id:String, baseUrl:String, environment:development|internal }
SessionView { status:signedOut|authenticated|expired|offline, user:Option<UserView> }
UserView { id:String, displayName:String }
CatalogQuery { q:String, mode:all_time|trending|hot, sourceId:Option<String>, page:u32, pageSize:u32 }
Page<T> { items:Vec<T>, page:u32, pageSize:u32, totalItems:u64 }
Taxonomy { sources:Vec<Source{id,label}>, catalogModes:Vec<Mode{id,label,available}> }
SkillSummary { id,name,description:String, metadataRevision:u64, roles,tags,categoryIds,sourceIds:Vec<String>, creator:UserView, latestRelease:Option<ReleaseRef>, compatibleAgents:Option<Vec<String>>, canDownload:bool, updatedAt:String }
ReleaseRef { id:String, sequence:u64, version:String }
Release { skillId,id:String, sequence:u64, version,installName,publishedAt,changelog:String, compatibleAgents,requiredMcps:Option<Vec<String>>, package:PackageInfo }
PackageInfo { format:zip|7z, fileName:String, sizeBytes:u64, sha256:String, downloadPath:String }
SkillDetail { summary:SkillSummary, storePageUrl:Option<String>, latestRelease:Option<Release> }
Target { agentKey:String, workspaceId:String, path:PathBuf }
McpReport { state:notRequired|unknown|configured|missing|unverifiable, missing:Vec<String>, reason:Option<String>, checkedAt:String }
Manifest = sorted Vec<{path:String,size:u64,sha256:String}>
InstallRequest { skillId:String, releaseId:String } // pilot backend resolves AIPro global; UI never submits destination
PreparedOperation { id:String, kind:install|remove, release:Release, target:Target, expectedFingerprint:Option<String>, mcp:McpReport, requiresMcpAck:bool, canExecute:bool, conflict:Option<String> }
OperationResult { id:String, state:prepared|running|completed|failed|cancelled|recoveryRequired, stage:String, target:Target, error:Option<SdsError>, mcp:McpReport }
DeploymentView { id:String, release:Release, target:Target, baselineFingerprint:String, actualState:installed|modified|missing, mcp:McpReport }
LibraryEntry { release:Release, deployments:Vec<DeploymentView> }
```

Wire `SkillDetail` vẫn theo API-DRAFT (summary fields ở top level); gateway chuyển thành domain `summary`. Request success envelope `{success:true,data:T}`; error envelope `{success:false,errorCode,message}`; requestId optional. Token DTO nội bộ không derives Debug/Serialize cho IPC.

Các trait/boundary được task định nghĩa: `CredentialVault::{read,write,clear}(profileId)`; `HttpTransport::send(HttpRequest)->Result<HttpResponse,SdsError>`; `StoreClient::{taxonomy,search,detail,release,download}`; `Repository`; `OperationService`. Signature async tại session/network/operation, đồng bộ cho pure parse/path/hash/SQLite. Tauri gọi blocking filesystem qua spawn_blocking; không giữ MutexGuard qua await.

## A00 — Baseline có thể tái tạo và quyền sở hữu thay đổi

**Files:** tạo `docs/superpowers/evidence/phase0-baseline.md`; không sửa source để che lỗi baseline.
**Interfaces:** đầu vào repo HEAD + worktree; đầu ra báo cáo toolchain, command/exit, lỗi sẵn có, thư mục thực thi.

- [ ] Chạy skill using-git-worktrees tại execution time; tái sử dụng worktree phù hợp hoặc tạo worktree qua native tool. Đảm bảo spec/plan có trong checkout; không mang thay đổi người dùng vào commit task.
- [ ] Ghi HEAD, status, Node/Rust/JDK và Windows build prerequisites. Cài app deps bằng `npm ci`; không cài dependencies vào Store lúc này.
- [ ] Chạy `npm run lint`, `npm run build`, `cargo test --manifest-path src-tauri/Cargo.toml`; ghi riêng exit/status và thời gian. Poll đúng handle nếu build lâu, không khởi động build trùng.
- [ ] Với baseline fail, xác định lỗi toolchain hay source và sửa phạm vi cần thiết thành task riêng có test; không ghi PASS hoặc tiếp tục đóng gói khi thiếu điều kiện build.
- [ ] Commit evidence file bằng stage tường minh (`git add -f -- docs/superpowers/evidence/phase0-baseline.md`); message `docs: record desktop build baseline`.

## A01 — Runtime SDS không chạm startup và dữ liệu upstream

**Files:** tạo `R/{mod,profile,runtime,error}.rs`, `R/tests/{mod,profile}.rs`; sửa `src-tauri/src/lib.rs`, `src-tauri/Cargo.toml`; tạo các config/capabilities SDS nêu trên, `scripts/sds.mjs`; sửa `package.json` và lockfile khi script cần.
**Interfaces:** `SdsProfile::resolve()->Result<SdsProfile,SdsError>`, `profile.info()->ProfileInfo`, `profile.data_root()->&Path`, `runtime::run()`; command `runtime_profile()->ProfileInfo` có ở cả runtime. Data root `%LOCALAPPDATA%/SDS/SkillsManager`; test override qua constructor, debug override phải chủ động.

- [ ] Viết tests `sds::tests::profile`: `assert_eq!(profile.info().kind,"sds")`; spy startup assert upstream initializer/Git/updater/CLI calls bằng 0; SDS command registry không có `install_from_git`, `install_from_skillssh`; root SDS không bằng upstream root. Release build từ chối fixture flag.
- [ ] Chạy `cargo test --manifest-path src-tauri/Cargo.toml --features sds sds::tests::profile`; expected fail trước implementation (feature/module chưa tồn tại).
- [ ] Thêm Cargo feature `sds`; dispatch vào runtime SDS trước `initialize_store()` ở lib.rs. Giữ runtime cũ cho default build, nhưng runtime SDS chỉ đăng ký commands cần dùng, không đăng ký updater/shell/Git/CLI. SDS exit không gọi auto_backup upstream. Giữ single instance/dialog/opener/log/window restore với dữ liệu riêng; capability config SDS chỉ chọn `sds.json`, không kế thừa updater permission từ default.
- [ ] Launcher `npm run sds:dev`, `sds:build`, `sds:check` dùng child process args, đặt VITE_APP_PROFILE=sds và Cargo feature sds đồng bộ; config SDS có identifier riêng `com.sds.skillsmanager.desktop`, productName `SDS Skills Manager`, cửa sổ 1440×860/min1100×640. Bỏ public updater endpoints/artifacts trong effective merged config. `cargo check --manifest-path src-tauri/Cargo.toml --features sds` và profile tests pass; kiểm tra effective config không còn public endpoints.
- [ ] Commit chỉ files task: `feat: isolate SDS desktop runtime`.

## A02 — Shell SDS có thể mở và đóng gói sớm

**Files:** tạo `F/{SdsApp,SdsShell,SdsThemeProvider}.tsx`, `F/{api,contracts}.ts`, `F/sds.css`, `F/SdsApp.test.tsx`, `vitest.config.ts`, `src/test/setup.ts`; sửa `src/main.tsx`, `package.json`, `package-lock.json`. Không gắn AppProvider/upstream ThemeProvider vào SDS vì chúng gọi commands cũ.
**Interfaces:** `getRuntimeProfile():Promise<ProfileInfo>`; `SdsApp` chỉ render sau handshake; `SdsThemeProvider` lưu preference dưới key `sds.theme`, default light, English shell. Các components cũ chỉ reuse nếu không có side effects upstream.

- [ ] Thêm Vitest/Testing Library/jsdom dev dependencies tương thích Vite/Node đã kiểm tra ở execution time; script `test:sds = vitest run src/features/sds`. Test `rejects_runtime_profile_mismatch`, `does_not_mount_upstream_context`, `renders_keyboard_accessible_shell`; assertions UI mismatch alert, không có call getSettings/updater/auto sync, tab focus tới navigation.
- [ ] Chạy `npm run test:sds -- SdsApp.test.tsx`, expected fail do SdsApp chưa có/thiếu handshake.
- [ ] Tạo shell theo layout Stitch mapped IDs, tái sử dụng CSS tokens/leaf controls; main.tsx chọn entry theo build profile và không gọi logStartupEvent/i18n bootstrap upstream trong SDS. Chỉ bật Store/Library/Settings khi có nội dung thật ở task sau, còn màn nền có thông báo setup. CSP fonts/images local; không CDN. Nếu bổ sung token compact thì có exception có lý do trong UI spec checker, không tắt lint toàn cục.
- [ ] Chạy frontend tests/build/lint, `npm run sds:build -- --bundles nsis`; mở bộ cài trong môi trường thử, xác minh app khởi động/đóng/mở lại, dữ liệu upstream không đổi. Đây là shell build, chưa gọi là pilot.
- [ ] Commit `feat: add SDS desktop shell and build entry`; ghi path/hash bộ cài nền trong evidence A00.

## A03 — Session Store và Settings kết nối thật

**Files:** tạo `R/{contracts,session,credentials,store_client}.rs`, `R/tests/{support,session}.rs`, `F/session/{SignIn,SessionProvider}.tsx`, `F/settings/ConnectionSettings.tsx`, tests `F/session/SignIn.test.tsx`, `contracts/sds-desktop/v1/{README.md,types.json,fixtures/auth-login.json,fixtures/auth-refresh.json}`; sửa `R/{runtime,commands}.rs` (commands.rs tạo ở đây), `F/{api,contracts,SdsApp}.tsx/ts` theo loại file thực tế.
**Interfaces:** `SessionService::{login(email,password)->SessionView,status()->SessionView,logout()->(),request(HttpRequest)->HttpResponse}` async; constructor nhận HttpTransport, CredentialVault, StoreProfile. IPC `sds_session_status`, `sds_login`, `sds_logout`, `sds_get_connection`, `sds_set_connection`; credentials không return qua IPC. `join_api(baseUrl,relativePath)->Result<Url,SdsError>` ở store_client.rs.

- [ ] Test parse payload LoginResponse thật: userId 1001→"1001", userName→displayName, không yêu cầu expiresIn. `assert_eq!(join_api("http://127.0.0.1:8080/skm","api/v1/auth/login").path(),"/skm/api/v1/auth/login")`. Test reject remote HTTP, URL credentials/query/fragment, foreign-origin redirect/downloadPath. Test 401 refresh một lần; 403 không refresh; hai request đồng thời chỉ một refresh; logout/profile switch trong refresh không phục hồi session; keyring fail không lưu plaintext; UI password bị clear sau submit.
- [ ] Chạy Rust filter `sds::tests::session` và `npm run test:sds -- SignIn.test.tsx`; expected failures ở behavior chưa làm.
- [ ] Implement bằng reqwest (đã có), redirects off, keyring (đã có), private auth DTO. Mutex cho refresh + generation counter chống stale completion. Session offline khi mạng lỗi nhưng identity retained; expired khi 401 sau refresh. Logout luôn clear memory/keyring/cache scope dù revoke lỗi. Thay Store URL buộc logout trước, profileId gắn canonical base URL.
- [ ] Run focused tests + `npm run sds:check`; logs/error serialization assertions không chứa password/token. Tạo local fake HTTP server trong Rust test, chỉ loopback và synthetic credentials; không gửi credential thật trong fixture mode.
- [ ] Commit `feat: connect SDS account session and Store settings`.

## A04 — Catalog/detail xuyên UI–gateway

**Files:** tạo `R/fixtures.rs`, `R/tests/catalog.rs`, `F/catalog/{CatalogPage,SkillDetailPanel}.tsx`, `F/catalog/useCatalog.ts`, `F/catalog/CatalogPage.test.tsx`; fixtures `catalog.json`, `detail.json`, `taxonomy.json`, `release.json`; sửa `R/{store_client,contracts,commands}.rs`, `F/{api,contracts,SdsApp}`.
**Interfaces:** StoreClient async `taxonomy()->Taxonomy`, `search(CatalogQuery)->Page<SkillSummary>`, `detail(skillId)->SkillDetail`, `release(skillId,releaseId)->Release`; IPC `sds_catalog_taxonomy`, `sds_catalog_search`, `sds_skill_detail`. Fixtures serve cùng HTTP envelope, không có alternate silent UI path.

- [ ] Tests q+mode+source AND query, page1/pageSize20 mặc định/max100; đổi filter reset page; response chậm account/query cũ bị bỏ; quay lại giữ query; 422 hiển thị mode unavailable; 401 khác empty/503; missing latestRelease/compatibility không tạo Installed/verified giả. Markdown không tải remote images hay javascript links.
- [ ] Run `cargo test --manifest-path src-tauri/Cargo.toml --features sds sds::tests::catalog` và `npm run test:sds -- CatalogPage.test.tsx`; expected fail trên mapping/state mới.
- [ ] Implement search debounce 300ms + Enter, Sources từ taxonomy, mode availability từ server, detail có MCP/release/size/changelog. Local Install hiện disabled "Available in a later release" trong pilot. Chỉ detail đã có release mới cho chuẩn bị cài. Cache tách account/profile/query; không sort một trang để giả server ranking.
- [ ] Chạy focused tests, build/lint; xem W02/W03 ở cả loading/empty/error với fixture banner rõ. Log một network trace synthetic chứng minh contract URL/payload.
- [ ] Commit `feat: browse internal Store catalog and releases`.

## A05 — AIPro target và MCP review

**Files:** tạo `R/{targets,mcp}.rs`, `R/tests/{targets,mcp}.rs`, `F/install/InstallReview.tsx`, `F/install/InstallReview.test.tsx`; sửa contracts/commands/api.
**Interfaces:** `resolve_aipro_global(home:&Path,override_root:Option<&Path>)->Result<Target,SdsError>`; `inspect_mcp(path:&Path,required:Option<&[String]>)->McpReport`; target backend resolves home/override, UI không gửi path cho execute.

- [ ] Test path home/.aipro/skills; missing AIPro root→notReady (override đã xác nhận có thể dùng); reject reparse roots/ancestors; `None` requirements→unknown, `[]`→notRequired, thiếu file/sai JSON/sai shape→unverifiable/missing; key case-sensitive; `{}` value không configured; non-AIPro future target không có green MCP badge; report không chứa token/URL config.
- [ ] Run filters `sds::tests::targets`, `sds::tests::mcp` và `npm run test:sds -- InstallReview.test.tsx`; expected fail trước implementation.
- [ ] Implement inspection local-only; UI path rõ và buttons Check again/Install anyway/Cancel; guide URL phải cùng Store origin/configured internal host, chỉ mở theo click. Không sửa mcp.json. Override chỉ trong Settings và backend validates folder, không tự cài AIPro.
- [ ] Focused tests pass; thêm test sửa MCP giữa hai lần Check again thay state ngay. Verify keyboard/dialog controls không bị che ở min window.
- [ ] Commit `feat: review AIPro target and MCP requirements`.

## A06 — Artifact ZIP/7z được xác minh trước cài

**Files:** tạo `R/packages/{mod,paths,zip,seven_zip}.rs`, `R/tests/packages.rs`, synthetic archives trong `contracts/sds-desktop/v1/fixtures/packages/`; sửa `src-tauri/Cargo.toml`/lockfile để thêm reader 7z khi đã kiểm tra dependency; sửa `R/store_client.rs`.
**Interfaces:** `PackageLimits { archiveBytes:104857600, expandedBytes:524288000, entries:10000, depth:32 }`; `prepare_package(archive:&Path,release:&Release,staging:&Path,limits:&PackageLimits)->Result<VerifiedArtifact,SdsError>`; `VerifiedArtifact { root:PathBuf, manifest:Manifest, fingerprint:String }`; `StoreClient::download(release:&Release,dest:&Path,cancel:&AtomicBool)->Result<(),SdsError>` async, same-origin bounded stream.

- [ ] Tests ZIP và 7z tương đương giữ bytes resources; root trực tiếp/one wrapper accepted; thiếu/nhiều SKILL.md, checksum/size sai, nested-only, encrypted/split7z, zip slip/backslash, ADS/drive/UNC, reserved names, trailing dot/space, duplicate/case collision, symlink/junction rejected. `before_target == after_target` cho mọi failure. Bounds enforced trên actual decompressed bytes kể cả header khai sai; cancel không tạo target/cache verified.
- [ ] Run `cargo test --manifest-path src-tauri/Cargo.toml --features sds sds::tests::packages`; expected fail missing preparation API.
- [ ] Reuse zip crate cho ZIP; chọn reader Rust 7z có entry metadata + streamed callback và kiểm tra official API/MSRV tại execution time, pin lockfile. Không gọi executable hệ thống hoặc giải nén unchecked. Reject entry trước write, chuẩn hóa collision key riêng, không sanitize tên thành một tên khác. Không reuse `install_skill_dir_to_destination` vì nó xóa destination trước copy.
- [ ] Focused tests pass; verify actual digest fixture bằng SHA-256 và manifest includes added/deleted files, binary và Unicode paths. Limits là defaults sản phẩm đề xuất trong plan, ghi rõ lỗi giới hạn cho người dùng.
- [ ] Commit `feat: validate ZIP and 7z release artifacts`.

## A07 — Cài có journal và recovery

**Files:** tạo `R/{repository,deployment,journal,operations}.rs`, `R/migrations/001_pilot.sql`, `R/tests/{deployment,recovery}.rs`; sửa runtime startup để recover trước accept mutation.
**Interfaces:** `Repository::open(path)->Result<Self,SdsError>`, `cache_release(&Release,&VerifiedArtifact)`, `list_library()->Vec<LibraryEntry>`, `load_operation(id)->PreparedOperation`, `record_result(&OperationResult)`; `OperationService::prepare_install(InstallRequest)->PreparedOperation`, `execute(id:String,ackMcp:bool)->OperationResult`, `cancel(id:String)`, `status(id:String)->OperationResult`; `recover_pending(repo,roots)->Vec<OperationResult>`. Service giữ dependencies/client/repo/roots và không nhận path từ frontend.

- [ ] Tests fresh install, exact release idempotent, unmanaged collision/local edits refused; race fingerprint/parent changed→REVIEW_REQUIRED; ack gắn operation ID, không dùng lại cho release mới. Fault injection trước/sau stage rename và DB commit: restart trả completed khi hashes/DB khớp hoặc rollback/quarantine có lỗi rõ; không xóa file người dùng mới xuất hiện. Cancellation trước commit không đổi target; during commit hoàn tất/rollback rồi mới cancelled. Lock hai process cùng canonical target phải serialize/refuse.
- [ ] Run filters `sds::tests::deployment` và `sds::tests::recovery`; expected failure trên journal/recovery chưa có.
- [ ] Implement SQLite tables releases/deployments/operations/catalog_cache dưới SDS root. Identity release scoped profile+skill+release; cache path theo hashed IDs, không nối raw remote IDs. Cache bất biến; target copy trên cùng volume. fs2 lock theo canonical path, durable intent trước rename; fingerprint recheck trong lock. SQLite failure không dọn stage/rollback cần recovery. Recovery validates mọi path trong journal trước file operation.
- [ ] Focused tests pass gồm OS process termination test ở commit boundary trong temp root; verify không ghi `.aipro`/Library thực. Cache staging bị lỗi không mang state verified; target fingerprint hash content và danh sách path.
- [ ] Commit `feat: deploy releases with recoverable operations`.

## A08 — Library và gỡ theo target

**Files:** tạo `F/library/{LibraryPage,RemoveReview}.tsx`, `F/library/LibraryPage.test.tsx`, thêm `R/tests/operations.rs`; sửa `R/{operations,repository,deployment,commands}.rs` và api/contracts.
**Interfaces:** `OperationService::prepare_remove(deploymentId:String)->PreparedOperation`; execute/cancel/status dùng A07; IPC `sds_library`, `sds_prepare_remove`; Library không xóa release cache như side effect.

- [ ] Test Library sau restart vẫn có release/path/MCP; file thêm/sửa/xóa→modified, mất thư mục→missing. Remove untouched target không xóa Library, target khác, `.aipro/skills` root hoặc mcp.json; local edits→conflict/canExecute=false trong pilot. Fault sau rename-to-quarantine trước DB remove phải recover; tampered DB path không được dùng để recursive delete.
- [ ] Run `cargo test --manifest-path src-tauri/Cargo.toml --features sds sds::tests::operations` và `npm run test:sds -- LibraryPage.test.tsx`; expected fail mới.
- [ ] Implement scoped remove bằng lock/fingerprint/journal/quarantine, DB update rồi cleanup. UI có review path rõ, local edits hướng dẫn giữ bản hiện tại; không bật force delete khi snapshot chưa có. Basic Library search, path/release/status; preview chỉ đọc file nằm trong release root.
- [ ] Run tests; disconnect fake Store vẫn xem Library được, protected downloads báo lỗi phù hợp; không nhận offline full acceptance phase 2 từ test này.
- [ ] Commit `feat: manage pilot Library and scoped removal`.

## A09 — Nối toàn luồng Tauri và kết quả cài

**Files:** tạo `F/install/{OperationProgress,OperationResult}.tsx`, `F/install/InstallFlow.test.tsx`; sửa `R/{commands,runtime,operations}.rs`, `F/{api,contracts,SdsApp}`, catalog/library review components; tạo `scripts/test-sds-contracts.mjs`.
**Interfaces:** IPC `sds_prepare_install`, `sds_execute`, `sds_cancel`, `sds_operation_status`; execute schedules background job trả id ngay, progress event `sds:operation` chứa OperationResult; status là authoritative nếu event missed. Không lấy completed từ chỉ 100% progress.

- [ ] Test Store/Library entry points cùng prepare→MCP→execute; disabled double submit; stale plan rejected; cancel + restart; failed download không biến thành empty Library hoặc success; 401 giữa operation không ghi target; progress subscription teardown không hủy background job ngoài ý định.
- [ ] Run `npm run test:sds -- InstallFlow.test.tsx`, Rust operations filter; expected fail ở orchestration chưa nối.
- [ ] Wire UI states với real IPC; thêm local activity log operation/skill/release/path/stage/result đã lọc secrets; opening logs/paths chỉ theo user click. Contract verifier đọc checked-in fixtures và so shapes với Rust/TS; unknown fields compatible, missing required metadata produces explicit error.
- [ ] Run `node scripts/test-sds-contracts.mjs`, frontend focused tests + build/lint; chạy app debug fixture end-to-end trên sandbox target. Xem W01–W07/W12/W13 thật qua UI; kiểm tra focus, resize, copy/reveal path, errors. Ghi evidence là fixture execution.
- [ ] Commit `feat: complete pilot installation flow and feedback`.

## Store integration checkpoint

- [ ] Thực hiện S01–S04 trong companion plan sau A09. Đầu ra cần: Store local running handle, real account, published immutable ZIP/7z release có digest thật, endpoints theo contract và audit chỉ thêm mới.
- [ ] Không chuyển sang nghiệm thu A10 nếu chỉ có fake HTTP server. Không đánh dấu mục tiêu 1.0 complete khi pilot pass.

## A10 — Windows pilot thật và bàn giao

**Files:** tạo `docs/superpowers/evidence/phase1-pilot.md`, `docs/sds-pilot/{RUNBOOK,KNOWN-LIMITATIONS}.md`; hoàn thiện `src-tauri/tauri.sds.conf.json`, scripts sds nếu packaging cần; không chứa account/password/token trong evidence.
**Interfaces:** người dùng cài bộ cài Windows, Store localhost:8080/skm thực; HTTP debug local integration được ghi rõ. Bản internal deployment dùng HTTPS profile, không hardcode localhost trong release.

- [ ] Chạy `npm run lint`, `npm run build`, `npm run test:sds`, `npm run sds:check`, `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo test --manifest-path src-tauri/Cargo.toml --features sds`, `node scripts/test-sds-contracts.mjs`. Mỗi suite phải exit0 hoặc có baseline failure được phân tích cụ thể; failure liên quan SDS chặn bàn giao.
- [ ] Build `npm run sds:build -- --bundles nsis`; launcher bảo đảm default-run chỉ bundle desktop binary, không CLI. Chọn WebView2 provisioning phù hợp máy pilot (offline installer nếu thiếu runtime); lưu SHA-256/path và hướng dẫn cài. Không giả đã ký khi chưa có certificate IT.
- [ ] Cài trên Windows thử nghiệm: login thật → search/detail → MCP missing/recheck/install anyway → ZIP và 7z → Library → gỡ; thử lỗi package, permission và cancel/restart. Thực hiện qua app IPC/UI, không dùng unit test thay nghiệm thu. Dùng skill thử tên riêng, không đụng skill hiện hữu.
- [ ] Xác minh AIPro thực nhận skill, không chỉ file tồn tại. Nếu thiếu AIPro/runtime/account thì ghi requirement chưa đạt, giữ goal active; tiếp tục phần độc lập. Kiểm tra không có public network/CLI/Git runtime tác dụng phụ bằng runtime evidence, không chỉ search source. Run Store file/schema additive audit lần cuối.
- [ ] Dùng Matt Pocock code-review cho changes, sửa actionable findings và chạy lại checks liên quan. Ghi acceptance theo ID A01–A10/S01–S04, hạn chế pilot và lộ trình phase 2–5; commit `docs: record verified SDS pilot delivery` chỉ khi evidence đúng. Không publish/deploy company-wide tự động.

## Self-review coverage và handoff

- Spec §1–4: constraints + phase boundaries A00/A10; roadmap đầy đủ vẫn trong approved spec.
- Spec §5–7: profile A01/A02, session/catalog A03/A04, paths/MCP A05, package A06, operation/data/recovery A07, remove A08, UI/log A09.
- Spec §8–9: Store companion S01–S04 + A03/A04; server cần các endpoint phase 2–4 sẽ có plan riêng trước phase đó, không giả đã triển khai.
- Spec §10–11: UI mapping/state A02/A04/A05/A09, review focus có tests A01–A09, native packaging/real integration A10.
- Đề xuất thực thi **Native**, làm tuần tự từng task trong session, test ở seam và code review cuối; giảm overhead chuyển context khi các task cùng chia sẻ DTO/journal/runtime. Subagent-driven là lựa chọn khác với reviewer riêng mỗi task, chi phí context cao hơn.
- Sau người dùng review plan và chọn method mới bắt đầu A00; không tự coi phê duyệt spec là phê duyệt plan. Không có task nào ở đây đã được thực hiện tại lúc viết kế hoạch.
