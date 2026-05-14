# AgentBoard Desktop

Electron + React desktop client for AgentBoard.

This client shell is AgentBoard's Electron + React desktop workbench, integrated with the local TaskLoop runtime. The product direction is:

- AgentBoard is the desktop product and user-facing workbench.
- TaskLoop is the Rust core for policy, memory, context, and task state.
- `crates/taskloop-sidecar` exposes TaskLoop to the Electron app through stdin/stdout JSON-RPC.
- The Electron client owns windows, renderer state, local integrations, providers, MCP, and rich desktop UI.

## Development

From the repository root:

```bash
npm run sidecar:build
npm run dev
```

Or from this directory:

```bash
npm install
npm run dev
```

## Verification

```bash
npm run typecheck
npm run build
```

For Rust runtime checks:

```bash
cd ../..
cargo test -p taskloop-core -p taskloop-sidecar
```

## Runtime Boundary

Runtime boundaries:

- TaskLoop provides the durable policy/context/memory/task projection boundary.
- The Electron-side agent runner handles desktop interaction, model calls, and tool orchestration while the TaskLoop boundary is tightened.
- Product-facing naming should use AgentBoard. Internal legacy keys can remain temporarily when renaming them would break local persisted settings or drag/drop protocols.
