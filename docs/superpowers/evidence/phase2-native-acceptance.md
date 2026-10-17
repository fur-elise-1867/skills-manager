# Phase 2 native acceptance — in progress

Observed on 2026-10-18, Asia/Saigon, using the fixture-free development installer and dedicated loopback Store/PostgreSQL pilot. This is partial acceptance; G2 remains open.

## Selected update and protected restore

Only the registered project-a deployment `a7dbb1c1-3148-4834-ac47-a8c2323f639f` was selected. The pre-existing global deployment remained unselected. The UI detected the owned `phase2-local.txt` edit. Keep current preserved its bytes. A release with a different installName was rejected during prepare before replacement.

The same-name synthetic release `05548974-6462-437b-8ff0-fa8d9f5a66e6`, version1.0.3, displayed its real changelog and missing `phase2-synthetic-mcp` warning. After explicit per-operation acknowledgement, operation `a3f08876-55c5-482d-b3ae-b01a6ae02d71` reported Updated. Automatic snapshot `933609e1-0af6-4aad-972d-073e2c9a361a` retained the exact local edit (SHA256 `cbff516a32bcf930ef537c07a2ba170425cc7bfdb511a2d8cf05939396a175a3`). All three files at the unselected global target retained their original SHA256 values.

Upgraded to final development installer SHA256 `3c4cb9ab64df1256284b064024a7e28f574d411fc94530e00559080969b1e50d` with installer exit0. The existing session, cache, registered workspace and snapshot survived. Backup's Verify snapshot action reported Verified. The restore review displayed project scope/path, original baseline and MCP warning, and required acknowledgement. Operation `ee90329c-96f5-453f-b4fc-09ab1b3ce4c6` reported Snapshot restored successfully. A Before restore snapshot preserved version1.0.3. Filesystem comparison verified all four restored files match their pre-update edited contents; all three unselected global files still match.

Owned machine-readable proofs: `.superpowers/sdd/2026-10-17-sds-phase2/native/{update-snapshot-proof,restore-proof,edited-project-before-update,pilot-target-before}.json`. No AIPro runtime recognition is inferred from synthetic content or MCP detection.

## Built artifacts

Final rebuild after the guidance correction completed for both profiles: development6,371,640bytes/SHA256 `82226e2d886949da0444f8fd8721ae550d0fb8d7e40816bc53e78aca93a6b3b8`, internal6,373,172bytes/SHA256 `688231d49d199a166beba02cc17308d00953c7ea0dfde7650ede56cfe441df69`. Final development installed into the same owned pilot shell with exit0. Earlier hashes below identify the exact native candidates used for prior observations and are superseded as delivery artifacts.

Manual snapshot then protected restore was also exercised through the native UI. After Create snapshot reported Local snapshot saved, a new owned `after-manual.txt` was added. Restore operation `fec3a40c-cf6b-4018-b5aa-fe417ae7a93f` reported success. The new edit is absent from the restored target and retained byte-for-byte in Before restore snapshot `1ff26c0c-883e-4ad7-b779-cf9d1e959f9a` (SHA256 `88c47444f9604f523cc5f47e4d6b530e70f1ef3bf2021510d344b5a1dcdf8420`). Proof: `native/manual-restore-proof.json`. The earlier restored release remained marked Local changes against its original baseline.

The latest installed Library at1440x860 client size wraps its64-character source-profile hash within each card and long project/global paths within the table. Minimum-size/dark/keyboard checks remain pending. Native inspection found a stale pilot-era modified-target reason claiming force removal was unavailable; source wording now directs users to snapshot before replacement/removal. This wording-only change postdates the artifacts below and must be included in the final candidate rebuild.

- Development:6,373,398bytes, SHA256 `3c4cb9ab64df1256284b064024a7e28f574d411fc94530e00559080969b1e50d`; installed for observations above.
- Internal:6,374,329bytes, SHA256 `801dbb3fc8b23c94c816f5ed85fb340862b5fc244eee86af130faf6130b7c51c`; build completed successfully; native internal acceptance remains pending.

## Remaining acceptance

Window was resized using the native system Size control to outer1102x672/client1100x640. Library and Backup were inspected in both light and dark themes; long hashes/paths wrapped within cards/table cells, the toolbar remained available and vertical content scrolled. Project picker selection was exercised with keyboard End/Return. This is not yet a complete keyboard/focus audit of every lifecycle dialog.

After offline deployment the owned Store was restarted and Retry connection returned Library to online install actions. A fresh guard check confirmed the same isolated database on loopback15432. File/schema inventories taken after synthetic publication and native workflows passed the stock additive audit: all original Store source and schema definitions unchanged, five permitted additions, zero new schema objects. Raw current inventories are `store-files-native-final.json` and `store-schema-native-final.json` under the owned task artifact root.

Actual Store outage was repeated by stopping only the process-identity-verified dedicated Store JVMs, leaving PostgreSQL intact. The Library disabled update checks and showed explicit prior-grant/unknown-current-permission guidance. Use cached copy for the old pilot ZIP was refused because that release had no persisted grant for this account/profile; no target was created. Registered project-b through the Windows folder picker, then installed the prior-authorized7z1.0.1 via Use cached copy, project selection by keyboard and MCP review. Operation `7e922207-779c-4f51-a2c9-8234b919474c` reported Installed successfully. At verification, the Store listener count was0; all three project-b files matched the known release hashes and project-a's local edit remained intact. Proof: `native/offline-cache-proof.json`. This establishes actual offline deployment and separate missing-grant rejection; it does not stand in for denied/expired UI acceptance.

Second registered project, snapshot failure UX, modified removal/cache deletion, explicit offline cached deployment and denied/expired UI, interrupted-operation restart, minimum-size/light-dark/keyboard inspection, final full-suite classification, Store audit refresh and independent final review remain open. Automated integration/crash evidence is recorded separately and does not check these native cases automatically.
