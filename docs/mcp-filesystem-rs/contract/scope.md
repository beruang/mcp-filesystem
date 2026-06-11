# Scope

## In scope

- Stdio MCP transport using the `rmcp` crate and Tokio runtime.
- Recursive allowed-root sandbox with canonical path comparison.
- 13 MCP tools: `list_allowed_directories`, `read_text_file`, `read_media_file`, `read_multiple_files`, `write_file`, `edit_file`, `create_directory`, `list_directory`, `list_directory_with_sizes`, `directory_tree`, `move_file`, `search_files`, `get_file_info`.
- Per-root `readOnly` / `readWrite` access modes.
- Overlapping root resolution (most specific wins).
- Symlink escape prevention via `std::fs::canonicalize`.
- Path traversal handling (`..` resolution inside roots).
- Relative path support (single-root only).
- JSON config file with roots, limits, and behavior flags.
- CLI with `--root`, `--config`, limit overrides, and log level.
- Structured MCP error responses with error codes.
- Unified diff output for `edit_file`.
- Glob-based file search with exclude patterns.
- MIME type inference for media files.
- Configurable limits: `maxReadBytes`, `maxEditBytes`, `maxSearchResults`, `maxDirectoryEntries`, `maxTreeDepth`.
- Comprehensive test suite covering sandbox, symlinks, readonly, overlapping roots, and tool contracts.

## Out of scope

- HTTP/SSE transport (deferred to future version).
- Arbitrary command execution.
- File watching / change notification.
- Backup, sync, or versioning.
- Database indexing.
- Permission management (chmod/chown).
- Windows-specific path handling (v1 targets macOS and Linux).
- Recursive enumeration in `list_allowed_directories`.
- Write-only root mode (only `readOnly` and `readWrite` exist).
- Multi-root relative path resolution.
- Content-based binary detection (uses extension and MIME sniffing only).

## Future considerations

- HTTP/SSE transport for remote MCP connections.
- `--workspace` flag for relative path resolution with multiple roots.
- Checksum/ETag support for file reads.
- Streaming large file reads.
- File locking for concurrent write safety.
- Docker image distribution.
- Homebrew / cargo-binstall installation.

## Explicitly deferred

- Windows path support — deferred until after Linux/macOS test suite is stable. Reason: Windows path semantics (UNC, drive letters, `\\?\` prefixes) add complexity without blocking the primary use case.
