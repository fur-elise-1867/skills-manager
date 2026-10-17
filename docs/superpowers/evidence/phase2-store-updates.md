# Phase 2 P2.4 — additive Store updates provider

Verified locally 2026-10-18 (Asia/Saigon). Provider scope only; the parent owns app metadata client, contracts and `scripts/test-sds-updates-provider.mjs`. No commits. No existing Store file was edited, moved, deleted or restored. The only app-side file written by this work is this report.

## Implementation

Store backend: `D:/Documents/Git Project/skills-manager-app/skills-store/skill-management/backend`.

Five new files:

- `src/main/java/com/furelise/skillmanagement/desktop/updates/DesktopUpdatesController.java`
- `src/main/java/com/furelise/skillmanagement/desktop/updates/DesktopUpdatesService.java`
- `src/main/java/com/furelise/skillmanagement/desktop/updates/DesktopUpdatesDtos.java`
- `src/test/java/com/furelise/skillmanagement/desktop/updates/DesktopUpdatesTest.java`
- `src/test/resources/desktop/updates/run.ps1`

`POST /skm/api/v1/desktop/updates/check`, enabled only for `desktop-pilot`/`desktop-test`, uses the existing protected Spring Security/JWT chain. No new security chain, bean override, duplicate route, repository, dependency, migration or table was added. The existing `/skm` context remains configuration-owned.

Input is exactly `{items:[{skillId,releaseId}]}` with 1–100 unique string pairs. Different releases of the same skill are accepted and retain input order. Explicit JSON validation rejects coercion from numbers, missing/null/extra fields, blank/oversized/control-containing IDs, invalid positive-bigint skill IDs and exact duplicate pairs. Release IDs are bounded opaque strings at the wire boundary; the existing repository treats unrecognized release IDs as absent. Results echo the exact submitted skill/release strings as `skillId`/`currentReleaseId`.

Output uses the existing `ApiResponse` envelope and `{checkedAt,items}`. Status values are `update_available`, `up_to_date`, `unavailable`, `unknown_release`. Successful items contain only the existing three-field `ReleaseRef` (`id`, `sequence`, `version`). Unavailable and unknown-release items omit `latestRelease`; no archive, download path, local target/hash or MCP/token data is returned.

The service has a read-only repeatable-read transaction. Before accessing release metadata it queries the exact existing `SkillRepository.visibleTo` specification using `SkillAccessHelper.viewerParts/viewerGroups`, then calls existing `DesktopCatalogRepository.canDownload`. It uses existing read-only `DesktopReleaseRepository.find/latest`. Withdrawn current releases return unavailable; withdrawn newer releases are excluded by the existing latest query. Newness uses immutable release sequence; business metadata revision and version text do not determine updates. It does not open/download package files or invoke Store mutation services.

Ruling: use the existing visibility specification directly rather than catch `catalog.visible/getVisible` denial exceptions. Those methods are transactional; catching their runtime exception after joining a transaction can leave it rollback-only and break a mixed-result batch. This query retains the same upstream visibility rules without that exception path. ACL parity tests exercise creator/admin/ordinary-viewer behavior. No existing helper was changed.

Ruling: this is the explicitly requested provider slice in the supplied Store checkout, with no commits or extra worktree. The report serves as its execution record. The referenced `docs/superpowers/handoffs/p2-4-store/README.md` was absent when inspected, so the supplied P2.4 plan, delivery-design constraints, API draft §8 and actual pilot source were used. No contract revision was invented.

## Fresh baseline and additive audit

Artifacts are outside both source checkouts:

`C:/Users/Harry/AppData/Local/Temp/p24-store-64a7f7f076644d779f4f4b91e2be7f6d`

Before adding anything, recursively enumerated **all 1,072 existing files** under `D:/Documents/Git Project/skills-manager-app/skills-store` with `Get-ChildItem -File -Recurse -Force`, recording full path, byte length and `Get-FileHash -Algorithm SHA256` in `before.json`. This includes the pilot additions, existing `pom.xml`, wrapper, security, migrations, Git bookkeeping and existing build output. Fresh `after.json` contains 1,077 files: **0 changed, 0 removed, exactly 5 additions**. The direct full-tree comparison needed no exclusions, including for Git bookkeeping. No reset/revert/checkout/restore was used.

