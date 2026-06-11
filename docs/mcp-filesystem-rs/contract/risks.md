# Risks

## Risk register

| # | Risk | Severity | Likelihood | Mitigation | Owner |
|---|---|---|---|---|---|
| R1 | `std::fs::canonicalize` fails on paths that don't exist yet, complicating write-path validation | High | Near-certain | Use `resolve_for_create` approach: canonicalize nearest existing parent, validate that, then construct child path | Sandbox module |
| R2 | Symlink TOCTOU race: symlink created/changed between validation and operation | High | Possible | Accept as inherent limitation of filesystem APIs; document in security section of README | Documentation |
| R3 | `rmcp` crate has protocol incompatibilities with specific MCP clients (Claude Desktop vs Claude Code) | Medium | Possible | Test against both clients before release; pin `rmcp` version; vendor if necessary | Integration testing |
| R4 | Large directory trees cause timeout on `directory_tree` or `search_files` | Medium | Likely | Enforce `maxTreeDepth`, `maxDirectoryEntries`, `maxSearchResults`; use `tokio::spawn_blocking` with timeout wrappers | Tool implementations |
| R5 | Binary detection for `read_text_file` fails on files without extensions (e.g., Mach-O, ELF) | Medium | Possible | Use magic byte detection as fallback; document that extensionless binary files may be misidentified | `read_text_file` tool |
| R6 | User configures overlapping roots with contradictory modes and expects different precedence | Low | Possible | Most-specific-wins is deterministic and documented; include example in README | Documentation |
| R7 | `edit_file` pattern matching fails on files with mixed line endings (CRLF vs LF) | Medium | Possible | Normalize line endings before matching; document behavior | `edit_file` tool |
| R8 | macOS `/tmp` is a symlink to `/private/tmp`, causing canonical path mismatches | Medium | Likely on macOS | Test canonicalization with macOS path quirks; ensure roots resolve through symlinks at startup | Sandbox module |

## Watch list

- **`rmcp` API stability** — The crate may change its API before 1.0. Monitor for breaking changes.
- **MCP protocol spec evolution** — The MCP spec may add new transport requirements or change tool registration. Track the spec repository.
- **Large file performance** — If users regularly hit `maxReadBytes`, may need streaming/chunking support.

## Kill switches

- If `rmcp` is found to have a security vulnerability, fork or switch to raw JSON-RPC.
- If canonicalization proves unreliable on a target platform, add platform-specific test gates in CI and document limitations.
- If a path escape is found in the wild, issue a patch release within 24 hours and add the specific scenario to the test suite.
