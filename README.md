# AgentBoard

Personal desktop AI agent for work. Covers daily office work and software development in one product.

## Architecture

```
apps/desktop-electron/    Electron + React 19 + Tailwind (client shell)
crates/taskloop-core/     Rust policy/memory/context engine (the brain)
crates/taskloop-sidecar/  JSON-RPC bridge (stdin/stdout, connects the two)
```

## Client Direction

AgentBoard now uses the Electron client line as the only desktop shell. The old
desktop prototype has been removed instead of archived.

AgentBoard uses an Electron shell with its own runtime boundary:

- `AgentBoard` is the product and desktop workbench.
- `TaskLoop` is the kernel for policy, context, memory, and task state.
- Electron owns UI, windows, providers, MCP, local integrations, and desktop
  affordances.
- The client keeps Electron-side integrations for now while agent execution is
  progressively pulled behind the TaskLoop sidecar.

## Workspace Contracts

Workspace behavior is controlled by markdown contracts in the project root:

| File | Purpose |
|------|---------|
| `AGENTS.md` | Operating rules, execution policy, allowed tools |
| `PROFILE.md` | User profile, preferences, knowledge |
| `FOCUS.md` | Current focus and priorities |
| `MEMORY.md` | Long-term stable project facts |
| `TOOLS.md` | Tool constraints and environment notes |

## Quick Start

```bash
# Build the Rust sidecar
cargo build -p taskloop-sidecar

# Start the Electron app in dev mode
cd apps/desktop-electron
npm run dev
```

## Provider Configuration

Configure AI providers in `config/providers.toml`:

```toml
[anthropic]
profile = "anthropic"
type = "anthropic"
base_url = "https://api.anthropic.com"
```

Direct API connection — no relay, no middle-tier.
