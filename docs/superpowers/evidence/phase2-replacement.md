# Phase 2 replacement and restore evidence

`replacement.rs` stages verified content on the target volume, persists an intent, renames the existing target to owned rollback, publishes the stage, commits deployment/baseline/result and cleans only owned rollback content. Identity, hash and durable evidence are rechecked during reconciliation. Completed intents remain available for validated replay while startup excludes them from pending work.

PreparedOperation retains the pilot wire shape. Lifecycle context/options live in additive tables by operationId. Legacy journal decoding is separate from replacement decoding; migration001 is unchanged. Restore uses the recorded profile/skill/agent/workspace target and rejects ambiguous/unmanaged collisions. Captured edited contents are compared against the saved original release baseline after restore.

Green evidence: `replacement::edited_snapshot_restore_keeps_original_baseline`, all six fault boundaries, actual update/restore twelve-exit matrix, pre-intent exit, stage tampering, recreated roots, unmanaged collision, repeat commit and four update/remove thread races. See [crash matrix](../testing/phase2-crash-matrix.md) for exact assertions and limits.

Native selected update and two restores completed with exact content hashes. The second restore first protected edits made after manual capture. [Native acceptance](phase2-native-acceptance.md) records operations and snapshot IDs. Native recovery after killing the installed GUI remains open.
