# Assumptions

| # | Assumption | Confidence impact | What would invalidate |
|---|---|---|---|
| 1 | `rmcp` crate provides sufficient MCP protocol support for stdio transport and tool registration | Medium | If `rmcp` lacks tool registration or has protocol bugs, may need to switch to raw JSON-RPC or another crate |
| 2 | Tokio's `spawn_blocking` is adequate for filesystem operations without a separate threadpool | Low | If profiling shows blocking overhead, may need a dedicated filesystem threadpool |
| 3 | `std::fs::canonicalize` resolves symlinks correctly on both macOS and Linux | Low | Some edge cases (e.g., deleted symlink targets, permission-denied intermediates) need explicit error handling |
| 4 | macOS and Linux path semantics are sufficiently similar for v1 | Medium | If Darwin has unique path quirks (e.g., `/private` prefix), they need separate tests |
| 5 | `similar` crate's unified diff output is acceptable for `edit_file` diff format | Low | If clients need a different diff format, add format selection |
| 6 | `mime_guess` crate plus extension-based detection is sufficient for MIME type inference | Low | If accuracy is insufficient, add magic-byte sniffing with `infer` or `tree_magic` crate |
| 7 | Single binary distribution (`cargo build --release`) meets user needs; no package manager integration required for v1 | Medium | If users request `brew install` or `cargo install`, add packaging tasks post-v1 |
| 8 | Config file at `~/.config/mcp-filesystem/config.json` is an acceptable default location | Low | If users need XDG config dir or a different default, add env var override |

## Open questions

1. Should `edit_file` use `similar` (pure Rust) or shell out to `diff` for unified diffs? Decision: use `similar` for portability; revisit if output quality is insufficient.
2. Should `read_media_file` support streaming/chunking for large files? Decision: defer to post-v1; v1 reads entire file into memory bounded by `maxReadBytes`.
3. Should the server support MCP resource templates in addition to tools? Decision: defer; tools-only for v1.

## Unknowns

- Performance of `walkdir` + `ignore` for large directory trees — needs benchmarking with 100k+ file trees.
- Whether MCP clients expect specific JSON Schema shapes for tool input/output — will validate against Claude Desktop and Claude Code.
