# Phase 4: Edits

**Depends on:** Phase 3 (write tools, write-path validation)
**Risk:** Medium
**Value:** Pattern-based text editing with dry-run preview and diff output — the primary tool AI agents use to modify code

## Purpose

Implement the `edit_file` tool with pattern matching, dry-run mode, unified diff output, single and replaceAll replacement strategies, and ambiguity detection.

## Deliverables

1. `edit_file` tool with `oldText`/`newText` pattern replacement
2. `dryRun` mode (default when client omits the flag)
3. Unified diff output using `similar` crate
4. `replaceAll` flag for multiple-occurrence replacement
5. Ambiguity error when multiple occurrences exist and `replaceAll` is false
6. Pattern-not-found error
7. Binary file rejection
8. File size limit (`maxEditBytes`)

## Key Design Decisions

- `similar` crate for diff generation (pure Rust, no external `diff` binary)
- Default `dryRun: true` when client doesn't specify — safety-first
- Pattern matching does exact substring match, not regex
- Line ending normalization before matching (handle CRLF vs LF)
- Edit operations are applied sequentially within a single call

## Validation

```bash
cargo test --test edit_file

# Manual: dry run edit → verify diff output, file unchanged
# Manual: apply edit → verify file changed, diff matches
# Manual: ambiguous pattern → verify error returned
# Manual: pattern not found → verify error returned
```
