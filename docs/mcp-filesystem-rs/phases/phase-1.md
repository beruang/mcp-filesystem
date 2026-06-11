# Phase 1: Read-only MVP

**Depends on:** None
**Risk:** High
**Value:** Working MCP server binary that proves the sandbox, transport, and read tool contracts

## Purpose

Establish the project skeleton, MCP transport, sandbox authorization, and the three foundational read tools. This phase proves the core security boundary works before any write tools are added.

## Deliverables

1. Cargo project with all dependencies declared
2. Config loading (CLI flags + JSON config file)
3. Sandbox module with root canonicalization and path validation
4. Stdio MCP server that responds to initialize requests
5. `list_allowed_directories` tool
6. `read_text_file` tool (with head/tail options, binary rejection, maxReadBytes)
7. `list_directory` tool (with maxDirectoryEntries)
8. `get_file_info` tool
9. Structured error responses for all failure paths

## Key Design Decisions

- Sandbox canonicalizes all roots at startup; rejects invalid roots before any MCP handshake
- `Sandbox::resolve_existing_read` is the single entry point for all read path validation
- Binary detection uses extension + MIME guess; magic byte detection is Phase 5 polish

## Validation

```bash
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt --check

# Manual: start server and send list_allowed_directories
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.1.0"}}}' | cargo run -- /tmp/test-root
```
