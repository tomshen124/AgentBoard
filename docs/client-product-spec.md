# AgentBoard Client Product Spec

## 1. Product Definition

`AgentBoard` is a desktop personal AI agent client for work.

It serves two core usage surfaces inside one product:

- daily work
  documents, reports, todos, approvals, schedules, messages, research, organization
- software development
  code, terminal, repositories, debugging, review, code analysis, automation

These are not two products and not two shells.

The user should always feel:

- I am working inside one workspace or project
- I am collaborating with one agent through a thread
- the agent can handle both office work and development work

`TaskLoop` remains the only execution kernel.

## 2. What The Client Is Not

The desktop client must not position itself primarily as:

- a runtime console
- a task dashboard
- a connector control plane
- a provider management homepage
- a settings-first app

Management exists, but it is not the product story.

## 3. Primary User Value

The product must optimize for five things:

- easy to start
- easy to understand
- agent can actually do work
- output is easy to consume
- configuration is available but lightweight

The user comes here to complete work, not to operate infrastructure.

## 4. Main Product Objects

The client-facing primary objects are:

- workspace / project
- thread
- composer
- result / artifact
- review / confirmation

The following are internal execution objects and must not define the homepage:

- task
- approval center
- connector center
- subagent
- runtime metrics

## 5. Core Interaction Flow

The default user path is:

1. Open the app
2. Land inside a workspace or project
3. Start a new thread or resume one
4. Describe the work in natural language
5. Watch the agent execute inside the same thread
6. Consume outputs, files, previews, and summaries
7. Review approvals only when needed

There should be no requirement to first understand internal system concepts.

## 6. Homepage Information Architecture

### Left Sidebar

Must contain:

- current workspace or project
- new thread
- thread list
- workspace switch
- settings entry

Must not contain:

- task center navigation
- approval center navigation
- connector center navigation
- dashboard stats as the main story

### Main Column

Must be the primary visual focus.

It contains:

- empty-state welcome area or active thread
- user messages
- agent messages
- execution blocks
- result blocks
- the main composer

The first screen must make it obvious where to type.

### Right Column

Must act as a result layer.

It contains:

- output files
- current preview
- pending confirmations
- current attached context

It must not become a management rail.

## 7. Management Boundary

Management is necessary but secondary.

Keep it lightweight and easy:

- model and API endpoint configuration
- default model selection
- skill import and enablement
- workspace switch
- simple connector setup
- contract editing for advanced users

These belong in settings, drawers, or contextual entry points.

They do not define the homepage.

## 8. Design Direction

The shell should keep:

- a desktop workstation feel, friendly shell, and calm confidence
- visible execution, approvals, background and foreground task handling, continuation

It should not copy a developer console or a dashboard grid.

The visual tone should feel:

- personal
- competent
- work-focused
- warm but not cute
- powerful without exposing internal complexity first

## 9. Hard Rules

Any client implementation should fail review if any of these are false:

1. The user can immediately see where to start typing
2. The page still feels like a product, not a system console
3. The right rail is mostly results and review, not management
4. Office work and development work both fit naturally in the same thread model
5. Internal runtime concepts are supporting structure, not homepage structure

## 10. Implementation Order

The desktop shell should be rebuilt in this order:

1. empty state
2. thread view
3. outputs and preview
4. review and confirmations
5. contextual management entry points
6. deeper runtime mappings

Never reverse this order by starting from task, approval, or connector centers.
