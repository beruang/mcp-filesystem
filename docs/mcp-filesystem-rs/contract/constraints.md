# Constraints

## File layout constraints

- Source layout follows the structure in spec section 12: `src/main.rs`, `src/config.rs`, `src/server.rs`, `src/sandbox.rs`, `src/path.rs`, `src/error.rs`, `src/tools/*.rs`.
- Tests live under `tests/` with one file per concern.
- Test fixtures live under `tests/fixtures/`.
- Human docs live under `docs/mcp-filesystem-rs/`.
- Agent artifacts live under `.agent/contracts/mcp-filesystem-rs/`.

## Naming constraints

- Project slug: `mcp-filesystem-rs`.
- Binary name: `mcp-filesystem-rs`.
- Crate name: `mcp-filesystem-rs`.
- Rust types match spec section 13: `RootMode`, `AccessKind`, `AllowedRoot`, `Sandbox`, `ResolvedPath`.

## Codebase constraints

- Greenfield project — no existing code to maintain compatibility with.
- Use `std::path::Path` for all path comparisons, never raw strings.
- Use `std::fs::canonicalize` for resolving symlinks and `..` components.
- Blocking filesystem operations must run on Tokio's blocking threadpool via `tokio::task::spawn_blocking`.

## Tool constraints

- Rust edition 2024 (or 2021 if 2024 is not widely supported).
- Required crates: `rmcp`, `tokio`, `serde`, `serde_json`, `schemars`, `thiserror`, `clap`, `tracing`, `tracing-subscriber`, `walkdir`, `ignore`, `globset`, `regex`, `similar`, `mime_guess`.
- Pin exact versions before `0.1.0` release.
- No `unsafe` blocks without explicit documentation and justification.

## Context constraints

- Spec files stay under 400 lines; split into templates if larger.
- NDJSON records are single-line; no Markdown in NDJSON.
- Contract detail files stay under 200 lines each.

## Security constraints

- No path comparison using raw strings — always use canonical `Path` comparison.
- Reject any path whose canonical form escapes all allowed roots.
- Do not follow symlinks during directory traversal unless explicitly configured.
- Default `maxReadBytes` to 10 MiB to prevent memory exhaustion.
- Default `maxDirectoryEntries` to 10,000 to prevent CPU exhaustion.
- Default `maxSearchResults` to 1,000.
- Never expose the server's own filesystem outside configured roots.
- Log access violations at `warn` level; do not log file contents.
