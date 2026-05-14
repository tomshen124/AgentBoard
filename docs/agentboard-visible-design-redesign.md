# AgentBoard Visible Product Redesign

Status: proposal
Scope: visible product structure, naming, settings IA, and first-pass screen design
Date: 2026-05-13

## 1. Goal

AgentBoard should feel like its own desktop workbench, not a rebadged shell.

The goal of this redesign is not to remove capabilities. The goal is to keep the useful functions while changing the visible product language:

- AgentBoard is the product and desktop workbench.
- TaskLoop is the execution kernel, not a top-level navigation item.
- Cron is a scheduling mechanism, not the product name for the automation page.
- Skills, Agents, Commands, Connectors, Models, and Automations should feel like parts of one workbench rather than copied modules.

This proposal is intentionally small-design first. It defines naming, layout, and page grouping before implementation.

## 2. Current Problem

The current app has enough working functionality, but the visible structure still has inherited shape:

- Top-level modules look like the source project module list.
- Settings groups expose system categories instead of AgentBoard product concepts.
- "TaskLoop" appears as a user-facing navigation item even though it is actually the internal execution loop.
- "Project" and "TaskLoop" overlap in user understanding because both imply a place where work happens.
- "Agent Studio", "Skills", "Canvas", "Connectors", and "TaskLoop" are useful capabilities, but their hierarchy is unclear.

The product should not ask users to understand the implementation architecture first.

## 3. Product Model

AgentBoard should be explained through six visible objects:

| Visible Object | User Meaning | Internal Mapping |
|---|---|---|
| Threads | Conversation and execution history | sessions, messages, run blocks |
| Projects | Local work context and memory boundary | projects, working folder, project archive |
| Studio | Place to design reusable agent behavior | agents, commands, role markdown |
| Skills | Packaged task abilities | local SKILL.md, external source import |
| Connections | External tools and services | MCP, web search, browser, vendor connectors |
| Automations | Scheduled or repeating agent work | Cron jobs, cron runs, scheduled TaskLoop tasks |

`TaskLoop` remains present in docs and advanced places, but not as the primary object in the left nav.

## 4. Recommended Navigation

### 4.1 Left Sidebar

Recommended primary navigation:

1. New Thread
2. Search
3. Projects
4. Studio
5. Connections
6. Automations

Recommended lower navigation:

1. Help
2. Settings
3. Version

Projects should remain a major visible area because this is where local work context lives. Automations should not sit between "Agent Studio" and "Projects" as if it were another work mode; it is a scheduled execution surface.

### 4.2 Studio Structure

Studio should group "things the user can create to shape agent behavior":

- Agents
- Commands
- Skills
- Canvas

This avoids exposing too many top-level modules while still keeping the features discoverable.

### 4.3 Connections Structure

Connections should group "things that connect AgentBoard to external capabilities":

- MCP Servers
- Web Search
- Browser
- Provider OAuth or channel auth
- Vendor integrations
- Skill Sources, only when treated as remote source configuration

Credentials belong here or in model/provider pages, not scattered across feature pages.

## 5. Proposed Layout

### 5.1 Shell Wireframe

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ AgentBoard                                      Project: New Project         │
├───────────────┬──────────────────────────────────────────────────────────────┤
│ New Thread    │                                                              │
│ Search        │  THREAD / PROJECT HOME                                       │
│               │                                                              │
│ Projects      │  ┌────────────────────────────────────────────────────────┐  │
│   New Project │  │ What do you want AgentBoard to work on?                │  │
│   Local path  │  │ [ Composer ]                                           │  │
│               │  └────────────────────────────────────────────────────────┘  │
│ Studio        │                                                              │
│   Agents      │  Recent threads, active runs, project memory, outputs         │
│   Commands    │                                                              │
│   Skills      │                                                              │
│   Canvas      │                                                              │
│               │                                                              │
│ Connections   │                                                              │
│ Automations   │                                                              │
├───────────────┤                                                              │
│ Help Settings │                                                              │
└───────────────┴──────────────────────────────────────────────────────────────┘
```

### 5.2 Project Home Wireframe

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ Project: New Project                                                        │
├──────────────────────────────┬───────────────────────────────────────────────┤
│ Start                         │ Context                                       │
│ ┌──────────────────────────┐ │ ┌───────────────────────────────────────────┐ │
│ │ Ask AgentBoard...        │ │ │ Working folder                            │ │
│ │                          │ │ │ Project memory                            │ │
│ │ [Prompt composer]        │ │ │ Linked skills                             │ │
│ └──────────────────────────┘ │ │ Recent files                              │ │
│                              │ └───────────────────────────────────────────┘ │
│ Recent Threads               │                                               │
│ - Fix release packaging      │ Active Work                                   │
│ - Review settings IA         │ - Running / paused / needs approval           │
│ - Draft project docs         │                                               │
└──────────────────────────────┴───────────────────────────────────────────────┘
```

