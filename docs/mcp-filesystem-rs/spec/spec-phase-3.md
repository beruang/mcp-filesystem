# Spec Phase 3: Writes

## Phase Goal

Add file creation, directory creation, and file move/rename tools. Introduce the write-path validation flow (`resolve_create_write`), overlapping root resolution (most-specific-wins), and read-only root enforcement.

## Dependencies

- Requires: Phase 2 (sandbox, config, read tools, recursive traversal)
- Produces: `write_file`, `create_directory`, `move_file` tools; `resolve_create_write` and `resolve_existing_write` sandbox methods; overlapping root resolution

## Existing Code References

- Pattern to follow: `src/sandbox.rs` — extend with write-path validation
- Related module: `src/path.rs` — `nearest_existing_parent` already exists
- Test pattern: `tests/sandbox_existing_paths.rs` — add create-path variant
- Config pattern: `src/config.rs` — root modes already parsed, need enforcement

## Technical Approach

### Sandbox extensions

Add to `src/sandbox.rs`:

```rust
impl Sandbox {
    pub fn resolve_existing_write(&self, path: &Path) -> Result<ResolvedPath, FsError> {
        let canonical = std::fs::canonicalize(path)?;
        let root = self.find_best_matching_root(&canonical)
            .ok_or(FsError::OutsideAllowedRoots { path: path.to_path_buf() })?;
        root.ensure_access(AccessKind::Write)?;
        Ok(ResolvedPath { requested: path.to_path_buf(), canonical, root })
    }

    pub fn resolve_create_write(&self, path: &Path) -> Result<ResolvedCreatePath, FsError> {
        if path.as_os_str().is_empty() {
            return Err(FsError::InvalidPath { path: path.to_path_buf() });
        }
        let parent = path.parent()
            .ok_or(FsError::InvalidPath { path: path.to_path_buf() })?;
        let existing_parent = nearest_existing_parent(parent)?;
        let canonical_parent = std::fs::canonicalize(&existing_parent)?;
        let root = self.find_best_matching_root(&canonical_parent)
            .ok_or(FsError::OutsideAllowedRoots { path: path.to_path_buf() })?;
        root.ensure_access(AccessKind::Write)?;
        Ok(ResolvedCreatePath {
            requested: path.to_path_buf(),
            canonical_existing_parent: canonical_parent,
            root,
        })
    }
}
```

New type:

```rust
#[derive(Debug, Clone)]
pub struct ResolvedCreatePath {
    pub requested: PathBuf,
    pub canonical_existing_parent: PathBuf,
    pub root: AllowedRoot,
}
```

### Overlapping root resolution

`find_best_matching_root` already sorts by depth (Phase 1). Ensure it's correct:

1. Filter roots where `canonical_path.starts_with(&root.canonical)`
2. Sort matching roots by `root.canonical.components().count()` descending
3. Return first (most specific)

`AllowedRoot` gets `ensure_access(access: AccessKind) -> Result<(), FsError>` that returns `ReadOnlyRoot` error when access is Write and mode is ReadOnly.

### write_file

1. Call `sandbox.resolve_create_write(path)`
2. If `createParents`: `std::fs::create_dir_all(parent_dir)` — but only under the validated canonical parent, not above it
3. Write content: `std::fs::write(path, content)`
4. Return `{path, bytesWritten}`

Note on `createParents`: the validated canonical parent is confirmed safe. If the path has extra components beyond that parent, create those directories. But NEVER create directories above the validated parent (the sandbox already rejected that case).

### create_directory

1. Call `sandbox.resolve_create_write(path)`
2. `std::fs::create_dir_all(path)` — safe because parent path is validated
3. Return `{path, created: true}`

### move_file

1. Validate source exists: `sandbox.resolve_existing_write(source)` — source must be under any allowed root, and we require Write access (moving is destructive to source)
2. Validate destination: `sandbox.resolve_create_write(destination)`
3. `std::fs::rename(source, destination)`
4. Return `{source, destination, moved: true}`

If source and destination are on different filesystems, `std::fs::rename` will fail. Return IO error — do not implement copy+delete fallback in v1.

## File Changes

### New Files

