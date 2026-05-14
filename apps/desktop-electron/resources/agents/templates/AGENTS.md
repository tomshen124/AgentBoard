# AGENTS.md - AgentBoard Workspace Protocol

This file defines how AgentBoard agents should work inside this project.

## Product Language

- Use `AgentBoard` for product-facing names.
- Use `Agent`, `Connection`, `Skill`, `Command`, and `Automation` for visible capabilities.
- Use `TaskLoop` only for internal runtime, protocol, or technical documentation.

## Session Startup

1. Read this file first for workspace protocol.
2. Read `PROFILE.md` for collaboration preferences if it exists.
3. Read `FOCUS.md` for the current project focus if it exists.
4. Read `MEMORY.md` and recent `memory/YYYY-MM-DD.md` notes only when long-term or recent context is needed.

## Work Boundaries

- Keep edits inside this workspace unless the user explicitly authorizes another path.
- Ask before deleting, overwriting, publishing externally, changing credentials, or running remote actions.
- Prefer small, verifiable changes with focused tests.
- Preserve user edits and do not revert unrelated work.

## Project Contracts

- `AGENTS.md`: workspace protocol and safety boundaries.
- `TOOLS.md`: local, remote, MCP, and approval policy.
- `MEMORY.md`: durable project decisions and context.
- `PROFILE.md`: user/project collaboration preferences.
- `FOCUS.md`: current phase, release goals, and active priorities.

## Memory Rules

- Store durable decisions, preferences, and long-lived context.
- Do not store secrets, API keys, credentials, or sensitive personal data.
- Do not store repository facts that can be derived by reading the current workspace.
- Put short-lived notes in `memory/YYYY-MM-DD.md` and distill them later.
