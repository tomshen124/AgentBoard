# AgentBoard Redesign Decision Brief

Status: proposal summary
Date: 2026-05-13

## Core Decision

AgentBoard should keep the current feature set but change the visible product structure.

The app should expose AgentBoard's own workbench model:

```text
Threads + Projects + Studio + Skills + Connections + Automations
```

`TaskLoop` stays as the kernel name, not a top-level navigation name.

## Recommended Top-Level IA

| Top-Level Area | Chinese | What It Contains |
|---|---|---|
| Threads | 会话 | Main conversation and execution flow |
| Projects | 项目 | Working folders, project memory, project threads |
| Studio | 工作室 / 能力库 | Agents, Commands, Skills, Canvas |
| Connections | 连接 | MCP, Web Search, Browser, vendor services, Skill Sources config |
| Automations | 自动化 | Cron, scheduled jobs, run history |
| Settings | 设置 | Workspace, Models, Capabilities, Automation, Memory, Data, About |

## Rename Decisions

| Current Visible Name | Proposed Visible Name | Keep Internally? |
|---|---|---|
| TaskLoop | Automations / 自动化 | Yes, as kernel |
| Skill Market | Skill Sources / Skill 来源 | No official market assumption |
| Project Archive | Project Contracts / 项目契约 | Route can remain compatible |
| Agent Studio | Studio / 工作室 | Yes as umbrella |
| Connectors | Connections / 连接 | MCP remains a type |

## Settings IA

Recommended settings groups:

1. Workspace
2. Models
3. Capabilities
4. Automation
5. Memory
6. Data
7. About

Avoid visible settings groups named:

- Plugin
- Channel
- Migration
- TaskLoop
- Marketplace Provider

## First Implementation Round

Do not touch runtime logic.

1. Rename visible TaskLoop nav/page labels to Automations.
2. Rename Project Archive to Project Contracts.
3. Rename Skill Market to Skill Sources.
4. Reorder settings menu into the new groups.
5. Rewrite empty states and page subtitles.
6. Keep existing routes, stores, IPC, and database names unchanged.

## Design Rule

User-facing names describe what the user is doing.

Internal names describe how the system works.

Examples:

- User sees Automations. Code can still use cron.
- User sees Project Contracts. Routes can still use existing archive identifiers for compatibility.
- User sees Connections. Implementation can still use MCP.
- User sees AgentBoard. Runtime can still use TaskLoop.
