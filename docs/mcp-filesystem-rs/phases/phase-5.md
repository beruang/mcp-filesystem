# Phase 5: Hardening

**Depends on:** Phase 3 (writes functional, overlapping roots working)
**Risk:** Medium
**Value:** Production readiness — comprehensive test suite, security validation, documentation, and release artifacts

## Purpose

Add the comprehensive test suite for all security scenarios, enforce all limits, polish error handling, write README, and prepare release binaries. This phase produces a releasable `0.1.0`.

## Deliverables

1. Symlink escape test suite (`tests/symlink_escape.rs`)
2. Read-only root test suite (`tests/readonly_roots.rs`)
3. Overlapping root test suite (`tests/overlapping_roots.rs`)
4. Test fixtures: `tests/fixtures/basic/`, `symlinks/`, `readonly/`
5. Large file limit enforcement and tests
6. Large directory limit enforcement and tests
7. macOS path quirk handling (`/tmp` → `/private/tmp`)
8. Structured error coverage for all 15 error codes
9. README.md with usage, config, MCP client examples (Claude Desktop, Claude Code)
10. `cargo build --release` binary verification

## Key Design Decisions

- Test fixtures are created programmatically in `#[test]` functions; no checked-in fixture files
- Symlink tests create real symlinks in temp directories during test setup
- Limit enforcement uses config values; tests override defaults to verify bounding

## Validation

```bash
cargo test
cargo test --test symlink_escape
cargo test --test readonly_roots
cargo test --test overlapping_roots
cargo test --test sandbox_existing_paths
cargo test --test sandbox_create_paths
cargo test --test tool_contract
cargo test --test edit_file
cargo clippy -- -D warnings
cargo fmt --check
cargo build --release

# Manual: start server, exercise each tool against real directories
# Manual: verify README client config works with Claude Desktop
```
