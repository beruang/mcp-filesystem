# Success Criteria

## Acceptance criteria

- [ ] Binary `mcp-filesystem-rs` starts over stdio and accepts MCP initialize requests.
- [ ] At least one root is required; server exits with error if no roots configured.
- [ ] `list_allowed_directories` returns only the configured roots, not recursive descendants.
- [ ] All 8 read tools (`read_text_file`, `read_media_file`, `read_multiple_files`, `list_directory`, `list_directory_with_sizes`, `directory_tree`, `search_files`, `get_file_info`) function correctly on authorized paths.
- [ ] All 4 write tools (`write_file`, `edit_file`, `create_directory`, `move_file`) respect `readWrite` mode and reject on `readOnly` roots.
- [ ] Symlink escape (`root/link -> /etc` then `root/link/passwd`) is rejected.
- [ ] Sibling prefix attack (`/Volumes/Data` vs `/Volumes/Database`) is rejected.
- [ ] Recursive root authorization: `/Volumes/Data/a/b/c/file.txt` is allowed when `/Volumes/Data` is a root.
- [ ] Internal symlinks (`root/link -> root/real`) are allowed because canonical target stays inside root.
- [ ] Overlapping roots resolve to the most specific matching root.
- [ ] Large reads are bounded by `maxReadBytes` (default 10 MiB).
- [ ] Large directory listings are bounded by `maxDirectoryEntries` (default 10,000).
- [ ] Search results are bounded by `maxSearchResults` (default 1,000).
- [ ] All errors return structured MCP-compatible error responses with `code`, `message`, and optional `path`.
- [ ] `edit_file` supports `dryRun`, `replaceAll`, and ambiguity detection.
- [ ] Relative paths work with a single root; rejected with multiple roots.
- [ ] CLI accepts `--root`, `--config`, and limit overrides.
- [ ] README contains client configuration examples for Claude Desktop and Claude Code.

## Validation checks

```bash
# Build
cargo build --release

# Unit and integration tests
cargo test

# Clippy lints
cargo clippy -- -D warnings

# Format check
cargo fmt --check

# Manual: start server and send MCP initialize via stdio
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.1.0"}}}' | mcp-filesystem-rs /tmp/test-root
```

## Artifact checks

- [ ] Binary at `target/release/mcp-filesystem-rs`
- [ ] `README.md` with usage, config, and MCP client examples
- [ ] `Cargo.toml` with pinned dependency versions
- [ ] Test fixtures under `tests/fixtures/`

## Quality checks

- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo fmt --check` passes
- [ ] All tests pass on macOS and Linux
- [ ] No `unsafe` blocks (or documented and minimized if unavoidable)

## Done/not-done boundary

**Done:** Binary starts, all 13 tools work, sandbox rejects all escape attempts, tests pass, README is complete, and the binary can be configured as an MCP server in Claude Desktop or Claude Code.

**Not done:** HTTP/SSE transport, file watching, backup/sync, arbitrary command execution, recursive `list_allowed_directories`, Windows-specific path handling (v1 targets macOS/Linux).
