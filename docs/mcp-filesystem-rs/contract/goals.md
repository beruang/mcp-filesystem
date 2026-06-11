# Goals

## Primary goals

1. **MCP compatibility** — Full stdio transport implementation using the MCP protocol so any MCP-compatible client can connect.
2. **Strict filesystem sandboxing** — All path operations validated through canonical path comparison; symlink escapes and prefix attacks rejected.
3. **Recursive allowed-root behavior** — Registering `/Volumes/Data` grants access to all descendants without enumerating subdirectories.
4. **Clear read/write/destructive tool semantics** — 13 tools with explicit access kinds; read-only roots reject writes; overlapping roots resolve deterministically.
5. **Single-binary distribution** — `cargo build --release` produces one self-contained binary with no runtime dependencies beyond the OS.
6. **Strong test coverage** — Dedicated test suites for path escape, symlink behavior, readonly roots, overlapping roots, and tool contracts.

## Secondary goals

1. **Config file support** — JSON config for roots, limits, and behavior flags.
2. **Diff output for edits** — `edit_file` produces human-readable unified diffs in dry-run and applied modes.
3. **MIME type inference** — `read_media_file` infers MIME types from file extensions and magic bytes.
4. **Glob-based file search** — `search_files` supports glob patterns with exclude patterns.

## Measurable targets

| Goal | Metric | Target |
|---|---|---|
| MCP compatibility | MCP protocol conformance | All 13 tools respond to valid MCP requests |
| Sandbox security | Symlink escape test pass rate | 100% of escape scenarios rejected |
| Recursive roots | Path authorization test pass rate | 100% of valid descendant paths authorized |
| Single binary | Binary size and startup time | `< 20 MiB`, starts in `< 100ms` |
| Test coverage | Line coverage (approx.) | `> 80%` on sandbox and path resolution modules |

## Priority order

1. Security (sandbox, symlink handling, path validation)
2. MCP protocol correctness
3. Read-only tool completeness
4. Write tool completeness
5. Edit tool with diff support
6. Config file and CLI ergonomics
