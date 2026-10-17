# Phase 2 native acceptance — in progress

Observed on 2026-10-18, Asia/Saigon, using the fixture-free development installer and dedicated loopback Store/PostgreSQL pilot. This is partial acceptance; G2 remains open.

## Selected update and protected restore

Only the registered project-a deployment `a7dbb1c1-3148-4834-ac47-a8c2323f639f` was selected. The pre-existing global deployment remained unselected. The UI detected the owned `phase2-local.txt` edit. Keep current preserved its bytes. A release with a different installName was rejected during prepare before replacement.

The same-name synthetic release `05548974-6462-437b-8ff0-fa8d9f5a66e6`, version1.0.3, displayed its real changelog and missing `phase2-synthetic-mcp` warning. After explicit per-operation acknowledgement, operation `a3f08876-55c5-482d-b3ae-b01a6ae02d71` reported Updated. Automatic snapshot `933609e1-0af6-4aad-972d-073e2c9a361a` retained the exact local edit (SHA256 `cbff516a32bcf930ef537c07a2ba170425cc7bfdb511a2d8cf05939396a175a3`). All three files at the unselected global target retained their original SHA256 values.

Upgraded to final development installer SHA256 `3c4cb9ab64df1256284b064024a7e28f574d411fc94530e00559080969b1e50d` with installer exit0. The existing session, cache, registered workspace and snapshot survived. Backup's Verify snapshot action reported Verified. The restore review displayed project scope/path, original baseline and MCP warning, and required acknowledgement. Operation `ee90329c-96f5-453f-b4fc-09ab1b3ce4c6` reported Snapshot restored successfully. A Before restore snapshot preserved version1.0.3. Filesystem comparison verified all four restored files match their pre-update edited contents; all three unselected global files still match.

Owned machine-readable proofs: `.superpowers/sdd/2026-10-17-sds-phase2/native/{update-snapshot-proof,restore-proof,edited-project-before-update,pilot-target-before}.json`. No AIPro runtime recognition is inferred from synthetic content or MCP detection.

## Built artifacts

Manual snapshot then protected restore was also exercised through the native UI. After Create snapshot reported Local snapshot saved, a new owned `after-manual.txt` was added. Restore operation `fec3a40c-cf6b-4018-b5aa-fe417ae7a93f` reported success. The new edit is absent from the restored target and retained byte-for-byte in Before restore snapshot `1ff26c0c-883e-4ad7-b779-cf9d1e959f9a` (SHA256 `88c47444f9604f523cc5f47e4d6b530e70f1ef3bf2021510d344b5a1dcdf8420`). Proof: `native/manual-restore-proof.json`. The earlier restored release remained marked Local changes against its original baseline.

The latest installed Library at1440x860 client size wraps its64-character source-profile hash within each card and long project/global paths within the table. Minimum-size/dark/keyboard checks remain pending. Native inspection found a stale pilot-era modified-target reason claiming force removal was unavailable; source wording now directs users to snapshot before replacement/removal. This wording-only change postdates the artifacts below and must be included in the final candidate rebuild.

- Development:6,373,398bytes, SHA256 `3c4cb9ab64df1256284b064024a7e28f574d411fc94530e00559080969b1e50d`; installed for observations above.
- Internal:6,374,329bytes, SHA256 `801dbb3fc8b23c94c816f5ed85fb340862b5fc244eee86af130faf6130b7c51c`; build completed successfully; native internal acceptance remains pending.

## Remaining acceptance

Second registered project, snapshot failure UX, modified removal/cache deletion, explicit offline cached deployment and denied/expired UI, interrupted-operation restart, minimum-size/light-dark/keyboard inspection, final full-suite classification, Store audit refresh and independent final review remain open. Automated integration/crash evidence is recorded separately and does not check these native cases automatically.
