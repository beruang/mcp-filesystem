# Contract: Filesystem MCP Server (Rust)

**Status:** Approved
**Created:** 2026-06-12
**Updated:** 2026-06-12
**Confidence Score:** 97/100
**Project Slug:** mcp-filesystem-rs
**Source Input:** Detailed specification provided by user — 21-section spec covering goals, non-goals, terminology, authorization model, permission model, path validation, MCP transport, CLI interface, 13 tool contracts, error model, internal architecture, Rust types, sandbox API, security requirements, config spec, test plan, milestones, and release criteria.
**Target Repository:** /Volumes/Workspace/rnd/workflow/mcp/filesystem

## Summary

Build a Rust MCP filesystem server (`mcp-filesystem-rs`) that exposes controlled local filesystem access to MCP-compatible clients. The server enforces strict sandboxing via canonical path comparison, supports recursive allowed-root behavior with per-root read-only/read-write modes, and rejects symlink escapes and prefix attacks. The v1 ships as a single binary with 13 MCP tools over stdio transport, comprehensive test coverage, and JSON config file support.

## Confidence Summary

| Dimension | Score | Reason |
|---|---|---|
| Problem Clarity | 19/20 | Clear need for a secure, sandboxed MCP filesystem server; "why build" is implicit from security requirements |
| Goal Definition | 20/20 | 6 explicit priorities, 13 fully-specified tool contracts, concrete Rust types and API surface |
| Success Criteria | 19/20 | 10 release criteria in spec section 20 are testable; validation commands are implied but not fully enumerated |
| Scope Boundaries | 20/20 | Non-goals enumerated (section 2), 5-phase milestone breakdown, clear in/out per phase |
| Consistency | 19/20 | Minor: `followSymlinks` appears in example config but not formal config spec; otherwise fully internally consistent |

## Problem Statement

Existing MCP filesystem servers lack proper sandboxing — they use string-based path comparison (vulnerable to prefix attacks), don't handle symlink escapes, and lack a clear read/write permission model per root. MCP-compatible clients need controlled, auditable filesystem access without exposing the entire filesystem.

Detailed version: `contract/problem.md`

## Goals

1. MCP protocol compatibility over stdio transport
2. Strict filesystem sandboxing with canonical path comparison
3. Recursive allowed-root behavior without subdirectory enumeration
4. Clear read/write/destructive tool semantics with per-root access modes
5. Single-binary distribution via `cargo build --release`
6. Strong test coverage for path escape and symlink behavior

Detailed version: `contract/goals.md`

## Success Criteria

- Binary starts over stdio with MCP initialize handshake
- All 13 tools respond correctly to valid MCP requests
- Symlink escapes and prefix attacks are rejected in 100% of test scenarios
- Overlapping roots resolve to the most specific match
- Structured MCP errors returned for all failure paths
- README contains client configuration examples

Detailed version: `contract/success-criteria.md`

## Scope Boundaries

**In scope:** Stdio MCP transport, 13 tools (read/write/search/edit/inspect), recursive sandbox, per-root modes, overlapping root resolution, symlink escape prevention, relative path support (single-root), JSON config, CLI, structured errors, unified diffs, comprehensive test suite.

**Out of scope:** HTTP/SSE transport, command execution, file watching, backup/sync, Windows path handling (v1), recursive `list_allowed_directories`.

Detailed version: `contract/scope.md`

## Constraints

- All path comparisons use `std::path::Path`, never raw strings
- Canonicalization via `std::fs::canonicalize` resolves symlinks and `..`
- Blocking FS ops on `tokio::task::spawn_blocking`
- Rust edition 2024 or 2021; specific crate set required
- No `unsafe` blocks without documentation
- Configurable limits for reads, searches, and directory entries

Detailed version: `contract/constraints.md`

## Assumptions

`contract/assumptions.md`

## Decisions

`contract/decisions.md`

## Risks

`contract/risks.md`

## Approval

- Status: Approved
- Approved By: User
- Approved At: 2026-06-12
