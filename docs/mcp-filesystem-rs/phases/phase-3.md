# Phase 3: Writes

**Depends on:** Phase 2 (recursive traversal, search)
**Risk:** High
**Value:** Write capability — the distinction between read-only and read-write roots becomes meaningful

## Purpose

Add file creation, directory creation, and file move/rename tools. This phase introduces the write-path validation flow (`resolve_for_create`) and the overlapping root resolution logic.

## Deliverables

1. `write_file` tool with `createParents` option
2. `create_directory` tool with recursive creation
3. `move_file` tool with cross-root detection and read-only source rejection
4. `Sandbox::resolve_create_write` — parent-resolution validation for non-existent paths
5. `Sandbox::resolve_existing_write` — full-path validation for existing write targets
6. Most-specific-root-wins resolution for overlapping roots
7. Read-only root write rejection with structured errors

## Key Design Decisions

- `resolve_for_create` walks up to find nearest existing parent, canonicalizes it, validates it's inside a readWrite root, then constructs the target path from validated parent + remaining components
- `move_file` validates source with `resolve_existing_write` (source must be under a writable root) and destination with `resolve_create_write`
- `createParents: true` creates intermediate directories; validation happens against the nearest existing ancestor

## Validation

```bash
cargo test --test sandbox_create_paths
cargo test --test readonly_roots
cargo test --test overlapping_roots

# Manual: write file, move file, create directory in readWrite root
# Manual: attempt write in readOnly root → must be rejected
```
