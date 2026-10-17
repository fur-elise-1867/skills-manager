# SDS Store Additive Pilot Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans. Thực hiện S01–S04 sau app A09, trước A10.

**Goal:** Cung cấp API Store local thật cho pilot AIPro, giữ nguyên bytes mọi file Store hiện hữu và schema mọi đối tượng database hiện hữu.

**Architecture:** Thêm package `com.furelise.skillmanagement.desktop` chứa controller/DTO/service/repository/model riêng. Đọc skill/ACL qua source hiện tại; release/package/source là đối tượng desktop mới. Dùng dependencies có sẵn, authenticated filter chain hiện tại và PostgreSQL local dành riêng cho thử nghiệm.

**Tech Stack:** Java21, Spring Boot3.4.2, Spring Security/JPA, Flyway, PostgreSQL, JUnit/MockMvc; H2 chỉ cho tests tương thích.

**Spec:** [Delivery design](../specs/2026-10-16-sds-app-delivery-design.md), [app plan](2026-10-16-sds-app-phase0-1.md), [API draft](../../sds-phase1/API-DRAFT.md). Review amendments của app plan quyết định auth/URL khi draft khác source.

## Global Constraints

- Store root `D:/Documents/Git Project/skills-manager-app/skills-store/skill-management`; B=`backend/src/main/java/com/furelise/skillmanagement/desktop`.
- Chỉ thêm file/folder/package mới. Không sửa pom.xml, SecurityConfig, application.yml, scripts/migrations cũ hoặc file hiện hữu khác. Không git init Store.
- Chỉ thêm database objects mới thuộc desktop; không FK/index/trigger lên bảng cũ. Không reset/recreate hay bootstrap database dùng chung.
- Kiểm tra JDBC/database identity trước mutation; localhost không chứng minh là database thử. Secrets qua env, không docs/log/frontend.
- Account/skill synthetic chỉ trên DB thử riêng. Publish desktop release không ghi Skill.version hoặc bảng nghiệp vụ upstream.
- Package/release immutable; checksum thực; catalog/detail/release/package kiểm tra ACL server-side. Không dùng download-all làm install artifact.
- Envelope/DTO theo `contracts/sds-desktop/v1` của app; auth giữ endpoint và DTO hiện tại.

## Review Focus

1. ACL trước count/pagination, parity với helper cũ; S02 tests.
2. Release/package không lộ dữ liệu người mất quyền; S03 tests.
3. Release ID không đổi bytes sau publish, kể cả retry/concurrent publish; S03 tests.
4. Chạy nhầm DB phải dừng trước mutation; S01/S04 identity guard.
5. File/schema cũ đổi hoặc mất phải làm audit fail; S04 tests.

## S01 — Baseline và môi trường local riêng

**Files:** thêm `backend/desktop-pilot/{inventory.ps1,start.ps1,README.md}`, `backend/src/main/resources/application-desktop-pilot.yml`; evidence ở app `docs/superpowers/evidence/store-baseline.md`.

**Interfaces:** `inventory.ps1 -Root <absolute> -Output <manifest>` xuất sorted relative path/size/SHA256; `start.ps1 -DatabaseUrl <jdbc>` chạy loopback sau DB identity/port checks.

- [ ] Chụp manifest mọi file Store hiện hữu trước thêm file, lưu ngoài Store source; ghi root/thời gian và schema baseline.
- [ ] Kiểm Java/Maven/wrapper, process/8080 và PostgreSQL. Wrapper thiếu thì dùng Maven runtime riêng ngoài source, không sửa wrapper/pom cũ.
- [ ] Compile/test baseline, ghi command/exit/time; không che lỗi bằng disable suite hoặc ddl-auto update/create.
- [ ] Tạo/chọn DB thử tên `sds_desktop_pilot_*`, kiểm identity và marker dành riêng; script từ chối DB mặc định `skill_management_db`, remote DB hoặc marker thiếu. Bootstrap upstream chỉ trên DB mới trống này, giữ ddl-auto validate.
- [ ] Thêm profile desktop pilot dùng env datasource/storage, bind loopback, secrets qua process env; không copy application-local.yml của nhóm.
- [ ] Test inventory phát hiện file synthetic sửa/mất; test start guard từ chối DB không đúng tên/marker. Bàn giao process handle/URL, inventory và baseline.

## S02 — Catalog/detail/taxonomy với ACL

**Files:** thêm `B/controller/DesktopCatalogController.java`, `B/dto/DesktopDtos.java`, `B/service/DesktopCatalogService.java`, `B/repository/DesktopCatalogRepository.java`; tests mới tại `backend/src/test/java/com/furelise/skillmanagement/desktop/` và fixtures resources riêng.