The tests ran against byte-identical copies of `pom.xml` and `src` in fresh external directories, so Maven did not overwrite the pre-existing Store `target/`. The new `run.ps1` performs that copy, uses the supplied installed Maven executable, captures a log, and propagates Maven's exit code. The wrapper and POM were neither repaired nor modified.

Before starting Store, ran the existing read-only `desktop-pilot/schema.sql` on the identity-verified dedicated PostgreSQL database, capturing **553 schema objects** in `schema-before.json`. Repeated after startup, tests and live HTTP checks as `schema-after.json`: exact textual equality, **0 schema changes or additions**. Startup validated upstream version 33 and desktop version 3, reporting no migration necessary in either history. No shared database or reset/recreate script was used.

Converted the file inventories to the existing audit's relative-path format (`audit-files-before.json`, `audit-files-after.json`) and ran, from the backend:

```powershell
./desktop-pilot/audit.ps1 `
  -BeforeFiles "$audit/audit-files-before.json" `
  -AfterFiles "$audit/audit-files-after.json" `
  -BeforeSchema "$audit/schema-before.json" `
  -AfterSchema "$audit/schema-after.json"
```

Actual output (`audit.log`):

```text
Additive audit passed: existing source files and schema unchanged; 5 added files (including generated target output), 0 desktop schema objects. Git bookkeeping excluded; raw inventories retained.
```

The audit's stock wording mentions generated target output; all five actual additions are the files listed above, with no new checkout build output. Its own Git exclusion is weaker than the separate full-tree SHA256 comparison, which also passed.

## Maven execution and tests

Verified installed runtime using `mvn.cmd -version`: **Apache Maven 3.9.11**, **Oracle Java 21.0.7**, Windows amd64. Executable:

`C:/Users/Harry/.m2/wrapper/dists/apache-maven-3.9.11/03d7e36a140982eea48e22c1dcac01d8862b2550b2939e09a0809bbc5182a5bc/bin/mvn.cmd`

`JAVA_HOME=C:/Program Files/Java/jdk-21`. Commands from the backend (set `$audit` to the artifact directory above, `$maven` to that executable):

```powershell
./src/test/resources/desktop/updates/run.ps1 -MavenPath $maven -OutputDirectory "$audit/red"
./src/test/resources/desktop/updates/run.ps1 -MavenPath $maven -OutputDirectory "$audit/green"
./src/test/resources/desktop/updates/run.ps1 -MavenPath $maven -Test all -OutputDirectory "$audit/full"
```

The first two execute installed Maven `-B -ntp -Dtest=DesktopUpdatesTest test`; the third executes `-B -ntp test`. Each copies current sources into a fresh directory before running; do not reuse an output directory.

| Run | Actual result |
| --- | --- |
| RED, tests before production files | Exit 1; 11 tests, 10 failures, 0 errors/skips. Auth behavior already passed. Authorized requests failed because the route did not exist (`NoResourceFoundException`, mapped to HTTP500 by existing global advice). |
| GREEN, new provider | Exit 0; **11 tests, 0 failures/errors/skips**, 39.042 seconds. |
| Full Store suite | Exit 0; **138 tests, 0 failures/errors/skips**, 1 minute 24 seconds. Includes existing catalog, release and guard tests. |
| Final full Store suite after review regression | Exit 0; **139 tests, 0 failures/errors/skips**, 1 minute 6 seconds. **DesktopUpdatesTest: 12 passed**, 4.609 seconds. Production code unchanged. |

Logs: `red/maven.log`, `green/maven.log`, `full/maven.log`, `final-full/maven.log`; actual JUnit XML/text under each `target/surefire-reports`. Repeat the full command with `-OutputDirectory "$audit/final-full"` in a fresh directory to reproduce the final run.

The new suite uses real Spring MVC/security, JPA/JDBC and a private H2 database `jdbc:h2:mem:desktopupdates` with `test,desktop-test` profiles, never the shared Store DB. It creates the existing desktop DDL only in that private test database. Eleven tests cover:

