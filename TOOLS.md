---
kind: tools
version: 1
scope: workspace
updated_at: 2026-05-14
source_of_truth: markdown_contract
visibility: internal
---

# AgentBoard Tool Policy

## Local Tools

- `exec` / `shell` 是一等工具能力，用于构建、测试、脚本化处理和本地诊断。
- `python`、`node`、`bash` 可以作为工具层脚本语言使用，但不代表项目架构必须绑定这些语言。
- 文件读写、命令执行、MCP、浏览器、终端和技能调用都必须经过 policy / approval / sandbox 边界。

## Approval

- 只读检索、类型检查、测试和普通构建默认可执行。
- 写文件应限定在 workspace 或用户明确授权目录内。
- 删除、覆盖、批量迁移、外部发布、凭据修改和远程执行默认需要确认。

## Remote Reserved

- 远程 runner、MCP proxy、SSH runner 和 team runtime 应统一走 `agentboard.remote-tool.permission.v1` 审批协议。
- 远程工具请求必须带上来源、工具名、输入摘要、风险级别和可追踪的 request id。
- 远程能力默认关闭；只有用户配置连接、授权 scope 并通过审批后才能执行。

## Provider Headers

- 第三方 provider 默认走严格 header 策略，不默认带额外系统请求头。
- API Key、token 和私密 endpoint 不写入 Markdown 契约文件。