**Interfaces:** GET `/api/v1/desktop/{me,taxonomy,skills}` và `/skills/{skillId}`. Principal User từ auth hiện có; query `{q,mode,sourceId,page,pageSize}`; DTO theo app contracts, IDs string/page1/default20/max100.

- [ ] Viết tests 401, q+source AND, ACL trước count/page, visibility parity với SkillAccessHelper, skill chưa release→latestRelease null, 400 invalid filter, 422 unavailable mode. Chạy thấy RED do facade thiếu.
- [ ] Implement read-only upstream access trong package mới. Sources thuộc desktop mapping, không tự đồng nhất category. Trending/Hot unavailable theo contract.
- [ ] All Time: người dùng đã chấp thuận ngày2026-10-17 thứ tự tên A–Z và skillId khi trùng tên, áp dụng tạm cho pilot. Triển khai name case-insensitive ascending + skillId ascending trước pagination; ghi rõ đây không phải leaderboard. Không suy ra ranking từ updatedAt/download count. Trending/Hot vẫn unavailable cho đến khi có công thức được chốt.
- [ ] Normalize userId/userName ở gateway app; không sửa LoginResponse để thêm expiresIn/user object.
- [ ] Run focused tests GREEN; consumer/provider fixture parity và server HTTP thật giữ context `/skm`. Ghi limitations/ranking decision trong evidence.

## S03 — Immutable release và authenticated package stream

**Files:** thêm `B/model/{DesktopRelease,DesktopPackage,DesktopSkillSource}.java`, `B/repository/DesktopReleaseRepository.java`, `B/service/DesktopReleaseService.java`, `B/controller/DesktopReleaseController.java`; migration mới trong `backend/src/main/resources/db/desktop-migration/`, `backend/desktop-pilot/publish.ps1` và tests/fixtures mới.

**Interfaces:** GET `/api/v1/desktop/skills/{skillId}/releases/{releaseId}` và `/package`. Release/Package DTO theo app contracts. Publish là tool local có guard, không anonymous API; metadata không chứa filesystem path.

- [ ] Tests digest/size đúng bytes, ZIP/7z không wrapper ZIP thêm, ID/digest immutable, sequence unique per skill, withdrawn→410, quyền download parity, traversal, missing file không trả HTML/success giả. Run RED.
- [ ] Migration chỉ tạo desktop tables/constraints và history riêng; config class/bean tên riêng nếu cần, không override bean cũ. Chạy sau upstream bootstrap trên DB thử đã xác minh.
- [ ] Implement same-origin exact stream content type/length/disposition; `/api/v1/...` downloadPath theo context rule app. Mọi request qua visibility/download ACL thực tế; không coi canDownload từ UI là permission.
- [ ] Publish ZIP và 7z synthetic trên DB thử, MCP metadata required/empty/unknown; storage root riêng theo opaque ID, không raw input path từ client.
- [ ] Run tests GREEN và HTTP integration thật; lưu IDs/digests, không secret. Endpoint phase sau chưa có trả unavailable có chủ đích.

## S04 — Contract integration và additive audit

**Files:** thêm `backend/desktop-pilot/audit.ps1`; evidence app `docs/superpowers/evidence/store-pilot.md`.

**Interfaces:** audit nhận before/after file manifest và schema snapshots; exit nonzero khi file/object cũ đổi/mất. Object thêm phải thuộc desktop allowlist; accounts/skill seed chỉ trong DB thử.

- [ ] Test audit với synthetic changed/deleted/new files/objects; chỉ additions pass. Schema scope gồm tables/columns/constraints/indexes/functions/triggers.
- [ ] Provider suite và route upstream tests; app `node scripts/test-sds-contracts.mjs`; kiểm success/error/auth/ACL/package digest qua HTTP thật.
- [ ] Run audit và ghi added inventory/schema diff. Thiếu baseline thì audit chưa đạt.
- [ ] Handoff A10 URL/process handle, stop/restart commands, DB/storage identity, contract fixtures/release IDs. Development installer release không fixtures phải kết nối được; internal installer vẫn HTTPS-only.
- [ ] Chỉ đóng S01–S04 khi evidence đạt. Thiếu Maven/DB/account/runtime ghi blocker cụ thể và tiếp tục việc độc lập.

## Self-review

S01 chứng minh local environment; S02 catalog/auth/ACL; S03 artifact; S04 integration/additive invariant. Không task nào sửa file/schema upstream. Quyết định ranking còn mở được nêu rõ ở S02, không biến đề xuất thành quyết định đã duyệt. Native tuần tự, contracts thống nhất với app.
