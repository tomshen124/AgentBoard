# AgentBoard Desktop Guidelines

This directory contains the Electron client shell for AgentBoard.

## Boundaries

- Use `AgentBoard` for product-facing names.
- Use `TaskLoop` for the local Rust runtime/kernel.
- Keep system access in the Electron main process.
- Keep renderer code focused on UI state, presentation, and user interaction.
- Route durable policy, memory, context, and task-state behavior through the TaskLoop boundary as integration matures.

## Structure

- `src/main/` contains Electron main-process code, IPC, local integrations, providers, MCP, SSH, update, and sidecar wiring.
- `src/preload/` exposes the secure renderer bridge.
- `src/renderer/src/` contains the React UI, stores, hooks, i18n, tools, and assets.
- `src/shared/` contains cross-process TypeScript contracts.
- `resources/` contains bundled runtime assets loaded by the desktop app.

## Commands

- `npm run dev` starts Electron + Vite for local development.
- `npm run typecheck` validates main/preload and renderer TypeScript.
- `npm run build` typechecks and builds the Electron bundles.
- From the repository root, `cargo test -p taskloop-core -p taskloop-sidecar` verifies the Rust runtime and sidecar.

Do not edit generated outputs in `out/`, `dist/`, `node_modules/`, or tsbuildinfo files.
