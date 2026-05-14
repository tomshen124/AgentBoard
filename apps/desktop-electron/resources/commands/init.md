Initialize this workspace for AgentBoard by creating or updating the project contract files under `.agents/`.

Create these files when they are missing. If root-level legacy files already exist, preserve their useful content while moving the structure toward `.agents/`.

Required files:

- `.agents/AGENTS.md`: workspace protocol, product language, safety boundaries, and collaboration rules.
- `.agents/TOOLS.md`: local tools, remote reserved capabilities, MCP/SSH/browser policy, and approval rules.
- `.agents/MEMORY.md`: durable project decisions and long-lived context.
- `.agents/PROFILE.md`: project-specific collaboration preferences and product taste.
- `.agents/FOCUS.md`: current goal, near-term tasks, deferred scope, and constraints.

Document requirements:

- Keep each file concise and specific to this repository.
- Do not store secrets, API keys, credentials, or sensitive personal data.
- Do not store code structure, file paths, or repository facts that can be derived by reading the workspace unless they are durable decisions.
- Use `AgentBoard` for product-facing names and `TaskLoop` only for internal runtime/kernel references.
- Ask before destructive operations, external publishing, credential changes, or remote execution.

After writing the files, briefly summarize what was created and mention any legacy root-level files that were left in place for compatibility.
