# Spec Phase 1: Read-only MVP

## Phase Goal

Establish the project skeleton, MCP stdio transport, sandbox authorization module, config loading, and three foundational read tools. After this phase, the binary starts, responds to MCP initialize, and serves `list_allowed_directories`, `read_text_file`, `list_directory`, and `get_file_info`.

## Dependencies

- Requires: None
- Produces: Working binary, sandbox module, 4 MCP tools, config system, structured errors

## Existing Code References

- Pattern to follow: None (greenfield)
- Related module: None
- Test pattern: Rust integration tests in `tests/`; unit tests inline in `src/`
- Config pattern: CLAP derive for CLI; serde_json for config file

## Technical Approach

### Project initialization

```bash
cargo init mcp-filesystem-rs
```

Edit `Cargo.toml` with dependencies from spec section 8.2. Pin versions after initial build succeeds.

### Module structure

```rust
// src/main.rs
mod config;
mod error;
mod sandbox;
mod path;
mod server;
mod tools;

// src/tools/mod.rs
mod list_allowed_directories;
mod read_text_file;
mod list_directory;
mod get_file_info;
```

### MCP transport

Use `rmcp` crate with Tokio. The server runs as a single async task reading from stdin, writing to stdout:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI args
    // Load config
    // Build Sandbox
    // Build Server
    // Run stdio transport
}
```

### Sandbox (src/sandbox.rs)

Core types from spec section 13:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootMode {
    ReadOnly,
    ReadWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessKind {
    Read,
    Write,
}

#[derive(Debug, Clone)]
pub struct AllowedRoot {
    pub original: PathBuf,
    pub canonical: PathBuf,
    pub mode: RootMode,
}

#[derive(Debug, Clone)]
pub struct ResolvedPath {
    pub requested: PathBuf,
    pub canonical: PathBuf,
    pub root: AllowedRoot,
}

#[derive(Debug, Clone)]
pub struct Sandbox {
    roots: Vec<AllowedRoot>,
}
```

Sandbox initialization canonicalizes every root at construction time. Any root that fails canonicalization returns an error — the server does not start.

`Sandbox::find_best_matching_root` — iterates roots, checks canonical path prefix, collects matching roots, sorts by canonical path depth (descending), returns first (most specific).

`Sandbox::resolve_existing_read` — canonicalizes the requested path, finds best matching root, verifies root allows Read access.

### Config (src/config.rs)

CLI struct:

```rust
#[derive(Parser)]
#[command(name = "mcp-filesystem-rs")]
struct Cli {
    #[arg(long, value_parser = parse_root_arg)]
    roots: Vec<RootConfig>,

    #[arg(long)]
    config: Option<PathBuf>,

    #[arg(long)]
    max_read_bytes: Option<u64>,

    #[arg(long)]
    max_search_results: Option<usize>,

    #[arg(long)]
    max_directory_entries: Option<usize>,

    #[arg(long, default_value = "info")]
    log_level: String,

    // Positional: shorthand for single root
    #[arg()]
    positional_root: Option<String>,
}
```

Config file struct matches spec section 16. CLI values override config file values.

### Path helper (src/path.rs)

```rust
pub fn nearest_existing_parent(path: &Path) -> Result<PathBuf, FsError>;
```

Walks up from `path` until it finds a path that exists. Returns error if `/` is reached without finding anything.

### Error (src/error.rs)

Enum with all error codes from spec section 11. Derives `thiserror::Error` and `serde::Serialize`. Each variant maps to a structured MCP error:

```rust
#[derive(Debug, thiserror::Error)]
pub enum FsError {
    #[error("Path is outside allowed directories")]
    OutsideAllowedRoots { path: PathBuf },

    #[error("Path not found: {path}")]
    PathNotFound { path: PathBuf },

    #[error("Not a file: {path}")]
    NotAFile { path: PathBuf },

    #[error("Not a directory: {path}")]
    NotADirectory { path: PathBuf },

    #[error("Read-only root: write denied for {path}")]
    ReadOnlyRoot { path: PathBuf },

    // ... all 15 error codes from spec section 11
}
```

### Tools

**list_allowed_directories:** Returns `Sandbox::list_allowed_directories()` — the configured root paths, not recursive descendants.

**read_text_file:** Validates path via `sandbox.resolve_existing_read()`. Checks path is a file (not dir). Checks size ≤ maxReadBytes. Reads to string. Returns content.

Optional `head: N` returns first N lines. Optional `tail: N` returns last N lines.

**list_directory:** Validates path is a directory. Reads directory entries. Maps each to `{name, path, type}` where type is `file|directory|symlink|other`. Enforces `maxDirectoryEntries`.

**get_file_info:** Uses `std::fs::metadata`. Returns `{path, type, size, created, modified, accessed, readonly}`. Timestamps formatted as ISO 8601.

### Error serialization

Every tool catches `FsError` and converts to MCP error response:

```json
{
  "error": {
    "code": "outside_allowed_roots",
    "message": "Path is outside allowed directories",
    "path": "/Users/kawed/.ssh/id_rsa"
  }
}
```

## File Changes

### New Files

