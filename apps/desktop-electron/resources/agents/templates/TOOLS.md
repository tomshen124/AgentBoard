# TOOLS.md - Tool Policy

This file describes which tools are expected in this workspace and how they should be used.

## Local Tools

- Shell commands, scripts, tests, and builds may run locally within this workspace.
- Prefer read-only inspection before editing.
- Keep generated artifacts out of source control unless they are intentionally part of the project.

## Approval

- Ask before destructive operations, external publishing, credential changes, or remote execution.
- Explain high-risk actions before requesting approval.

## Remote Reserved

- Remote runners, SSH, MCP proxies, and team runtimes should use a traceable permission request.
- Remote tools should include request id, source, tool name, input summary, and risk level.
- Remote capabilities are disabled until a connection is configured and approved.

## Notes

- Add project-specific commands, MCP servers, and tool constraints here.
