# Phases: Filesystem MCP Server (Rust)

## Summary

Five sequential phases. Each builds on the prior: read-only MVP → recursive traversal → writes → edits → hardening. Phases 1-3 are the critical path; Phase 4 (edits) and Phase 5 (hardening) can largely parallelize once Phase 3 is stable.

## Dependency Graph

```text
Phase 1: Read-only MVP
  └─ Phase 2: Recursive traversal
       └─ Phase 3: Writes
            ├─ Phase 4: Edits
            └─ Phase 5: Hardening
```

Phase 4 and Phase 5 can run in parallel after Phase 3.

## Phase Index

| ID | Title | Depends On | Spec | Risk | Status |
|---|---|---|---|---|---|
| phase-1 | Read-only MVP | - | spec/spec-phase-1.md | High | Draft |
| phase-2 | Recursive traversal | phase-1 | spec/spec-phase-2.md | Medium | Draft |
| phase-3 | Writes | phase-2 | spec/spec-phase-3.md | High | Draft |
| phase-4 | Edits | phase-3 | spec/spec-phase-4.md | Medium | Draft |
| phase-5 | Hardening | phase-3 | spec/spec-phase-5.md | Medium | Draft |

## Parallelization Notes

- Phase 4 and Phase 5 share no source files. They can run concurrently after Phase 3 completes.
- Phase 4 touches `src/tools/edit_file.rs`. Phase 5 touches `tests/` and documentation. No file conflicts.
- Phase 1-3 must be sequential: each builds APIs the next phase consumes.

## Shared File Risks

| File | Phases | Resolution |
|---|---|---|
| `src/sandbox.rs` | Phase 1 (create), Phase 3 (extend), Phase 5 (harden) | Phase 1 owns creation; Phase 3 adds `resolve_create_write`; Phase 5 adds edge case handling. Sequential, no conflict. |
| `src/tools/mod.rs` | All phases | Each phase adds new tool module declarations. Sequential — later phases append to the module list. |
| `src/server.rs` | All phases | Each phase registers new tools. Sequential appends. |

## Per-Phase Detail Files

- [Phase 1: Read-only MVP](phases/phase-1.md)
- [Phase 2: Recursive traversal](phases/phase-2.md)
- [Phase 3: Writes](phases/phase-3.md)
- [Phase 4: Edits](phases/phase-4.md)
- [Phase 5: Hardening](phases/phase-5.md)
