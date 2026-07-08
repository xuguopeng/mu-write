# 爆文工坊 PlotForge

为网文作者打造的 AI 连载生产工作台。

仓库地址：

```text
git@github.com:xuguopeng/mu-write.git
```

## 开发

```bash
corepack pnpm install
corepack pnpm run tauri:dev
```

## 构建

```bash
corepack pnpm run build
corepack pnpm run tauri:build
```

## Codex / CLI 调用

```bash
corepack pnpm run cli:build
src-tauri/target/debug/plotforge-cli list
```

更多说明见 [docs/codex-cli.md](docs/codex-cli.md)。

## 开源协议 / License

本项目采用 [GPL-3.0 License](LICENSE) 开源。您可以自由地运行、研究、分享和修改代码，但基于本项目修改并分发的新软件也必须同样遵循 GPL-3.0 协议开源。

## 修改说明

本项目是基于 [heider-x/vela](https://github.com/heider-x/vela) 的修改版本，主要改造方向包括：

- 使用 Tauri 作为桌面端运行与本地能力层。
- 移除内置 AI 聊天面板，保留写作生成、润色、知识库与工作流能力。
- 新增 Codex / CLI 调用入口与 `plotforge://` 深链调用能力。
- 面向中文网文连载生产场景调整产品定位与界面文案。

## 特别感谢

本项目基于 [heider-x/vela](https://github.com/heider-x/vela) 仓库改造而来，感谢原项目提供的 AI 小说创作 IDE 思路与实现参考。