Design intent:

- Project is a context container, not a dashboard.
- The composer remains central.
- Advanced context is visible but secondary.

### 5.3 Automations Wireframe

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ Automations                                                                 │
│ Scheduled and repeating agent work                                           │
├──────────────────────────────┬───────────────────────────────────────────────┤
│ Automation Calendar           │ Selected Automation                           │
│ ┌──────────────────────────┐ │ ┌───────────────────────────────────────────┐ │
│ │ Month / week / today     │ │ │ Name                                      │ │
│ │ Run counts by day        │ │ │ Schedule: every day 09:00                 │ │
│ │ Planned + completed      │ │ │ Source thread                             │ │
│ └──────────────────────────┘ │ │ Prompt / task                              │ │
│                              │ │ Recent runs                                │ │
│ Today                        │ │ Replay transcript                          │ │
│ - Morning project review     │ └───────────────────────────────────────────┘ │
│ - Weekly cleanup             │                                               │
└──────────────────────────────┴───────────────────────────────────────────────┘
```

Naming:

- Page title: Automations / 自动化
- Card title: Automation Calendar / 自动化日历
- Schedule type: One-time, Interval, Cron Expression
- Internal tool names can stay CronAdd, CronUpdate, CronRemove.

### 5.4 Studio Wireframe

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ Studio                                                                       │
│ Build reusable agent behavior                                                │
├──────────────────────────────────────────────────────────────────────────────┤
│ Agents        Commands        Skills        Canvas                           │
├──────────────────────────────────────────────────────────────────────────────┤
│ Agents                                                                      │
│ ┌──────────────────────┐ ┌──────────────────────┐ ┌──────────────────────┐ │
│ │ Code Reviewer        │ │ Project Planner      │ │ Document Drafter     │ │
│ │ Role + constraints   │ │ Planning prompts     │ │ Office workflows     │ │
│ └──────────────────────┘ └──────────────────────┘ └──────────────────────┘ │
│                                                                              │
│ Selected item detail: markdown, variables, linked skills, test prompt         │
└──────────────────────────────────────────────────────────────────────────────┘
```

Design intent:

- Studio is not a random list of imported modules.
- Everything in Studio is something the user can shape.
- "Agents / Commands" can remain as a tab label, but the shell should call the whole area Studio.

### 5.5 Connections Wireframe

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ Connections                                                                  │
│ External tools, services, search, and sources                                 │
├──────────────────────────────────────────────────────────────────────────────┤
│ MCP Servers     Web Search     Browser     Skill Sources     Vendor Services │
├──────────────────────────────────────────────────────────────────────────────┤
│ Connection list                         Detail / test result                  │
│ - Local filesystem MCP                  - Status                              │
│ - Browser tools                         - Auth                                │
│ - Tencent service connector             - Available tools                     │
│ - SkillHub / ClawHub / GitHub source    - Risk notes                          │
└──────────────────────────────────────────────────────────────────────────────┘
```

Design intent:

- This is where external keys and external services belong.
- Skills page should install and review skills. Connections page should configure sources and service credentials.

## 6. Settings Redesign

Settings should be restructured by user intent, not by inherited implementation modules.

### 6.1 Recommended Settings Groups

| Group | Pages | Purpose |
|---|---|---|
| Workspace | General, Appearance, Project Defaults | Personal workbench behavior |
| Models | AI Providers, Model Management, Model Routing | Provider and model decisions |
| Capabilities | Skills, Connections, Web Search, Browser | What AgentBoard can use |
| Automation | Schedules, Run Limits, Notifications | Background and repeating work |
| Memory | Global Memory, Project Contract Policy | Identity, preference, and project context |
| Data | Analytics, Backup, Import, Clear Data | Local data visibility and management |
| About | Version, Links, License | Product information |

### 6.2 Settings Wireframe

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ Settings                                                                     │
├──────────────────────────────┬───────────────────────────────────────────────┤
│ Workspace                    │ Appearance                                    │
│   General                    │ Theme, font, language, default project path    │
│   Appearance                 │                                               │
│   Project Defaults           │                                               │
│                              │                                               │
│ Models                       │                                               │
│   Providers                  │                                               │
│   Model Routing              │                                               │
│                              │                                               │
│ Capabilities                 │                                               │
│   Connections                │                                               │
│   Skills                     │                                               │
│   Web Search                 │                                               │
│   Browser                    │                                               │
│                              │                                               │
│ Automation                   │                                               │
│ Memory                       │                                               │
│ Data                         │                                               │
│ About                        │                                               │
└──────────────────────────────┴───────────────────────────────────────────────┘
```

