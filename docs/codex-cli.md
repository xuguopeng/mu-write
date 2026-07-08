# Codex CLI 调用说明

爆文工坊移除了内置 AI 聊天面板，但保留章节生成、润色、知识库、MCP、模型配置等核心能力。Codex 或脚本可以通过 `plotforge-cli` 调用这些功能。

## 构建

```bash
pnpm run cli:build
```

开发期也可以直接运行：

```bash
cargo run --manifest-path src-tauri/Cargo.toml --bin plotforge-cli -- list
```

## 基本格式

```bash
plotforge-cli call <channel> '[json-array-args]'
```

如果功能依赖当前项目，先用 `--project` 打开项目：

```bash
plotforge-cli call --project /path/to/novel <channel> '[json-array-args]'
```

`json-array-args` 与前端 `ipc.invoke(channel, ...args)` 的参数顺序一致。

注意：`config:get`、`llm:list-models` 等通道会返回本地配置，可能包含 API Key 或第三方服务密钥。不要把完整输出粘贴到公开场合。

## 常用示例

读取全局配置：

```bash
plotforge-cli call config:get
```

查看最近项目：

```bash
plotforge-cli call project:recent-list
```

打开项目并读取核心设定：

```bash
plotforge-cli call --project /path/to/novel db:project-core-get
```

搜索知识库：

```bash
plotforge-cli call --project /path/to/novel kb:search '["主角 金手指", 5]'
```

列出模型：

```bash
plotforge-cli call llm:list-models
```

非流式调用模型：

```bash
plotforge-cli call llm:generate '[{"modelId":"","messages":[{"role":"user","content":"写一句测试文本"}]}]'
```

## 深链格式

CLI 支持把 `plotforge://` 链接解析成同样的调用：

```bash
plotforge-cli deeplink 'plotforge://call?channel=config:get'
```

带参数：

```bash
plotforge-cli deeplink 'plotforge://call?channel=kb:search&project=/path/to/novel&args=%5B%22%E4%B8%BB%E8%A7%92%22%2C5%5D'
```

深链字段：

| 字段 | 说明 |
|---|---|
| `channel` | 必填，现有 IPC 通道名，如 `db:project-core-get` |
| `args` | 可选，URL 编码后的 JSON 数组 |
| `project` | 可选，项目目录；传入后先执行 `project:open` |

## 可调用范围

CLI 支持：

- `config:*`
- `project:create/open/save/update-config/recent-list`
- `fs:*`
- `db:*`
- `kb:*`
- `llm:list-models`
- `llm:generate`
- `llm:test-connection`
- `mcp:*`

CLI 不支持需要 GUI 的通道：

- `dialog:*`
- `import:*`
- `llm:generate-stream`

这些功能仍需要桌面应用窗口或前端事件流。
