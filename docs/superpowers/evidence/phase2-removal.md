# Phase 2 target removal and cache cleanup evidence

Modified removal requires explicit local-edit acknowledgement and supports selected snapshot backup. A requested backup failure stops removal. Missing-target retirement remains scoped to its recorded deployment. Cache deletion is separate, reviewed, journaled and refused while deployments/pending work reference the release. Snapshot content does not depend on the cache row.

Green `phase2_lifecycle` tests cover backup/remove/cache-delete/restore with saved baseline, backup failure preserving target and deployment, and a new cache reference created after cleanup review. Existing `removal.rs` covers other target/Library/MCP preservation, cancellation, new files in quarantine, DB failures and actual child kills at five removal boundaries. Cache cleanup actual exit73 runs five boundaries with repeated recovery; see [crash matrix](../testing/phase2-crash-matrix.md).

Native modified-target review was opened for project-a with backup selected, explicit acknowledgement and exact destination. Its final Back up and remove action is awaiting action-time human confirmation required by the Windows Computer Use skill. It has not executed. Native cache deletion is likewise not claimed. Automated lifecycle evidence is separate from those open UI cases.
