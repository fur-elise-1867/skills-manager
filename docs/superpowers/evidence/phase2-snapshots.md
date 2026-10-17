# Phase 2 snapshot evidence

`SnapshotService` stores self-contained captured contents plus release, target identity and separate original baseline metadata. Public IDs are validated UUIDs; no raw snapshot content path is exported. Capture holds target pins/lock, compares source fingerprints around copy and validates copied hashes before publishing metadata. Existing depth32/entry10000/expanded524288000 bounds remain in force.

The saved scoped run contains green essential snapshot tests and `phase2_extra` cases for missing bytes, destination occupied by a file, depth33, path escape IDs, source mutation at BeforeCopy/AfterCopy and an injected post-copy write fault. Failure cases preserve current files and publish no completed snapshot. Added/changed/deleted resources, including missing SKILL.md, are supported independently of new-package validation. The depth regression initially crashed with STATUS_STACK_OVERFLOW; the scanner buffer now uses heap storage without widening limits.

`phase2_lifecycle::modified_removal_backup_then_cache_delete_then_restore_without_cache_preserves_baseline` deletes the live deployment and original cache, verifies its snapshot and restores edited content with the original baseline. Snapshot records therefore survive those lifecycle changes.

Native Verify snapshot reported Verified. Automatic before-update, manual capture and before-restore protection were inspected and hash-compared; see [native acceptance](phase2-native-acceptance.md). Native snapshot-destination failure UX remains open. Injected faults establish failure safety, not physical disk-full behavior on the installed machine.
