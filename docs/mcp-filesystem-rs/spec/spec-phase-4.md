# Spec Phase 4: Edits

## Phase Goal

Implement the `edit_file` tool with pattern-based text replacement, dry-run preview, unified diff output, replaceAll support, and ambiguity detection.

## Dependencies

- Requires: Phase 3 (write tools, write-path validation)
- Produces: `edit_file` tool with full edit contract

## Existing Code References

- Pattern to follow: `src/tools/write_file.rs` — same sandbox validation for existing file write
- Related module: `src/sandbox.rs` — `resolve_existing_write` validates target file
- Test pattern: `tests/tool_contract.rs` — extend with edit-specific test cases

## Technical Approach

### edit_file flow

1. Validate path exists and is writable: `sandbox.resolve_existing_write(path)`
2. Verify path is a file (not a directory)
3. Read file content as string
4. Verify file size ≤ `maxEditBytes`
5. For each edit in the `edits` array:
   - Find occurrences of `oldText` in file content
   - If 0 occurrences: return `edit_pattern_not_found` error with the oldText
   - If >1 occurrences and `replaceAll` is false: return `edit_pattern_ambiguous` error with count
   - If 1 occurrence or `replaceAll` is true: replace all occurrences with `newText`
6. Generate unified diff: `similar::TextDiff::from_lines(original, modified)`
7. If `dryRun: true` (default): return diff without writing
8. If `dryRun: false`: write modified content to file, return diff

### Diff generation

Use `similar` crate:

```rust
use similar::{ChangeTag, TextDiff};

fn generate_diff(original: &str, modified: &str) -> String {
    let diff = TextDiff::from_lines(original, modified);
    diff.unified_diff()
        .context_radius(3)
        .to_string()
}
```

### Line ending normalization

Before matching, normalize `\r\n` → `\n` in both file content and edit strings. Write back with original line endings if unchanged, or with `\n` if modified.

### Edit object shape

```rust
struct Edit {
    old_text: String,
    new_text: String,
    replace_all: bool, // default false
}
```

### Dry run default

When `dryRun` is absent from the request, treat as `true`. Client must explicitly pass `dryRun: false` to apply edits.

## File Changes

### New Files

| File | Purpose |
|---|---|
| `src/tools/edit_file.rs` | Edit tool implementation |
| `tests/edit_file.rs` | Edit-specific tests |

### Modified Files

| File | Change |
|---|---|
| `src/tools/mod.rs` | Add edit_file module |
| `src/server.rs` | Register edit_file tool |
| `src/config.rs` | Add `max_edit_bytes` to Limits (default 10 MiB) |
| `src/error.rs` | Add `EditPatternNotFound` and `EditPatternAmbiguous` variants |

## Implementation Steps

1. Add `max_edit_bytes` to config Limits
2. Add `EditPatternNotFound` and `EditPatternAmbiguous` to FsError
3. Implement `edit_file` tool with pattern matching
4. Implement unified diff generation with `similar`
5. Implement line ending normalization
6. Handle `dryRun` default and behavior
7. Add binary file rejection (reuse Phase 1 binary detection)
8. Update `src/tools/mod.rs` and `src/server.rs`
9. Write `tests/edit_file.rs`

## Data / API / Interface Contract

### edit_file input

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

### edit_file output (dry run)

```json
{
  "path": "/Volumes/Data/src/main.rs",
  "changed": true,
  "diff": "--- original\n+++ modified\n@@ -1,3 +1,3 @@\n-println!(\"hello\");\n+println!(\"hello world\");"
}
```

### edit_file output (applied)

Same shape, `changed: true`, file is written.

### edit_file output (no change / pattern not found)

```json
{
  "path": "/Volumes/Data/src/main.rs",
  "changed": false,
  "diff": null,
  "error": {
    "code": "edit_pattern_not_found",
    "message": "Edit pattern not found in file",
    "pattern": "println!(\"hello\");"
  }
}
```

## Error Handling

| Scenario | Error Code |
|---|---|
| Path outside roots | `outside_allowed_roots` |
| Read-only root | `read_only_root` |
| File not found | `path_not_found` |
| Not a file | `not_a_file` |
| File exceeds maxEditBytes | `file_too_large` |
| Binary file | `binary_file_not_supported` |
| Pattern not found | `edit_pattern_not_found` |
| Multiple occurrences, replaceAll false | `edit_pattern_ambiguous` (with count) |

## Observability

- Log at `info`: edit operations (path, number of edits, dryRun/applied); never log content
- Log at `warn`: rejected edits (pattern not found, read-only root)

## Testing Requirements

### Unit Tests

- Single replacement with exact match
- Multiple occurrence ambiguity detection
- `replaceAll: true` replaces all occurrences
- Pattern not found error
- Diff generation with additions and deletions
- Line ending normalization (CRLF → LF matching)

### Integration Tests

- `tests/edit_file.rs`: dry run preserves file; apply writes file; diff output is valid
- Edit on readOnly root is rejected
- Edit with binary file is rejected

### Regression Tests

None.

## Validation Commands

```bash
cargo test --test edit_file
cargo test
```

## Acceptance Criteria

- [ ] `edit_file` with `dryRun: true` returns diff but does not modify file
- [ ] `edit_file` with `dryRun: false` modifies file and returns diff
- [ ] `replaceAll: true` replaces all occurrences of oldText
- [ ] Multiple occurrences without `replaceAll` return ambiguity error
- [ ] Pattern not found returns `edit_pattern_not_found` error
- [ ] Diff output is valid unified diff format
- [ ] Edit on readOnly root is rejected
- [ ] Edit on binary file is rejected
- [ ] Multiple edits applied sequentially within single call
- [ ] Default `dryRun` is true when flag is omitted

## Risks

| Risk | Severity | Mitigation |
|---|---|---|
| Pattern matching O(n*m) on large files with many edits | Medium | Bound by maxEditBytes (10 MiB); document performance characteristic |
| Similar crate diff output differs from GNU diff | Low | Accept similar output; test that output is parseable and human-readable |
| Multi-edit order dependency (edit 2 depends on edit 1 result) | Medium | Apply edits sequentially; document that order is significant |
