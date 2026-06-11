# Spec Index: Filesystem MCP Server (Rust)

## Summary

Five sequential phase specs implementing a Rust MCP filesystem server with strict sandboxing, recursive root behavior, per-root access modes, and 13 MCP tools over stdio transport.

## Specs

| Spec | Title | Depends On | Purpose | Status |
|---|---|---|---|---|
| spec-phase-1.md | Read-only MVP | - | Project skeleton, MCP transport, sandbox, 4 read tools | Draft |
| spec-phase-2.md | Recursive traversal | Phase 1 | Directory tree, file search, batch reads, media files | Draft |
| spec-phase-3.md | Writes | Phase 2 | File creation, directory creation, move/rename, overlapping roots | Draft |
| spec-phase-4.md | Edits | Phase 3 | Pattern-based text editing with diff output | Draft |
| spec-phase-5.md | Hardening | Phase 3 | Comprehensive tests, limits, docs, release | Draft |

## Recommended Reading Order

1. `../contract.md` — what and why
2. `../phases.md` — implementation breakdown and dependency graph
3. `spec-phase-1.md` — start here for implementation
4. `spec-phase-2.md` through `spec-phase-5.md` — sequential

## Agent Loading Guidance

Implementation agents should start with:

1. `.agent/contracts/mcp-filesystem-rs/manifest.json`
2. `.agent/contracts/mcp-filesystem-rs/specs.index.ndjson`
3. The specific phase spec assigned to them
4. `../contract/constraints.md` for file layout and naming rules
5. `../contract/decisions.md` for technical choices already made
