# Spec Phase 2: Recursive traversal

## Phase Goal

Add recursive directory tree, glob-based file search, batch text file reading, and media file support on top of the Phase 1 sandbox and read tools.

## Dependencies

- Requires: Phase 1 (sandbox, config, MCP transport, `read_text_file`, `list_directory`, `get_file_info`)
- Produces: `directory_tree`, `search_files`, `list_directory_with_sizes`, `read_multiple_files`, `read_media_file` tools

## Existing Code References

- Pattern to follow: `src/tools/read_text_file.rs` — same sandbox validation, similar error handling
- Related module: `src/sandbox.rs` — `resolve_existing_read()` is the entry point for all read tools
- Test pattern: `tests/sandbox_existing_paths.rs` — create temp dirs and files, validate authorization
- Config pattern: `src/config.rs` — add `maxTreeDepth` default 20 to limits

## Technical Approach

### directory_tree

Use `walkdir` crate. Walk directory recursively with `max_depth` filter. At each entry, check file type. Build a recursive JSON tree structure. Stop recursing when `maxDepth` is reached or `maxDirectoryEntries` is exceeded. Do not follow symlinked directories by default.

Tree node shape:

```rust
struct TreeNode {
    name: String,
    path: String,
    node_type: String, // "file" | "directory" | "symlink" | "other"
    children: Option<Vec<TreeNode>>,
}
```

### search_files

Use `ignore::WalkBuilder` (from `ignore` crate) for gitignore-aware traversal. Apply `globset::GlobSet` for the pattern. Apply `globset::GlobSet` for exclude patterns. Collect matches up to `maxSearchResults`. Return flat list of matching paths.

Pattern matching is case-sensitive by default. Add `caseSensitive: bool` input option.

### list_directory_with_sizes

Extends `list_directory`. After listing entries, use `std::fs::metadata` for each to get `len()`. Directory sizes are set to `null`. Symlinks report size of the link target metadata (not the link file).

### read_multiple_files

Iterates over provided paths. For each: `read_text_file` logic. Collects successes into `files` array and failures into `errors` array. Does not fail-fast — continues on per-file errors.

### read_media_file

Similar to `read_text_file` but reads raw bytes, base64-encodes, and infers MIME type. Uses `mime_guess::from_path` for extension-based detection. Returns `{mimeType, data: "<base64>"}`.

## File Changes

### New Files

| File | Purpose |
|---|---|
| `src/tools/directory_tree.rs` | Recursive directory tree tool |
| `src/tools/search_files.rs` | Glob-based file search tool |
| `src/tools/list_directory_with_sizes.rs` | Directory listing with file sizes |
| `src/tools/read_multiple_files.rs` | Batch text file read |
| `src/tools/read_media_file.rs` | Binary/media file read with base64 + MIME |

### Modified Files

| File | Change |
|---|---|
| `src/config.rs` | Add `max_tree_depth` to Limits struct (default 20) |
| `src/tools/mod.rs` | Add 5 new tool modules, register tools with server |
| `src/server.rs` | Register new tools |
| `Cargo.toml` | Ensure `walkdir`, `ignore`, `globset`, `mime_guess`, `base64` (or manual base64) are declared |

## Implementation Steps

1. Add `max_tree_depth` to config Limits
2. Implement `directory_tree` — walkdir recursion, depth limit, entry count limit
3. Implement `search_files` — ignore::WalkBuilder + globset pattern matching
4. Implement `list_directory_with_sizes` — extend list_directory with size metadata
5. Implement `read_multiple_files` — batch read with per-file error collection
6. Implement `read_media_file` — binary read, base64, MIME inference
7. Update `src/tools/mod.rs` — register all new tools
8. Add tool contract tests for new tools

## Data / API / Interface Contract

### directory_tree input

```json
{
  "path": "/Volumes/Data/project",
  "maxDepth": 5
}
```

### search_files input

```json
{
  "path": "/Volumes/Data/project",
  "pattern": "*.rs",
  "excludePatterns": ["target/**", ".git/**"]
}
```

### read_multiple_files output

```json
{
  "files": [{"path": "...", "content": "..."}],
  "errors": [{"path": "...", "error": {"code": "...", "message": "..."}}]
}
```

### read_media_file output

```json
{
  "mimeType": "image/png",
  "data": "iVBORw0KGgo..."
}
```

## Error Handling

| Scenario | Error Code |
|---|---|
| Path not inside root | `outside_allowed_roots` |
| Search exceeds maxSearchResults | `too_many_results` |
| Tree depth exceeds maxTreeDepth | Truncated, not an error |
| Tree entries exceed maxDirectoryEntries | `too_many_results` |
| File is not a directory (for tree/search/list) | `not_a_directory` |
| Media file exceeds maxReadBytes | `file_too_large` |

## Observability

- Log at `debug` level: tool name, path, and result summary (no file contents)
- Log at `warn` level: directory listing truncated due to maxDirectoryEntries

## Testing Requirements

### Unit Tests

- `directory_tree` depth limiting
- `search_files` glob matching and exclude patterns
- `read_multiple_files` error collection behavior
- MIME type inference from extensions

### Integration Tests

- Real temp directories with nested structure for tree tests
- Mixed file types for search tests
- Batch read with one valid and one invalid path

### Regression Tests

None.

## Validation Commands

```bash
cargo test --test tool_contract
cargo test --test sandbox_existing_paths
```

## Acceptance Criteria

- [ ] `directory_tree` returns nested tree structure for a real directory
- [ ] `directory_tree` respects maxDepth
- [ ] `search_files` finds files matching `*.rs` glob
- [ ] `search_files` excludes patterns in excludePatterns
- [ ] `list_directory_with_sizes` includes file sizes
- [ ] `read_multiple_files` returns per-file errors without failing the whole batch
- [ ] `read_media_file` returns base64 data with correct MIME type
- [ ] Symlinked directories are not followed by default
- [ ] Search results capped at maxSearchResults

## Risks

| Risk | Severity | Mitigation |
|---|---|---|
| Large directory trees exhaust memory building tree | Medium | Enforce maxDirectoryEntries; stream response if possible |
| walkdir following symlinks loops | High | Default to not following symlinks; use walkdir's `follow_links(false)` |
| MIME type wrong for extensionless files | Low | Document limitation; Phase 5 can add magic byte sniffing |
