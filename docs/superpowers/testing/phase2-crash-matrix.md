# Phase 2 crash matrix

Source checked against `src-tauri/src/sds/tests/{phase2_extra,phase2_lifecycle,recovery,removal,replacement}.rs`. Observed green parent tests are recorded in `.superpowers/sdd/2026-10-17-sds-phase2/scoped-sds-final.log` (150 passed, zero failed). Helper ignore markers prevent accidental standalone runs; the parent tests explicitly execute those helpers. Process tests use isolated temporary roots and the actual test binary, not the installed user's data.

| Operation / parent test | Interruption | Persistent state and asserted outcome |
|---|---|---|
| Update and restore / `phase2_extra::real_process_termination_recovers_each_replacement_boundary` | Actual child exit73 at AfterIntent, OldRenamed, NewRenamed, DatabaseCommitted, CleanupEntry, CleanupFinished; twelve scenarios | Frozen plan/intent saved before exit. At least a verified old or new tree survives; reopen and recover twice yields the expected new fingerprint, Completed status and saved original baseline. Restore remains Modified when snapshot contains edits. |
| Replacement before intent / `phase2_lifecycle::process_exit_before_replacement_intent_keeps_current_target_and_expires_review` | Actual child exit73 after staging, before intent publication | Original target hash equals reviewed fingerprint; new runtime expires review, recovery does not replace current contents and operation becomes Failed. |
| Install / `recovery::a_real_process_kill_at_each_commit_boundary_recovers_without_partial_library_success` | Parent kills child at before-rename, after-rename, after-db | Before rename: target absent, verified stage retained, no deployment and Failed result. Later: complete target bytes, exactly one deployment and Completed result. Busy concurrent commit is refused. |
| Remove / `removal::removal_recovers_real_process_termination_before_after_rename_db_and_partial_cleanup` | Parent kills child at before-rename, after-rename, after-db, partial-cleanup, after-cleanup | Before rename: existing target/deployment retained and Failed result. Later: owned target/quarantine removed, deployment retired, Library release retained and Completed result. |
| Cache cleanup / `phase2_lifecycle::cache_cleanup_survives_actual_process_exit_at_every_durable_boundary` | Actual child exit73 at AfterIntent, AfterRename, AfterDatabase, AfterCleanupEntry, AfterCleanup | Reopen, recover twice and assert the deleted release is absent from Library. Additional reference/fingerprint safety is covered separately; this parent alone does not prove every foreign-file race. |

Fault-return tests in `replacement.rs` also cover all six replacement boundaries with actual manifests/fingerprints and a second recovery. They supplement the process-exit matrix; they are not described as process kills.

## Ambiguity and races

`phase2_extra::recovery_preserves_unmanaged_collision_and_retained_old_copy` introduces unmanaged content after old rename and checks RecoveryRequired twice while retaining current, rollback and stage bytes. `replacement_cannot_write_into_a_recreated_root_after_review` preserves the replacement root and old moved tree. `changed_staged_bytes_are_rejected_before_old_target_rename` keeps the existing target. `repeating_a_completed_replacement_keeps_its_record_and_content` verifies explicit completed replay.

`phase2_extra::competing_update_and_remove_have_only_one_commit_winner` runs four actual thread races with a barrier. Exactly one commit succeeds; winning update leaves one correct deployment, winning remove retires the target while the losing replacement retains verified stage content. `phase2_lifecycle::snapshot_failure_blocks_modified_removal_and_cache_plan_rechecks_new_reference` proves cache cleanup refuses a new pending reference created after review.

Native prepared-operation kill/restart now separately verified review expiry with unchanged target hashes; see phase2-native-acceptance.md. These isolated child-process results do not claim that the installed GUI was killed at each durable boundary.
