---
kind: memory
version: 1
scope: workspace
updated_at: 2026-05-14
source_of_truth: structured_projection
visibility: internal
---

# Stable Workspace Facts

- AgentBoard 当前采用 Electron + React 客户端，TaskLoop 作为内部运行时/内核。
- `AgentBoard` 是产品名；`TaskLoop` 只用于内部运行时、协议、技术文档和内核测试。
- 工作区契约文件用于指导 Agent 行为、工具边界、长期记忆、用户偏好和阶段焦点。
- Markdown 文件是人工可维护入口和运行提示来源，不是唯一数据库；可由客户端读取、展示和初始化。

## Durable Decisions

- Skill、Command、Agent、Connection、Automation 是产品层能力；底层可以映射到 skills、hooks、MCP servers、remote tool permissions。
- 远程执行能力先预留协议和 UI 入口，不默认开启。
- 长期记忆不保存可从当前代码仓库直接推导出的文件结构、路径和实现事实。
