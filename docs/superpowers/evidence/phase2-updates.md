# Phase 2 selected update evidence

Metadata consumer/provider evidence is in [consumer](phase2-update-consumer.md) and [Store provider](phase2-store-updates.md). The consumer deduplicates exact pairs, chunks at100, validates response completeness/privacy and uses one captured session context. Metadata checks do not download packages.

Selected groups persist frozen per-target PreparedOperation payloads alongside operation IDs so the UI can review the actual new release/MCP requirements. Execution returns promptly through the runtime command and authoritative polling supplies results. Work is serial within a group; failure and successful items remain distinct. Retry prepares a fresh review for failed targets and does not replay successful ones. A different installName requires an explicit unsupported-migration outcome.

Green lifecycle tests cover one selected target among global/two projects, new MCP acknowledgement, Keep with zero network, permission revocation, snapshot failure preserving current bytes, installName rejection and failed-target fresh retry. Contract fixtures include frozen group operations and current target fingerprints; actual Rust serde-to-TypeScript parity passed.

Native selection changed project-a only after its new MCP warning/changelog review. Keep preserved edits; the earlier changed-installName candidate was refused. Updated snapshot/restore hashes and unselected global hashes are in [native acceptance](phase2-native-acceptance.md). Successful provider tests and native same-name update do not imply automatic updating or agent runtime recognition.
