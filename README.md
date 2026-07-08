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

## 特别感谢

本项目基于 [heider-x/vela](https://github.com/heider-x/vela) 仓库改造而来，感谢原项目提供的 AI 小说创作 IDE 思路与实现参考。
