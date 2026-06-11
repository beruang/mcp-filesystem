<div align="center">

# mcp-filesystem-rs

**Secure MCP filesystem server with sandboxed access control**

[![Rust](https://img.shields.io/badge/rust-1.85+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/beruang/mcp-filesystem)
[![MCP](https://img.shields.io/badge/MCP-2024--11--05-purple.svg)](https://modelcontextprotocol.io)
[![Tests](https://img.shields.io/badge/tests-34%20passed-brightgreen.svg)](https://github.com/beruang/mcp-filesystem)
[![Clippy](https://img.shields.io/badge/clippy-pedantic%20%2B%20nursery%20%2B%20cargo-brightgreen.svg)](https://github.com/beruang/mcp-filesystem)

</div>

---

## Overview

`mcp-filesystem-rs` is a production-grade [MCP](https://modelcontextprotocol.io) filesystem server that exposes controlled local filesystem access to MCP-compatible clients (Claude Desktop, Claude Code, and other MCP hosts).

Every path operation is validated through **canonical path comparison** — no string-based prefix matching, no symlink escapes, no sandbox breakouts. The server ships as a single static binary with zero runtime dependencies.

### Why this exists

Existing MCP filesystem servers typically use string-based prefix checks (`path.starts_with("/foo")`), which are vulnerable to:

- **Prefix attacks** — `/Volumes/Data` grants access to `/Volumes/Database`
- **Symlink escapes** — `root/link → /etc` then `root/link/passwd` reads `/etc/passwd`
- **Traversal escapes** — `root/../../../etc/passwd`

`mcp-filesystem-rs` uses `std::fs::canonicalize` to resolve every path to its true filesystem location before comparing against allowed roots.

---

## Table of Contents

- [Quick Start](#quick-start)
- [Installation](#installation)
- [Usage](#usage)
  - [CLI Reference](#cli-reference)
  - [Configuration File](#configuration-file)
  - [MCP Client Setup](#mcp-client-setup)
- [Permission Model](#permission-model)
- [Security Model](#security-model)
- [Tools Reference](#tools-reference)
- [Architecture](#architecture)
- [Development](#development)
- [Contributing](#contributing)
- [License](#license)

---

## Quick Start

```bash
# Clone and build
git clone https://github.com/beruang/mcp-filesystem.git
cd mcp-filesystem
make release

# Grant read-write access to a directory
./target/release/mcp-filesystem-rs /Volumes/Data

# Read-only + read-write roots
./target/release/mcp-filesystem-rs \
  --root /Volumes/Data/projects:rw \
  --root /Volumes/Data/reference:ro
```

---

## Installation

### Prerequisites

| Requirement | Version |
|---|---|
| Rust | 1.85+ (edition 2021) |
| OS | macOS or Linux |
| Cargo | Bundled with Rust |

### From source

```bash
git clone https://github.com/beruang/mcp-filesystem.git
cd mcp-filesystem
make release
```

The binary is at `target/release/mcp-filesystem-rs` (~4.5 MB).

### Install via cargo

```bash
cargo install --git https://github.com/beruang/mcp-filesystem.git
```

### Verify

```bash
mcp-filesystem-rs --version
```

---

## Usage

### CLI Reference

```
mcp-filesystem-rs [ROOT] [OPTIONS]

Arguments:
  [ROOT]                        Single root path (shorthand for --root <PATH>:rw)

Options:
  --root <PATH[:ro|rw]>         Allow a root directory (repeatable)
  --config <PATH>               Path to JSON config file
  --max-read-bytes <BYTES>      Maximum bytes for read operations (default: 10 MiB)
  --max-search-results <N>      Maximum search results (default: 1000)
  --max-directory-entries <N>   Maximum directory entries (default: 10000)
  --log-level <LEVEL>           Log level: trace, debug, info, warn, error
  --no-relative-paths           Disallow relative paths
  --allow-relative-paths        Allow relative paths with multiple roots
  --version                     Print version
  --help                        Print help
```

### Configuration File

```bash
mcp-filesystem-rs --config ~/.config/mcp-filesystem/config.json
```

```json
{
  "roots": [
    { "path": "/Volumes/Data/projects", "mode": "readWrite" },
    { "path": "/Volumes/Data/reference", "mode": "readOnly" }
  ],
  "limits": {
    "maxReadBytes": 10485760,
    "maxEditBytes": 10485760,
    "maxSearchResults": 1000,
    "maxDirectoryEntries": 10000,
    "maxTreeDepth": 20
  },
  "behavior": {
    "allowRelativePaths": true,
    "followSymlinkedDirectories": false,
    "dryRunEditsByDefault": true
  }
}
```

### MCP Client Setup

<details>
<summary><b>Claude Desktop</b></summary>

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "/usr/local/bin/mcp-filesystem-rs",
      "args": ["--root", "/Volumes/Data:rw"]
    }
  }
}
```

</details>

<details>
<summary><b>Claude Code</b></summary>

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "/usr/local/bin/mcp-filesystem-rs",
      "args": [
        "--root", "/Volumes/Data/projects:rw",
        "--root", "/Volumes/Data/reference:ro"
      ]
    }
  }
}
```

</details>

<details>
<summary><b>Generic MCP Client</b></summary>

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "/usr/local/bin/mcp-filesystem-rs",
      "args": ["--root", "/home/user/projects:rw"]
    }
  }
}
```

</details>

---

## Permission Model

### Root Modes

| Mode | Read Tools | Write Tools | Use Case |
|---|---|---|---|
| `readOnly` | Allowed | Rejected | Reference data, SDKs, system headers |
| `readWrite` | Allowed | Allowed | Project directories, scratch space |

### Overlapping Roots

When multiple roots match a path, the **most specific** (deepest) root wins. This enables broad read-only access with targeted read-write carve-outs:

```json
{
  "roots": [
    { "path": "/Volumes/Data",           "mode": "readOnly" },
    { "path": "/Volumes/Data/projects",  "mode": "readWrite" }
  ]
}
```

| Path | Matching Root | Mode |
|---|---|---|
| `/Volumes/Data/notes.txt` | `/Volumes/Data` | `readOnly` |
| `/Volumes/Data/projects/main.rs` | `/Volumes/Data/projects` | `readWrite` |

### Relative Paths

Relative paths are resolved against the single root when exactly one root is configured. With multiple roots, relative paths are rejected unless `--allow-relative-paths` is set.

| Configuration | Relative path `notes/todo.md` |
|---|---|
| Single root `/Volumes/Data` | Resolves to `/Volumes/Data/notes/todo.md` |
| Multiple roots | Rejected with `ambiguous_relative_path` error |

---

## Security Model

### Path Validation Flow

```
user path
  → resolve to absolute
  → canonicalize (resolve symlinks, .., .)
  → check canonical path against allowed roots
  → check root access mode (readOnly/readWrite)
  → perform operation
```

### Protections

| Attack | Prevention |
|---|---|
| **Prefix attack** | `canonical_path.starts_with(canonical_root)` — uses `Path`, not strings |
| **Symlink escape** | `std::fs::canonicalize` resolves all symlinks before comparison |
| **Path traversal** | `..` components resolved during canonicalization |
| **TOCTOU race** | Documented as inherent filesystem limitation |
| **Resource exhaustion** | Configurable limits on reads, searches, directory entries |

### Write Path Validation

For operations where the target may not exist yet (`write_file`, `create_directory`, `move_file` destination):

1. Resolve the nearest existing parent
2. Canonicalize the parent (catches symlinks in the path)
3. Validate parent is inside a `readWrite` root
4. Verify remaining path components contain no `..` traversal

### Limits

| Limit | Default | Guards Against |
|---|---|---|
| `maxReadBytes` | 10 MiB | Memory exhaustion from large files |
| `maxEditBytes` | 10 MiB | Memory exhaustion from edit operations |
| `maxSearchResults` | 1,000 | CPU exhaustion from broad searches |
| `maxDirectoryEntries` | 10,000 | CPU exhaustion from large directories |
| `maxTreeDepth` | 20 | Infinite recursion in deep trees |

### Known Limitations

| Limitation | Detail |
|---|---|
| **TOCTOU race** | Symlink may be created between validation and operation. Inherent to filesystem APIs. |
| **Binary detection** | Extension-based; extensionless binary files (ELF, Mach-O) may be misidentified as text. |
| **Cross-filesystem moves** | `move_file` uses `rename(2)` which fails across filesystem boundaries. |
| **Windows paths** | Not supported in v0.1.0 (deferred). |

---

## Tools Reference

### Read Tools

| Tool | Description | Key Parameters |
|---|---|---|
| `read_text_file` | Read a text file | `path`, `head`/`tail` (optional) |
| `read_media_file` | Read binary/media as base64 | `path` |
| `read_multiple_files` | Batch read text files | `paths` (array) |
| `list_directory` | List directory contents | `path` |
| `list_directory_with_sizes` | List directory with file sizes | `path` |
| `directory_tree` | Recursive directory tree | `path`, `maxDepth` (optional) |
| `search_files` | Search by glob pattern | `path`, `pattern`, `excludePatterns` |
| `get_file_info` | File/directory metadata | `path` |
| `list_allowed_directories` | List configured roots | _(none)_ |

### Write Tools

| Tool | Description | Key Parameters |
|---|---|---|
| `write_file` | Create or overwrite a file | `path`, `content`, `createParents` |
| `edit_file` | Pattern-based text editing | `path`, `edits[]`, `dryRun` |
| `create_directory` | Create directories recursively | `path` |
| `move_file` | Move or rename | `source`, `destination` |

### `edit_file` in Detail

```json
{
  "path": "/Volumes/Data/src/main.rs",
  "edits": [
    {
      "oldText": "println!(\"hello\");",
      "newText": "println!(\"hello world\");",
      "replaceAll": false
    }
  ],
  "dryRun": true
}
```

| Feature | Behavior |
|---|---|
| **dryRun** (default: `true`) | Preview changes without writing — returns unified diff |
| **replaceAll** (default: `false`) | When `true`, replaces all occurrences. When `false` and multiple matches exist, returns `edit_pattern_ambiguous` error |
| **Diff format** | Unified diff with 3 lines of context |
| **Line endings** | Normalizes CRLF to LF before matching |

### Error Codes

All errors return structured JSON with `code`, `message`, and optional `path`:

```json
{
  "code": "outside_allowed_roots",
  "message": "Path is outside allowed directories: /etc/passwd",
  "path": "/etc/passwd"
}
```

| Code | Meaning |
|---|---|
| `outside_allowed_roots` | Path resolves outside all configured roots |
| `read_only_root` | Write attempted on a `readOnly` root |
| `path_not_found` | File or directory does not exist |
| `not_a_file` | Directory passed where file expected |
| `not_a_directory` | File passed where directory expected |
| `file_too_large` | File exceeds `maxReadBytes` or `maxEditBytes` |
| `binary_file_not_supported` | Binary file passed to `read_text_file` |
| `edit_pattern_not_found` | `oldText` not found in file |
| `edit_pattern_ambiguous` | Multiple matches and `replaceAll` is `false` |
| `too_many_results` | Result count exceeds configured limit |
| `ambiguous_relative_path` | Relative path with multiple roots |
| `invalid_path` | Empty or malformed path |
| `permission_denied` | OS-level permission error |
| `io_error` | General filesystem I/O error |

---

## Architecture

```
                        MCP Client
                     (Claude Desktop,
                      Claude Code, ...)
                           │
                    stdio (stdin/stdout)
                           │
              ┌────────────┴────────────┐
              │       server.rs         │
              │   MCP JSON-RPC 2.0      │
              │   initialize            │
              │   tools/list            │
              │   tools/call            │
              └────────────┬────────────┘
                           │
              ┌────────────┴────────────┐
              │     sandbox.rs          │
              │   canonicalize()        │
              │   find_best_match()     │
              │   resolve_existing_*()  │
              │   resolve_create_*()    │
              └────────────┬────────────┘
                           │
              ┌────────────┴────────────┐
              │      tools/             │
              │   13 tool handlers      │
              │   read, write, edit,    │
              │   search, tree, info    │
              └─────────────────────────┘
```

### Module Map

| Module | Responsibility |
|---|---|
| `main.rs` | Entry point, CLI parsing, async runtime |
| `config.rs` | CLI + JSON config loading, merge logic |
| `server.rs` | MCP JSON-RPC protocol, stdio transport |
| `sandbox.rs` | Authorization engine, path validation |
| `path.rs` | Path helpers (normalize, resolve, traversal detection) |
| `error.rs` | `FsError` enum (16 variants), structured error responses |
| `tools/*.rs` | 13 MCP tool implementations |

---

## Development

### Setup

```bash
git clone https://github.com/beruang/mcp-filesystem.git
cd mcp-filesystem
make setup-hooks   # Install git pre-commit hook
```

### Makefile Targets

| Target | Description |
|---|---|
| `make check` | Full quality gate: fmt + clippy + test + build |
| `make check-fast` | Fast gate: fmt + clippy only |
| `make check-full` | Full gate with pedantic/nursery/cargo lints |
| `make fmt` | Check formatting |
| `make lint` | Run clippy with `-D warnings` |
| `make lint-full` | Run clippy with all + pedantic + nursery + cargo |
| `make test` | Run all 34 tests |
| `make build` | Debug build |
| `make release` | Release build |
| `make clean` | `cargo clean` |

### Pre-commit Hook

The pre-commit hook (`.githooks/pre-commit`) runs on every commit:

1. `cargo fmt --check`
2. `cargo clippy` (all + pedantic + nursery + cargo)
3. `cargo test` (34 tests)
4. `cargo build --release`

**Bypass flags:**

```bash
SKIP_TESTS=1 git commit -m "..."     # Skip tests
SKIP_CHECKS=1 git commit -m "..."    # Skip all checks
```

### Manual Smoke Test

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.1.0"}}}' | cargo run -- /tmp/test-root
```

---

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feat/my-feature`)
3. Run `make setup-hooks` to install the pre-commit gate
4. Make your changes
5. Commit — the pre-commit hook runs the full quality gate
6. Push and open a pull request

### Quality Requirements

All PRs must pass:

| Gate | Command |
|---|---|
| Formatting | `cargo fmt --check` |
| Linting | `cargo clippy --all-targets -- -D warnings -W clippy::all -W clippy::pedantic -W clippy::nursery -W clippy::cargo` |
| Tests | `cargo test` (34 tests) |
| Build | `cargo build --release` |

---

## License

MIT — see [LICENSE](LICENSE) for details.

---

<div align="center">

**Built with Rust** · **MCP 2024-11-05** · **34 tests** · **13 tools** · **Zero runtime deps**

</div>