- Existing security rejects unauthenticated/invalid bearer requests with 401.
- 1/100 accepted and ordered; 0/101/duplicate pairs rejected with 400; same skill/different releases accepted.
- Malformed/null/missing/non-string/extra JSON fields rejected with the local `INVALID_REQUEST` error envelope.
- Sequence wins over version text and business metadata changes; reference-only output and exact pair identity.
- Revoked/private/missing skill denies before corrupt release metadata can be read; unavailable never exposes latest.
- Creator/admin/viewer parity with pilot visibility/download behavior.
- Withdrawn current unavailable even with another active release; latest skips withdrawn newer releases; no downgrade.
- A release belonging to another skill returns unknown-release with no latest.
- Corrupt authorized metadata returns HTTP503 `RELEASE_UNAVAILABLE`, never false up-to-date.
- Metadata checks succeed with no archive file present and leave release/package rows and business version unchanged.

Existing Hibernate dialect/Flyway version-support and Mockito dynamic-agent warnings remain; no existing configuration was changed to silence them. The full suite had no failures to classify or repair.

## Dedicated runtime and live verification

Initial inspection found no Store JVM/listener on 8080 and no task PostgreSQL listener on 15432. The unrelated PostgreSQL service on port5432 was left alone. Historical PID values were treated as stale.

Restarted only the previously created task cluster using installed PostgreSQL18 tools and its exact directory:

```powershell
& 'D:/Program Files/PostgreSQL/18/bin/pg_ctl.exe' `
  -D 'D:/Documents/Git Project/skills-manager-app/.worktrees/sds-phase0-1/.superpowers/sdd/2026-10-16-sds-app-phase0-1/postgres-pilot/data' `
  -o '-h 127.0.0.1 -p 15432' -l "$audit/postgres.log" -w start
```

Dot-sourced the existing ignored environment source:

`D:/Documents/Git Project/skills-manager-app/.worktrees/sds-phase0-1/.superpowers/sdd/2026-10-16-sds-app-phase0-1/store-env.ps1`

Ran existing `desktop-pilot/start.ps1 -DatabaseUrl $env:DESKTOP_PILOT_JDBC_URL -PsqlPath $pilotPsql -MavenPath $pilotMaven -ValidateOnly` successfully. Independently queried PostgreSQL identity:

```text
sds_desktop_pilot_20261017|127.0.0.1|15432|SDS_DESKTOP_PILOT_ISOLATED_20261017
```

Schema inventory psql commands used `-X -w -At -v ON_ERROR_STOP=1`, loopback15432, the dedicated username/database and `PGPASSWORD` supplied in process memory. One initial inventory invocation lacked `PGPASSWORD` and waited for a prompt; that exact inspected psql child was stopped, then the noninteractive invocation above succeeded. No DB authentication settings were changed.

Checked again for an occupied HTTP port or existing Store JVM before launching installed Maven from `$audit/green`:

```powershell
& $pilotMaven -B -ntp spring-boot:run '-Dspring-boot.run.profiles=desktop-pilot'
```

The copied POM/source remain byte-identical to the originals plus these new provider files. Runtime log: `runtime.log`. At verification:

- **URL:** `http://127.0.0.1:8080/skm/`; health200 and protected updates route401 without authentication.
- **HTTP PID:** 42892, executable `C:/Program Files/Java/jdk-21/bin/java.exe`, created 2026-10-18 00:03:58 +07:00; only loopback8080 listener.
- **Dedicated PostgreSQL PID:** 20876, only task listener `127.0.0.1:15432`.
- Store log confirms profile `desktop-pilot`, Tomcat context `/skm`, no necessary migrations.
- Launch exec session 55762 is retained for ownership information only; recheck process identity before stopping it.

The environment source supplies `DESKTOP_PILOT_*` DB/storage/JWT settings plus `$pilotMaven/$pilotPsql`; it does **not** set `SDS_PROVIDER_EMAIL/PASSWORD`. For the live check those two client variables were loaded into process memory from the existing dedicated development seed credential literals in `DataSeeder.java`, without printing/persisting their values or changing the account. `SDS_PROVIDER_URL=http://127.0.0.1:8080/skm/`. No new credentials file was written.

Ran the parent's existing harness, without editing it, from `.worktrees/sds-phase0-1`:

