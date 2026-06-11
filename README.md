# mcp-filesystem-rs

MCP filesystem server with strict sandboxing and recursive root access control. Built in Rust.

Exposes controlled local filesystem access to MCP-compatible clients (Claude Desktop, Claude Code, and other MCP hosts). Every path operation is validated through canonical path comparison — no string-based prefix matching, no symlink escapes, no sandbox breakouts.

## Features

- **Strict sandboxing**: canonical path comparison prevents prefix attacks (`/Volumes/Data` vs `/Volumes/Database`)
- **Recursive roots**: registering `/Volumes/Data` grants access to all descendants
- **Per-root access modes**: `readOnly` and `readWrite` modes with most-specific-root-wins for overlaps
- **Symlink escape prevention**: symlinks pointing outside allowed roots are rejected
- **13 MCP tools**: read, write, edit, move, search, tree, info, and more
- **Single binary**: `cargo build --release` produces one self-contained binary (~4 MB)
- **Stdio transport**: compatible with any MCP client that speaks stdio
- **Configurable limits**: max read bytes, search results, directory entries, tree depth

## Installation

### From source

```bash
git clone https://github.com/user/mcp-filesystem-rs.git
cd mcp-filesystem-rs
cargo build --release
# Binary at target/release/mcp-filesystem-rs
```

### Requirements

- Rust 1.85+ (edition 2021)
- macOS or Linux (Windows support deferred)

## Quick start

```bash
# Grant read-write access to a directory
mcp-filesystem-rs /Volumes/Data

# With explicit mode
mcp-filesystem-rs --root /Volumes/Data:rw --root /Volumes/Reference:ro

# With config file
mcp-filesystem-rs --config ~/.config/mcp-filesystem/config.json
```

## CLI reference

```
mcp-filesystem-rs [ROOT] [OPTIONS]

Arguments:
  [ROOT]  Single root path (shorthand for --root <PATH>:rw)

Options:
  --root <PATH[:ro|rw]>       Allow a root directory (repeatable)
  --config <PATH>              Path to JSON config file
  --max-read-bytes <BYTES>     Maximum bytes for read operations (default 10 MiB)
  --max-search-results <N>     Maximum search results (default 1000)
  --max-directory-entries <N>  Maximum directory entries (default 10000)
  --log-level <LEVEL>          Log level: trace, debug, info, warn, error [default: info]
  --no-relative-paths          Disallow relative paths
  --allow-relative-paths       Allow relative paths even with multiple roots
  --version                    Print version
  --help                       Print help
```

## Config file

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

## MCP client configuration

### Claude Desktop

```json
{
  "mcpServers": {
    "filesystem-rs": {
      "command": "/usr/local/bin/mcp-filesystem-rs",
      "args": ["--root", "/Volumes/Data:rw"]
    }
  }
}
```

### Claude Code

```json
{
  "mcpServers": {
    "filesystem-rs": {
      "command": "/usr/local/bin/mcp-filesystem-rs",
      "args": [
        "--root", "/Volumes/Data/projects:rw",
        "--root", "/Volumes/Data/reference:ro"
      ]
    }
  }
}
```

## Permission model

### Root modes

| Mode | Read tools | Write tools |
|---|---|---|
| `readOnly` | Allowed | Rejected |
| `readWrite` | Allowed | Allowed |

### Overlapping roots

When multiple roots match a path, the most specific (deepest) root wins.

```json
{
  "roots": [
    { "path": "/Volumes/Data", "mode": "readOnly" },
    { "path": "/Volumes/Data/projects", "mode": "readWrite" }
  ]
}
```

- `/Volumes/Data/notes.txt` → readOnly (only broad root matches)
- `/Volumes/Data/projects/main.rs` → readWrite (narrower root wins)

### Relative paths

Relative paths are resolved against the single root when exactly one root is configured. With multiple roots, relative paths are rejected unless `--allow-relative-paths` is set.

## Security model

### Path validation

All paths are validated through `std::fs::canonicalize`, which resolves symlinks and `..` components before comparison. Authorization uses `std::path::Path::starts_with`, never raw string matching. This prevents:

- **Prefix attacks**: `/Volumes/Data` does not match `/Volumes/Database`
- **Symlink escapes**: `root/link -> /etc` is detected because canonicalization resolves the link target
- **Path traversal**: `root/project/../Secrets` resolves to `root/Secrets` and is allowed only if inside root

### Limits

All configurable to prevent resource exhaustion:

| Limit | Default | Description |
|---|---|---|
| `maxReadBytes` | 10 MiB | Maximum file size for reads |
| `maxEditBytes` | 10 MiB | Maximum file size for edits |
| `maxSearchResults` | 1,000 | Maximum search matches |
| `maxDirectoryEntries` | 10,000 | Maximum directory listing entries |
| `maxTreeDepth` | 20 | Maximum tree recursion depth |

### Known limitations

- **TOCTOU race**: A symlink could be created between validation and operation. This is an inherent filesystem API limitation.
- **Cross-filesystem moves**: `move_file` uses `rename` and will fail across filesystem boundaries.
- **Binary detection**: Extension-based; extensionless binary files may be misidentified as text.

## Tools

| Tool | Access | Description |
|---|---|---|
| `list_allowed_directories` | Read | Return configured root directories |
| `read_text_file` | Read | Read a text file (supports head/tail) |
| `read_media_file` | Read | Read binary/media file as base64 |
| `read_multiple_files` | Read | Batch read multiple text files |
| `write_file` | Write | Create or overwrite a file |
| `edit_file` | Write | Pattern-based text editing with diff |
| `create_directory` | Write | Create directories recursively |
| `list_directory` | Read | List directory contents |
| `list_directory_with_sizes` | Read | List directory with file sizes |
| `directory_tree` | Read | Recursive directory tree |
| `move_file` | Write | Move or rename a file |
| `search_files` | Read | Search files by glob pattern |
| `get_file_info` | Read | Get file metadata |

## Development

```bash
# Build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Lint
cargo clippy -- -D warnings

# Format
cargo fmt --check

# Run server manually
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.1.0"}}}' | cargo run -- /tmp/test-root
```

## License

MIT
