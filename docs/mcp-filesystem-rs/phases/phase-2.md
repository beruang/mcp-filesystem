# Phase 2: Recursive traversal

**Depends on:** Phase 1 (sandbox, config, MCP transport, read tools)
**Risk:** Medium
**Value:** Directory tree exploration, file search, and batch reads — the tools AI coding agents use most

## Purpose

Add recursive directory traversal, glob-based file search, batch file reading, and media file support on top of the Phase 1 sandbox.

## Deliverables

1. `directory_tree` tool with `maxDepth` and `maxDirectoryEntries` enforcement
2. `search_files` tool with glob patterns, exclude patterns, and `maxSearchResults`
3. `list_directory_with_sizes` tool
4. `read_multiple_files` tool with per-file error reporting
5. `read_media_file` tool with base64 output and MIME type inference
6. Symlink-safe traversal (do not follow symlinked directories by default)

## Key Design Decisions

- `walkdir` for directory traversal; `ignore` for gitignore-style exclude patterns in search
- `globset` for compiling glob patterns at tool invocation time
- `read_multiple_files` collects errors per-file rather than failing fast
- `read_media_file` uses `mime_guess` + extension; magic byte sniffing deferred

## Validation

```bash
cargo test --test tool_contract
cargo test --test sandbox_existing_paths

# Manual: directory_tree with depth limit
# Manual: search_files with *.rs pattern
```
