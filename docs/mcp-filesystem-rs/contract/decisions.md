# Decisions

| # | Decision | Reason | Alternatives considered | Impact |
|---|---|---|---|---|
| D1 | Use `rmcp` crate for MCP protocol | Purpose-built for MCP stdio transport; avoids hand-rolling JSON-RPC | Raw `serde_json` over stdin/stdout, `mcp-sdk` crate | Determines server scaffolding and tool registration pattern |
| D2 | Use `std::fs::canonicalize` for symlink resolution | Built-in, well-tested, handles `..` and symlinks in one call | Manual path normalization with symlink checking | Core security boundary — all path validation depends on this |
| D3 | Sandbox resolves all roots to canonical form at startup | Ensures consistent comparison; catches misconfigured roots early | Lazy canonicalization per request | Startup validation catches config errors before any tool is called |
| D4 | Most-specific-root-wins for overlapping roots | Deterministic, intuitive; deeper path = more specific grant | First-match-wins, union-of-permissions | Users can grant broad read-only with narrow read-write carve-outs |
| D5 | Relative paths only with single root | Avoids ambiguity; simple to explain and implement | Require root identifier in path, reject all relative paths | Single-root is the common case; multi-root users use absolute paths |
| D6 | `edit_file` defaults to `dryRun: true` when client omits the flag | Prevents accidental destructive edits; spec section 10.6 | Default to apply, require explicit dryRun | Safety-first; AI coding agents often preview edits before applying |
| D7 | Do not follow symlinked directories in `directory_tree` and `search_files` by default | Prevents infinite loops and accidental traversal outside the logical tree | Follow symlinks with cycle detection | Configurable via `followSymlinkedDirectories` flag |
| D8 | Use `similar` crate for unified diff generation | Pure Rust, no system dependency on `diff` binary, consistent output across platforms | Shell out to `diff` command, `diffy` crate | `similar` is well-maintained and produces standard unified diff format |
| D9 | Use `walkdir` + `ignore` for `search_files` and `directory_tree` | `walkdir` handles recursion; `ignore` handles gitignore-style exclude patterns | `glob` crate with manual recursion, `jwalk` for parallel walks | Both are well-tested crates; the combination handles ignore files naturally |
| D10 | Limit configuration to JSON file only (no TOML, YAML) | One format reduces maintenance; JSON is universal in MCP ecosystem | TOML (more readable), YAML (more features) | Simpler code, fewer dependencies; users editing MCP config are already in JSON |
| D11 | Tokio for async runtime; `spawn_blocking` for FS ops | MCP is async; filesystem ops are blocking; Tokio is the Rust standard | `async-std`, `smol` | Largest ecosystem, best MCP crate compatibility |

## Superseded decisions

None yet.

## Rejected alternatives

- **Manual JSON-RPC over stdin/stdout** — Rejected in favor of `rmcp`. Building MCP protocol handling from scratch would duplicate work and risk protocol incompatibility.
- **String-based path prefix matching** — Rejected. Explicitly called out in spec section 4 as a security risk. Canonical `Path` comparison only.
- **Recursive `list_allowed_directories`** — Rejected. Would enumerate potentially millions of subdirectories, leaking structure and causing performance issues. Spec section 5.2.
- **Per-operation root configuration** — Rejected. Each tool call specifying a root adds friction. Configured roots at startup is simpler and more secure.
- **`actix-rt` or `warp` for HTTP transport** — Rejected for v1. Stdio is the primary MCP transport. HTTP/SSE is deferred.
