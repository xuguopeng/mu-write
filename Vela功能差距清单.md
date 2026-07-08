# Vela 功能差距清单

生成时间：2026-05-19

## 结论

当前项目已经具备小说生成器的基础数据模型、Tauri 本地能力、AI 生成调用和 IDE 壳层，但与 Vela 的完整写作 IDE 相比，差距主要集中在：

- 缺少完整的多项目/工作区文件化管理闭环。
- 写作流水线已有单点能力，但缺少可编排、可暂停、可追踪的端到端流程。
- 世界观、角色、大纲、章节已有数据表和生成入口，但缺少成熟的交互与版本管理。
- 模型配置已有基础 BYOK，但缺少持久化加载、默认模型、流式生成、代理、Embedding/RAG 等 Vela 级能力。
- 本地知识库/RAG 基本缺失。
- 前端是 Vue，和 Vela React 技术栈差异较大；后续适合先迁移壳层，再分批迁移业务模块。

## 功能覆盖矩阵

| 模块 | Vela 目标能力 | 当前项目覆盖 | 状态 | 证据 | 后续处理 |
| --- | --- | --- | --- | --- | --- |
| 桌面壳/IDE 布局 | 类 IDE 多面板工作区、活动栏、侧栏、编辑区、AI 面板、底栏、状态栏 | 已有 `DesktopShell.vue`，包含 `ActivityBar`、`PrimarySidebar`、`EditorWorkbench`、`AssistantPanel`、`BottomPanel`、`StatusBar` | 部分具备 | `src/shell/components/DesktopShell.vue`、`src/shell/stores/useShellStore.ts` | React 壳层优先迁移，保持布局语义不变 |
| 快捷键与面板开关 | 常用 IDE 快捷键、面板显示隐藏、缩放 | 已有 `Cmd/Ctrl+N`、缩放、侧栏、底栏快捷键 | 部分具备 | `src/shell/components/DesktopShell.vue` | 迁移到 React hook，并补充命令面板/焦点管理 |
| 多项目/工作区 | 创建、打开、最近项目、项目目录 `.vela` | 有项目目录命令和最近项目 store，但当前核心数据固定为 `id='main'` | 部分具备 | `src-tauri/src/lib.rs` 的 `ensure_project_dir`，`src/api/database.api.ts` 的 `project_core` | 建立项目上下文，避免单库单项目假设 |
| 小说基础信息 | 小说名称、类型、受众、章节数、字数、结构、视角、风格 | 数据模型完整，具备保存读取 API | 已具备 | `src/api/database.api.ts` 的 `ProjectCore` | React 表单迁移即可 |
| 世界观管理 | 世界观生成、编辑、结构化管理 | 有生成 API 和展示/生成组件，保存到 `project_core.worldbuilding` | 部分具备 | `generateWorldbuild`、`WorldbuildView.vue`、`WorldbuildGenerator.vue` | 增加结构化字段、版本、引用到章节上下文 |
| 角色管理 | 角色创建、生成、关系、成长弧、状态追踪 | 有 `characters` 表，字段覆盖角色设定和章节状态；有生成/列表组件 | 部分具备 | `Character`、`generateCharacter`、`CharacterList.vue` | 补关系图、角色状态随章节更新、冲突检测 |
| 大纲/蓝图 | 按结构生成全书章节蓝图，章节意图维护 | 有 `blueprints` 表、`generateOutline`、章节意图编辑组件 | 部分具备 | `Blueprint`、`generateOutline`、`ChapterIntentEditor.vue` | 补分卷/剧情线/局部重排/锁定章节 |
| 章节生成 | 根据蓝图、角色、世界观、前文生成章节 | 有 `generateChapter`，会读取蓝图、角色、前一章尾部上下文 | 部分具备 | `generateChapter` | 需要保存生成结果、版本流、批量生成、失败重试 |
| 章节编辑 | Markdown/RichText 编辑、章节列表、草稿版本 | 有章节列表、编辑器、草稿表和内容表 | 部分具备 | `ChapterEditor.vue`、`MarkdownEditor.vue`、`RichTextEditor.vue`、`drafts`/`contents` | React 迁移时保留 Tiptap 能力，补自动保存与版本比较 |
| 审阅/审核 | 章节质量评估、问题列表、建议 | 有 `auditChapter` 和 `AuditReportViewer.vue` | 部分具备 | `generate.api.ts` 的 `auditChapter` | 需要把审核结果结构化持久化，并和修订链路关联 |
| 重写/修订/润色 | 按审核意见或指定维度重写、润色 | 有 `reviseChapter`，但未看到独立 polish/rewrite 流程闭环 | 部分具备 | `generate.api.ts` 的 `reviseChapter` | 拆分 rewrite/polish/review action，接入版本管理 |
| 写作流水线 | 大纲 → 章节 → 审核 → 修订 → 确认的可编排流程 | `useWorkflowStore` 有 run/step/log/waiting 状态，但 `confirmContinue` 仍是 TODO | 部分具备 | `src/stores/useWorkflowStore.ts` | 实现执行器、暂停确认、日志、错误恢复 |
| 本地知识库/RAG | 文档导入、切片、Embedding、检索增强生成 | 当前未见知识库、embedding 表、检索 API；模型类型有 `embedding` 设想但后端未实现 | 缺失 | `ModelProfile.purposes`、项目目录未见 RAG 模块 | 新增知识库模块、向量存储/FTS、上下文注入 |
| 模型配置/BYOK | 多供应商模型、用途绑定、连接测试、默认模型 | 有模型配置结构、保存/删除/测试、按 purpose 选择；但后端内存态，缺少启动加载和默认模型持久化 | 部分具备 | `src-tauri/src/lib.rs`、`src/stores/useLLMStore.ts`、`src/api/llm.api.ts` | 持久化加载 SQLite，补默认模型、供应商预设、密钥安全存储 |
| 流式生成 | AI 输出流式返回、取消生成 | 当前本地 Tauri command 是同步 `llm_generate`，没有 stream/cancel | 缺失 | `src-tauri/src/lib.rs` | 参考 Vela `llm:generate-stream` 设计 Tauri event stream |
| 代理配置 | HTTP/SOCKS 代理，模型请求走代理 | 当前未见全局代理配置 | 缺失 | `src-tauri/src/lib.rs` | 后续加全局设置并在 reqwest client 层生效 |
| 导出发布 | 导出章节/全文 | 有 `export.api.ts` 和 Tauri save dialog | 部分具备 | `src/api/export.api.ts`、`src-tauri/src/lib.rs` 文件命令 | 补 docx/epub/pdf、多章节批量导出 |
| 任务/历史 | 生成任务、历史记录、状态跟踪 | 有 `tasks.api.ts`、`useTaskStore.ts`、`useWorkflowStore.ts`，但当前生成 API 多为同步兼容 task_id | 部分具备 | `generate.api.ts`、`useTaskStore.ts` | 统一任务模型和工作流执行记录 |
| 设置页 | 模型、主题、偏好配置 | 有 `SettingsView.vue`、主题 store、LLM store | 部分具备 | `SettingsView.vue`、`useThemeStore.ts` | React 迁移并补代理、默认模型、知识库设置 |
| Electron 到 Tauri 等价能力 | 本地文件、数据库、窗口、IPC、网络请求 | Tauri 2 基础已在位，Rust command 和插件可承接 | 已具备基础 | `src-tauri/tauri.conf.json`、`src-tauri/src/lib.rs` | 不迁移 Electron 主进程，按能力重写必要 IPC |