| File | Purpose |
|---|---|
| `Cargo.toml` | Dependencies and metadata |
| `src/main.rs` | Entry point, CLI parsing, MCP transport startup |
| `src/config.rs` | Config loading (CLI + JSON file) |
| `src/error.rs` | `FsError` enum with all error codes |
| `src/sandbox.rs` | `Sandbox`, `AllowedRoot`, `ResolvedPath`, path validation |
| `src/path.rs` | Path helper utilities |
| `src/server.rs` | MCP server setup and tool registration |
| `src/tools/mod.rs` | Tool module declarations |
| `src/tools/list_allowed_directories.rs` | Root listing tool |
| `src/tools/read_text_file.rs` | Text file read tool |
| `src/tools/list_directory.rs` | Directory listing tool |
| `src/tools/get_file_info.rs` | File metadata tool |
| `tests/sandbox_existing_paths.rs` | Sandbox path validation tests |
| `tests/tool_contract.rs` | Tool input/output contract tests |

### Modified Files

None (greenfield).

## Implementation Steps

1. `cargo init mcp-filesystem-rs` and populate `Cargo.toml`
2. Implement `src/error.rs` — FsError enum
3. Implement `src/path.rs` — `nearest_existing_parent`
4. Implement `src/config.rs` — CLI parsing, JSON config loading, merge logic
5. Implement `src/sandbox.rs` — core types, `new()`, `find_best_matching_root()`, `resolve_existing_read()`, `list_allowed_directories()`
6. Implement `src/tools/list_allowed_directories.rs`
7. Implement `src/tools/read_text_file.rs` — with head/tail, binary rejection
8. Implement `src/tools/list_directory.rs`
9. Implement `src/tools/get_file_info.rs`
10. Implement `src/tools/mod.rs` — register all tools
11. Implement `src/server.rs` — MCP server setup with `rmcp`
12. Implement `src/main.rs` — wire CLI → config → sandbox → server → transport
13. Write `tests/sandbox_existing_paths.rs`
14. Write `tests/tool_contract.rs`
15. Build, test, clippy, fmt

## Data / API / Interface Contract

### MCP Tool Registration

Each tool registers with JSON Schema for inputs. Example for `read_text_file`:

```json
{
  "name": "read_text_file",
  "description": "Read a text file within an allowed root",
  "inputSchema": {
    "type": "object",
    "properties": {
      "path": { "type": "string" },
      "head": { "type": "integer", "minimum": 1 },
      "tail": { "type": "integer", "minimum": 1 }
    },
    "required": ["path"]
  }
}
```

### Sandbox API (public)

```rust
impl Sandbox {
    pub fn new(roots: Vec<RootConfig>) -> Result<Self, FsError>;
    pub fn list_allowed_directories(&self) -> Vec<String>;
    pub fn resolve_existing_read(&self, path: &Path) -> Result<ResolvedPath, FsError>;
}
```

## Error Handling

| Scenario | Error Code |
|---|---|
| Path outside all roots | `outside_allowed_roots` |
| Directory where file expected | `not_a_file` |
| File where directory expected | `not_a_directory` |
| File not found | `path_not_found` |
| File exceeds maxReadBytes | `file_too_large` |
| Binary file in read_text_file | `binary_file_not_supported` |
| Relative path with multiple roots | `ambiguous_relative_path` |
| Empty path | `invalid_path` |

## Observability

- Logs: `tracing` at configured level; `info` for server start; `warn` for access violations; `debug` for tool calls; never log file contents
- Metrics: None in v1
- Traces: None in v1
- Alerts: None in v1

## Testing Requirements

### Unit Tests

- `sandbox::find_best_matching_root` with single/multiple/overlapping roots
- `sandbox::resolve_existing_read` with valid paths, path traversal, symlink escape
- `FsError` serialization to JSON
- Config parsing: CLI flags, JSON config, merge precedence

### Integration Tests

- `tests/sandbox_existing_paths.rs`: real temp directories with real files, test authorization
- `tests/tool_contract.rs`: start server in test, send MCP JSON-RPC, verify responses

### Regression Tests

None yet — added in Phase 5.

## Validation Commands

```bash
# Inner loop
cargo test -p mcp-filesystem-rs

# Full validation
cargo test
cargo clippy -- -D warnings
cargo fmt --check
cargo build --release

# Manual smoke test
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.1.0"}}}' | cargo run -- /tmp/test-root
```

## Acceptance Criteria

- [ ] `cargo build` succeeds with all dependencies
- [ ] Binary starts and exits with error if no root configured
- [ ] `list_allowed_directories` returns configured roots
- [ ] `read_text_file` reads a file under an allowed root
- [ ] `read_text_file` rejects paths outside roots
- [ ] `read_text_file` rejects directories
- [ ] `list_directory` lists directory contents
- [ ] `get_file_info` returns correct metadata
- [ ] All rejections return structured error JSON with `code`, `message`, and `path`
- [ ] Sandbox canonicalizes roots at startup
- [ ] Sandbox rejects `/Volumes/DataEvil` when root is `/Volumes/Data`

## Risks

| Risk | Severity | Mitigation |
|---|---|---|
| `rmcp` API mismatch with project needs | High | Verify tool registration, response format, and error handling in first implementation step |
| Canonicalization failures on macOS | Medium | Test on macOS explicitly; handle `/tmp` → `/private/tmp` case |
| Binary detection false positives on extensionless files | Low | Document limitation; add magic byte detection in Phase 5 |
