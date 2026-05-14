# AgentBoard Desktop

AgentBoard 的 Electron + React 桌面客户端。

这个客户端 Shell 是 AgentBoard 的 Electron + React 桌面工作台，接入本地 TaskLoop 运行时。产品边界如下：

- AgentBoard 是产品名，也是用户看到的桌面工作台。
- TaskLoop 是 Rust 内核，负责 policy、memory、context 和 task state。
- `crates/taskloop-sidecar` 通过 stdin/stdout JSON-RPC 把 TaskLoop 暴露给 Electron。
- Electron 客户端负责窗口、渲染层状态、本地集成、Provider、MCP 和桌面 UI。

## 开发

从仓库根目录运行：

```bash
npm run sidecar:build
npm run dev
```

或进入当前目录：

```bash
npm install
npm run dev
```

## 验证

```bash
npm run typecheck
npm run build
```

Rust 运行时检查：

```bash
cd ../..
cargo test -p taskloop-core -p taskloop-sidecar
```

## 运行边界

当前应用的运行边界：

- TaskLoop 提供持久化的 policy、context、memory、task projection 边界。
- Electron 侧 agent runner 承接桌面交互、模型调用和工具编排，并逐步收紧到清晰的 TaskLoop 边界。
- 对外产品命名统一使用 AgentBoard。内部历史 key 如果会影响本地设置迁移或拖拽协议，先保留，后续单独做迁移。
