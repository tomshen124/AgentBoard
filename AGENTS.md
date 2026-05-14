---
kind: agents
version: 1
scope: workspace
updated_at: 2026-03-25
source_of_truth: markdown_contract
visibility: internal
writable_roots:
  - .
backup_before_write: true
destructive_requires_approval: true
exec_requires_approval: false
require_task_heartbeat: true
heartbeat_interval_ms: 30000
allowed_exec_languages:
  - python
  - node
  - bash
---

# AgentBoard Workspace Rules

- `AgentBoard` 是产品名，`TaskLoop` 是内核名。
- 内部执行主体始终是 `TaskLoop task`，外部宿主对象不直接等于内部 task。
- 默认在当前 workspace 内读写；跨目录写入、外部网络发布、 destructive 操作需要明确授权。
- 普通构建、类型检查、测试和只读检索可以直接执行；高危覆盖、删除、批量破坏性操作必须先确认。
- 写文档和用户材料时优先保留旧文件副本；代码变更应保持局部、可验证。
- 长任务必须持续反馈，禁止 silent running。
- 对外展示时使用 `AgentBoard`、`Agent`、`Automation`、`Connection` 等产品语言；只有内部运行时、技术文档和协议层可使用 `TaskLoop`。
- 项目级记忆、工具和偏好应优先维护在 `.agents/` 下的 Markdown 契约文件中；根目录同名文件只作为兼容入口。