```powershell
$env:SDS_UPDATES_ITEMS='[{"skillId":"2","releaseId":"94de5a6f-5e44-4770-9e86-6a8da12d7fe5"},{"skillId":"2","releaseId":"15ed525e-f1a3-4dd1-8d44-6d0f756a9f71"}]'
node scripts/test-sds-updates-provider.mjs
```

Exit0, output (`live-harness.log`):

```text
Live updates provider passed: 2 authorized releases, 100-result boundary, 0/101/duplicate rejection, auth and denied metadata privacy; no package downloads.
```

These are actual pre-existing releases of skill2 at sequence1 and sequence3. The harness validates the parent's v1 JSON Schema, login/logout, both current-release pairs, 100 results, 400 boundaries/duplicates, unauthenticated401 and unavailable/unknown metadata privacy over real HTTP/PostgreSQL. It performs no package downloads. No releases were published or withdrawn and no Store business records were edited for live fixtures.

At the parent's follow-up request, reran the current harness against the same freshly reverified Spring JVM and guarded PostgreSQL identity at **2026-10-18T00:11:50.5735706+07:00**. Actual exit0, identical successful live output in `live-harness-final.log`. This is real HTTP/Spring/PostgreSQL evidence, distinct from synthetic fixture smoke. Harness SHA256 at that run: `907a0c1e9ef4239c7c5d4f4a9bcda6f410fad626828cc27a5d4e65d9b3cbf9e2`. Refreshed file/schema inventories after that run and review fixes: **all1,072 original files unchanged, exactly5 additions; all553 schema definitions unchanged**, stock additive audit passed again. Summing the final Surefire XML independently confirmed139 tests,0 failures,0 errors,0 skips.

The runtime is intentionally left available for the parent's consumer/live integration. Reinspect the current listener/JVM before any restart; do not start a duplicate or use historical PID/session values as authority. This is local development HTTP only; internal deployment still requires verified HTTPS.

## Review and limits

Independent read-only provider review found no production correctness defect, and identified two verification/tooling issues. Both were resolved:

- **P2, commit-time transaction coverage:** added `committed_acl_revocation_and_mixed_batch_finish_the_service_transaction` with `Propagation.NOT_SUPPORTED`, committed private H2 fixtures and a batch containing revoked known/unknown pairs plus an authorized current release. It first verifies access, commits revocation, then requires the mixed batch to return200 after the service transaction actually finishes. Cleanup removes only this test's committed fixtures. A mutation in the **external scratch copy only** replaced the visibility lookup with `catalog.visible/getVisible` plus a caught denial. Maven `-Dtest=DesktopUpdatesTest#committed_acl_revocation_and_mixed_batch_finish_the_service_transaction test` then failed exactly once, expected200/actual500 with `UnexpectedRollbackException` (`review-red/maven.log`, exit1). The actual production service was not mutated. The final full run of the real code passed all139 tests, including the new regression.
- **P3, relative runner paths:** resolve the newly created output directory to an absolute path before changing working directory. A separate invocation uses relative `relative-runner-check` while working in the external artifact directory, running `DesktopDatabaseGuardTest`: exit0,2 tests,0 failures/errors/skips,15.385 seconds (`relative-runner-check/maven.log`).

One intermediate focused run (`review-green/maven.log`) had two fixture errors: the new cleanup queried `skill` instead of the actual `skills_master` table, leaving its committed role and causing the following fixture's unique-role error. Inspected the real entity table mapping, corrected only the new test, and reran in fresh private databases. The final139-test run has zero errors/failures/skips. This intermediate setup error is not counted as the intentional mutation RED evidence.

Ruling: fix the small relative-path issue alongside the required regression rather than leave a known defect in the new runner; no existing script was changed. No deferred review findings remain.

No consumer/native/installer completion or overall P2.4/G2 acceptance is claimed here. Withdrawal/revocation state-change cases run against private H2 fixtures; real PostgreSQL live checks use pre-existing active releases and unavailable/missing IDs. The parent owns remaining consumer and native integration. Runtime and raw audit/test artifacts live in an external temporary directory and can be removed by OS temp cleanup; this report retains commands/counts/results.
