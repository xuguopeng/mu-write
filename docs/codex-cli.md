# Codex CLI 调用说明

爆文工坊移除了内置 AI 聊天面板，但保留章节生成、润色、知识库、MCP、模型配置等核心能力。Codex 或脚本可以通过 `plotforge-cli` 调用这些功能。

给其他 AI 读取和执行的完整命令手册见 [`docs/AI_COMMANDS.md`](./AI_COMMANDS.md)。

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

### 短命令

查看最近项目：

```bash
plotforge-cli project recent
```

查看每个项目的蓝图、草稿、定稿进度：

```bash
plotforge-cli project status
```

创建项目：

```bash
plotforge-cli project new "测试小说" \
  --dir "/Users/xuguopeng/Documents/徐徐如声/徐徐如声/创作库/小说" \
  --genre "都市" \
  --audience "男频读者"
```

创建章节蓝图：

```bash
plotforge-cli chapter blueprint \
  --project "/path/to/novel" \
  --number 1 \
  --title "第一章" \
  --role "开篇" \
  --purpose "建立主角困境" \
  --events "主角准备修路，全村阻拦"
```

手动保存章节草稿：

```bash
plotforge-cli chapter draft \
  --project "/path/to/novel" \
  --number 1 \
  --content "这里是第一章正文。"
```

预览 AI 自动写下一章会使用的章节号、版本和提示词，不调用模型：

```bash
plotforge-cli chapter next --project "/path/to/novel" --dry-run
```

让 AI 自动写指定章节并保存为草稿：

```bash
plotforge-cli chapter write \
  --project "/path/to/novel" \
  --number 78 \
  --guidance "节奏紧一点，结尾留下强钩子"
```

让 AI 自动判断下一章并保存为草稿：

```bash
plotforge-cli chapter next \
  --project "/path/to/novel" \
  --guidance "承接上一章，不要跳时间线"
```

`chapter write` 和 `chapter next` 会读取项目设定、章节蓝图、角色状态、上一章结尾和后续蓝图参考，然后调用默认模型生成正文。可以用 `--model <model-id>` 指定模型，用 `--temperature` 和 `--max-tokens` 调整生成参数。

### 底层通道

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
