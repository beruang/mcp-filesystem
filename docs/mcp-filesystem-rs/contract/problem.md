# Problem

## Current pain

MCP-compatible clients (Claude Code, Claude Desktop, and other MCP hosts) need controlled filesystem access to perform software engineering tasks — reading source files, writing edits, searching directories, and inspecting file metadata. Existing solutions either:

1. Grant unrestricted filesystem access with no sandboxing, creating security risk.
2. Lack recursive root behavior — requiring explicit registration of every subdirectory.
3. Use string-based path comparison, vulnerable to prefix attacks (`/Volumes/Data` vs `/Volumes/Database`).
4. Do not handle symlink escape consistently, allowing sandbox breakout.
5. Lack a clear read/write permission model per root.

## Affected user

MCP client users (developers, AI coding agents) who want to grant limited, auditable filesystem access to a specific directory tree without risking exposure of the broader filesystem.

## Current workaround

Users either accept full filesystem access (insecure), manually register each subdirectory (unscalable), or avoid filesystem MCP tools entirely (limiting agent capability).

## Impact

- **Security risk:** Unrestricted or poorly sandboxed filesystem access exposes SSH keys, environment files, and system configuration.
- **Friction:** Manual subdirectory registration is tedious and error-prone.
- **Capability gap:** Without a trustworthy filesystem MCP server, AI coding agents cannot reliably read, write, or search project files.

## Why now

The MCP ecosystem is growing rapidly. A well-tested, security-first Rust implementation sets the standard for filesystem MCP servers and gives MCP hosts a trustworthy option for filesystem access.