| File | Purpose |
|---|---|
| `src/tools/write_file.rs` | File creation/overwrite tool |
| `src/tools/create_directory.rs` | Directory creation tool |
| `src/tools/move_file.rs` | File move/rename tool |
| `tests/sandbox_create_paths.rs` | Write-path validation tests |
| `tests/readonly_roots.rs` | Read-only root enforcement tests |
| `tests/overlapping_roots.rs` | Overlapping root resolution tests |

### Modified Files

| File | Change |
|---|---|
| `src/sandbox.rs` | Add `resolve_existing_write`, `resolve_create_write`, `ResolvedCreatePath`, `ensure_access` on AllowedRoot |
| `src/tools/mod.rs` | Add 3 new modules |
| `src/server.rs` | Register new write tools |

## Implementation Steps

1. Add `resolve_existing_write` and `ensure_access` to sandbox
2. Add `resolve_create_write` and `ResolvedCreatePath` to sandbox
3. Implement `write_file` tool with `createParents` support
4. Implement `create_directory` tool
5. Implement `move_file` tool
6. Update `src/tools/mod.rs` and `src/server.rs`
7. Write `tests/sandbox_create_paths.rs`
8. Write `tests/readonly_roots.rs`
9. Write `tests/overlapping_roots.rs`

## Data / API / Interface Contract

### write_file input

```json
{
  "path": "/Volumes/Data/notes/todo.md",
  "content": "new content",
  "createParents": true
}
```

### write_file output

```json
{
  "path": "/Volumes/Data/notes/todo.md",
  "bytesWritten": 11
}
```

### move_file input

```json
{
  "source": "/Volumes/Data/old.txt",
  "destination": "/Volumes/Data/new.txt"
}
```

### create_directory output

```json
{
  "path": "/Volumes/Data/new/project",
  "created": true
}
```

## Error Handling

| Scenario | Error Code |
|---|---|
| Destination outside roots | `outside_allowed_roots` |
| Write to read-only root | `read_only_root` |
| Source not found | `path_not_found` |
| Source root is read-only (move) | `read_only_root` |
| Parent does not exist (createParents=false) | `path_not_found` |
| Empty path | `invalid_path` |
| Cross-filesystem move | `io_error` |

## Observability

- Log at `info`: write/create/move operations (path only, never content)
- Log at `warn`: rejected write attempts (outside roots, read-only root)

## Testing Requirements

### Unit Tests

- `find_best_matching_root` with overlapping roots (depth-based selection)
- `ensure_access` rejects Write on ReadOnly root
- `resolve_create_write` with non-existent target path

### Integration Tests

- `tests/sandbox_create_paths.rs`: create files and dirs under allowed root; reject creates outside
- `tests/readonly_roots.rs`: configure read-only root; verify reads work, writes rejected
- `tests/overlapping_roots.rs`: configure overlapping roots; verify most-specific wins

### Regression Tests

None.

## Validation Commands

```bash
cargo test --test sandbox_create_paths
cargo test --test readonly_roots
cargo test --test overlapping_roots
cargo test
```

## Acceptance Criteria

- [ ] `write_file` creates a new file under a readWrite root
- [ ] `write_file` with `createParents: true` creates intermediate directories
- [ ] `write_file` with `createParents: false` and missing parent returns error
- [ ] `write_file` on readOnly root is rejected with `read_only_root` error
- [ ] `create_directory` creates directories recursively
- [ ] `create_directory` outside root is rejected
- [ ] `move_file` renames a file within the same root
- [ ] `move_file` rejects when source is under readOnly root
- [ ] `move_file` rejects when destination is under readOnly root
- [ ] Overlapping roots: most specific root's mode applies

## Risks

| Risk | Severity | Mitigation |
|---|---|---|
| `resolve_create_write` bypass via `..` in unresolved components | Critical | After canonicalizing parent, construct final path by joining canonical_parent + remaining normalized components; reject if result contains `..` |
| Symlink in non-existent path chain | High | `nearest_existing_parent` walks up; if it encounters a symlink to outside, canonicalize reveals it |
| Race between parent resolution and directory creation | Medium | Accept as inherent filesystem limitation; document |
