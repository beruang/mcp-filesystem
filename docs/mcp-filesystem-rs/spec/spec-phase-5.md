# Spec Phase 5: Hardening

## Phase Goal

Comprehensive security test suite, limit enforcement, documentation, and release preparation. After this phase, `mcp-filesystem-rs` is ready for `0.1.0` release.

## Dependencies

- Requires: Phase 3 (writes functional, overlapping roots); Phase 4 (edits functional) is independent and can run in parallel
- Produces: Test suites, fixture infrastructure, README, release binary

## Existing Code References

- Pattern to follow: `tests/sandbox_existing_paths.rs` — same temp-dir-based test approach
- Related module: `src/sandbox.rs` — the primary module under test
- Config pattern: `src/config.rs` — limit enforcement testing

## Technical Approach

### Symlink escape test suite (`tests/symlink_escape.rs`)

Creates real symlinks in temp directories using `std::os::unix::fs::symlink`. Test scenarios:

1. Symlink pointing outside root: `root/link -> /etc` — read and write attempts rejected
2. Symlink pointing inside root: `root/link -> root/real` — read and write allowed
3. Symlink to symlink (chain) pointing outside: rejected
4. Symlink to non-existent target inside root: allow (path is inside root)
5. Symlink in parent path for write: `root/link_to_outside/new.txt` rejected because canonical parent resolves outside
6. Dangling symlink: allowed for reads (returns ENOENT), allowed for writes (parent is inside root)
7. Symlink to `/dev/null` or other device files: rejected (canonical resolves outside)

### Read-only root tests (`tests/readonly_roots.rs`)

1. Read text file on readOnly root: allowed
2. Write file on readOnly root: rejected with `read_only_root`
3. Edit file on readOnly root: rejected
4. Create directory on readOnly root: rejected
5. Move file with source on readOnly root: rejected
6. Move file with destination on readOnly root: rejected
7. Read tools on readOnly root: all allowed

### Overlapping root tests (`tests/overlapping_roots.rs`)

1. Broad readOnly + narrow readWrite: writes allowed in narrow, rejected in broad
2. Broad readWrite + narrow readOnly: writes rejected in narrow (most specific wins)
3. Three-level nesting: deepest match wins
4. Adjacent roots (no overlap): no interference

### Large limit tests

1. File exactly at maxReadBytes: allowed
2. File at maxReadBytes + 1: rejected
3. Directory with maxDirectoryEntries entries: all listed
4. Directory with maxDirectoryEntries + 1: truncated, error returned
5. Search returning maxSearchResults: all returned
6. Search returning maxSearchResults + 1: truncated with `too_many_results`

### macOS path quirk handling

On macOS, `/tmp` is a symlink to `/private/tmp`. If user configures root as `/tmp/root`, canonicalization resolves through `/private/tmp/root`. Tests verify this is handled correctly.

### Structured error coverage

Verify all 15 error codes from spec section 11 are produced by at least one test case each.

### README.md

Sections:
1. Overview and features
2. Installation (`cargo install`, pre-built binaries, build from source)
3. Quick start with examples
4. CLI reference (all flags)
5. Config file reference
6. MCP client configuration (Claude Desktop, Claude Code, generic)
7. Permission model explanation
8. Security model (sandbox, symlink handling, limits)
9. Tool reference (all 13 tools with input/output examples)
10. Development and testing instructions

## File Changes

### New Files

| File | Purpose |
|---|---|
| `tests/symlink_escape.rs` | Symlink escape scenarios |
| `tests/readonly_roots.rs` | Read-only root enforcement |
| `tests/overlapping_roots.rs` | Overlapping root resolution |
| `tests/fixtures/` | Test fixture support (programmatic, not checked-in files) |
| `README.md` | User-facing documentation |

### Modified Files

| File | Change |
|---|---|
| `src/sandbox.rs` | Edge case handling for all test scenarios |
| `src/error.rs` | Ensure all 15 error codes are used |
| `src/tools/read_text_file.rs` | Binary detection improvements (magic bytes) |
| `src/tools/directory_tree.rs` | Symlink handling edge cases |
| `src/tools/search_files.rs` | Symlink handling edge cases |
| `Cargo.toml` | Pinned versions, metadata for release |

## Implementation Steps

1. Write `tests/symlink_escape.rs` — all 7 scenarios
2. Run and fix any failing scenarios in sandbox
3. Write `tests/readonly_roots.rs` — all 7 scenarios
4. Run and fix any failing scenarios
5. Write `tests/overlapping_roots.rs` — all 4 scenarios
6. Run and fix any failing scenarios
7. Write large limit tests (extend existing test files)
8. Add magic byte detection for binary files
9. Verify all 15 error codes are covered
10. Test on macOS (handle `/tmp` → `/private/tmp`)
11. Pin dependency versions in `Cargo.toml`
12. Write `README.md`
13. `cargo build --release` and verify binary
14. Run full test suite: `cargo test && cargo clippy && cargo fmt`

## Data / API / Interface Contract

No new tools. All existing tool contracts are validated through the test suite.

## Error Handling

Verify coverage of all error codes:

```text
invalid_path, path_not_found, outside_allowed_roots, permission_denied,
read_only_root, not_a_file, not_a_directory, file_too_large,
binary_file_not_supported, too_many_results, ambiguous_relative_path,
edit_pattern_not_found, edit_pattern_ambiguous, io_error, serialization_error,
unsupported_operation
```

## Observability

- Log at `info`: startup (roots, limits, config path)
- Log at `warn`: all access violations, limit breaches
- Log at `debug`: all tool invocations (path only)
- Never log file contents at any level

## Testing Requirements

### Unit Tests

- Sandbox edge cases discovered during hardening
- Binary detection with magic bytes

### Integration Tests

- All test files listed above
- Cross-platform path handling

### Regression Tests

- Every CVE-class scenario (escape, traversal, injection) has a dedicated test

## Validation Commands

```bash
# Full suite
cargo test
cargo test --test symlink_escape
cargo test --test readonly_roots
cargo test --test overlapping_roots
cargo test --test sandbox_existing_paths
cargo test --test sandbox_create_paths
cargo test --test tool_contract
cargo test --test edit_file

# Quality
cargo clippy -- -D warnings
cargo fmt --check

# Release build
cargo build --release
ls -lh target/release/mcp-filesystem-rs
```

## Acceptance Criteria

- [ ] All symlink escape scenarios rejected
- [ ] All read-only root violations rejected
- [ ] Overlapping root resolution is deterministic (most specific wins)
- [ ] All limits enforced (file size, directory entries, search results)
- [ ] All 15 error codes produced by at least one test
- [ ] Binary builds without warnings
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo fmt --check` passes
- [ ] Binary size < 20 MiB
- [ ] README contains complete client configuration examples
- [ ] Tests pass on macOS (target platform)

## Risks

| Risk | Severity | Mitigation |
|---|---|---|
| Symlink test creation fails on restricted filesystems | Low | Use temp directories; skip tests gracefully if symlink creation is not supported |
| macOS `/tmp` canonicalization differs from Linux | Medium | Add CI matrix for macOS + Linux; handle platform-specific path quirks |
| Finding actual CVEs during hardening | High | Add discovered scenarios to test suite; if fix is invasive, escalate to user |