## 优先级建议

### P0：React 壳层迁移前必须保留的能力

- `DesktopShell` 布局结构：标题栏、活动栏、主侧栏、编辑区、AI 面板、底栏、状态栏。
- 初始化流程：主题、模型配置、最近项目。
- 快捷键：新建项目、缩放、侧栏、底栏。
- Tauri API 适配层：`shared/api/tauriClient.ts` 和 `databaseClient.ts`。

### P1：最贴近 Vela 的核心写作能力

- 项目核心表单与工作区。
- 世界观、角色、大纲、章节四个域的 React 页面。
- 章节生成、审核、修订的版本化保存。
- 可执行的 workflow runner，替换当前 TODO 状态。

### P2：拉开差距的能力

- 本地知识库/RAG。
- 流式生成与取消。
- 默认模型/Embedding 模型/代理配置。
- 导出格式扩展和审阅报告持久化。

## React 迁移边界

建议下一步只落 React 前端壳层，不改 Rust 后端，不改数据库表结构：

- 替换入口：`src/main.ts`、`src/App.vue`。
- 新建 React 壳层：`src/react/App.tsx`、`src/react/shell/*`。
- 状态层从 Pinia 迁移到轻量 React store；优先迁移 shell/theme/project/llm 初始化，不碰复杂业务。
- 样式按项目规则改用 UnoCSS utility class，逐步减少 scoped CSS/SCSS。