### 6.3 What To Remove From Visible Settings

Do not expose these as first-class settings names:

- Plugin
- Channel
- Migration
- TaskLoop
- Marketplace provider

Keep the capabilities where needed, but express them as:

- Connections
- Import
- Compatibility
- Automations
- Skill Sources

## 7. Naming System

### 7.1 Keep

| Name | Keep Where |
|---|---|
| AgentBoard | Product, window title, docs |
| TaskLoop | Technical docs, logs, internal execution explanation |
| Skill | User-facing capability package |
| Agent | User-created role/behavior |
| Command | Slash command and reusable prompt action |
| Connector | External tool/service |

### 7.2 Change

| Current | Proposed | Reason |
|---|---|---|
| TaskLoop top nav | Automations | Current page is scheduled work, not the execution kernel |
| Agent Studio section list | Studio | Cleaner umbrella for agents, commands, skills, canvas |
| Skills Market | Skill Sources | No official AgentBoard market yet |
| Connectors and Skills group | Capabilities | User thinks in capabilities, not integration categories |
| Project Archive | Project Contracts | More direct user value |
| Canvas | Canvas / Lab | If it stays experimental, "Lab" can clarify status |

## 8. Visual Direction

AgentBoard should feel like a calm local workbench:

- restrained paper-like surfaces
- compact but not cramped
- sidebar for orientation, not feature advertising
- cards only for real objects, not every section
- stronger hierarchy around the composer and current work
- fewer decorative labels and fewer inherited module names

Avoid:

- big dashboard grids on first screen
- copied settings group names
- overexposed internal runtime names
- equal-weight module lists
- blank pages that only say "select something"

## 9. Minimal Redesign Iterations

### Iteration 1: Visible IA Cleanup

No logic change.

- Rename top-level TaskLoop to Automations.
- Rename Skill Market to Skill Sources.
- Group Agents, Commands, Skills, Canvas under Studio.
- Group MCP, Web Search, Browser, Skill Sources under Connections or Capabilities.
- Remove visible references to plugin/channel/migration unless inside compatibility import.

### Iteration 2: Settings Reframe

No runtime change.

- Rebuild settings left menu into Workspace, Models, Capabilities, Automation, Memory, Data, About.
- Move backup/import/clear data into Data.
- Move Skill Sources near Connections.
- Keep model provider pages but adjust names and descriptions.

### Iteration 3: Project Home Redesign

Low logic change.

- Make project home composer-first.
- Add project context panel.
- Add active work and recent output summary.
- Rename Project Archive to Project Contracts.

### Iteration 4: Studio Redesign

Moderate UI change.

- Make Studio tabbed: Agents, Commands, Skills, Canvas.
- Add selected item detail area.
- Add "test this agent/command/skill" affordance.

### Iteration 5: Automation Page Polish

Moderate UI change.

- Keep current cron logic.
- Rename labels and empty states.
- Add list/week toggle later if needed.
- Present run transcript as execution replay, not generic task detail.

## 10. Recommended First Decision

Adopt this rule:

> User-facing navigation names describe what the user is doing. Internal names describe how the system works.

That means:

- Automations is user-facing.
- TaskLoop is internal.
- Connections is user-facing.
- MCP is a connection type.
- Skill Sources is user-facing.
- ClawHub, SkillHub, and GitHub are source adapters.

## 11. Open Questions

1. Should "Studio" be named "Studio", "Workbench", or "能力库" in Chinese?
2. Should "Project Archive" become "Project Contracts" or "Project Context"?
3. Should Canvas remain top-level under Studio, or move to a separate "Lab" later?
4. Should Automations be visible by default, or only after the user creates the first scheduled task?
5. Should Connections own Skill Sources, or should Skills own Skill Sources?

My recommendation:

- Chinese: 工作室 or 能力库. Use "工作室" if product tone is friendly; use "能力库" if product tone should be more utilitarian.
- Project Archive: rename to 项目契约.
- Canvas: keep under Studio for now.
- Automations: visible by default because scheduled work is a real differentiator.
- Skill Sources: visible under Skills page, configurable under Connections/Capabilities settings.
