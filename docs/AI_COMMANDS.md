# AI Command Reference

This file is the command entrypoint for AI agents that need to operate Mu Write / PlotForge from the terminal.

Read this file first, then use `plotforge-cli` instead of editing project databases directly.

## Runtime

From the repository root:

```bash
cd "/Users/xuguopeng/Documents/徐徐如声/徐徐如声/产品库/AI爆款小说生成器/code/徐-写小说/desktop"
```

Development binary:

```bash
src-tauri/target/debug/plotforge-cli
```

Build or rebuild the CLI:

```bash
pnpm run cli:build
```

Direct Cargo run:

```bash
cargo run --manifest-path src-tauri/Cargo.toml --bin plotforge-cli -- help
```

Default novel library path:

```bash
/Users/xuguopeng/Documents/徐徐如声/徐徐如声/创作库/小说
```

Recent project record:

```bash
/Users/xuguopeng/.vela/recent-projects.json
```

Do not print full `config:get` or `llm:list-models` output into public logs because it can contain API keys.

## High-Level Commands

### Help

```bash
src-tauri/target/debug/plotforge-cli help
```

### List Low-Level Channels

```bash
src-tauri/target/debug/plotforge-cli list
```

### Recent Projects

```bash
src-tauri/target/debug/plotforge-cli project recent
```

Returns JSON:

```json
[
  {
    "name": "项目名",
    "path": "/absolute/project/path",
    "updatedAt": "2026-07-03T04:06:11.269051+00:00"
  }
]
```

### Project Status

```bash
src-tauri/target/debug/plotforge-cli project status
```

Returns JSON:

```json
[
  {
    "name": "项目名",
    "path": "/absolute/project/path",
    "blueprintToChapter": 100,
    "draftToChapter": 77,
    "finalizedToChapter": 76,
    "draftCount": 86
  }
]
```

Meaning:

| Field | Meaning |
|---|---|
| `blueprintToChapter` | Highest chapter number that has a chapter blueprint. |
| `draftToChapter` | Highest chapter number that has any draft. |
| `finalizedToChapter` | Highest chapter number marked as finalized. |
| `draftCount` | Total draft rows found across blueprint chapters. |

### Create Project

```bash
src-tauri/target/debug/plotforge-cli project new "测试小说" \
  --dir "/Users/xuguopeng/Documents/徐徐如声/徐徐如声/创作库/小说" \
  --genre "都市" \
  --audience "男频读者"
```

Arguments:

| Argument | Required | Meaning |
|---|---:|---|
| `<name>` | yes | Project folder and display name. |
| `--dir` | yes | Parent directory where the project folder will be created. |
| `--genre` | no | Novel genre. |
| `--audience` | no | Target audience. |

### Create Or Update Chapter Blueprint

```bash
src-tauri/target/debug/plotforge-cli chapter blueprint \
  --project "/absolute/project/path" \
  --number 1 \
  --title "第一章" \
  --role "开篇" \
  --purpose "建立主角困境" \
  --events "主角准备修路，全村阻拦" \
  --characters "主角,村长" \
  --hook "为什么全村都不让他修路？" \
  --guidance "年代感强一点，少解释，多用生活细节推进。" \
  --notes "本章需要埋下后续反转。"
```

Arguments:

| Argument | Required | Meaning |
|---|---:|---|
| `--project` | yes | Project directory. |
| `--number` | yes | Chapter number. |
| `--title` | no | Chapter title; defaults to `第N章`. |
| `--role` | no | Story role of this chapter. |
| `--purpose` | no | Writing purpose. |
| `--events` | no | Key events. |
| `--characters` | no | Comma-separated character names. |
| `--hook` | no | Suspense hook. |
| `--guidance` | no | Author guidance for this chapter. |
| `--notes` | no | Chapter notes / timeline details. |

### Save Manual Draft

```bash
src-tauri/target/debug/plotforge-cli chapter draft \
  --project "/absolute/project/path" \
  --number 1 \
  --content "这里是第一章正文。"
```

Optional:

```bash
--source write
```

The CLI automatically chooses the next draft version.

### Preview AI Writing Without Calling Model

Use this before real generation.

```bash
src-tauri/target/debug/plotforge-cli chapter next \
  --project "/absolute/project/path" \
  --dry-run
```

For a specific chapter:

```bash
src-tauri/target/debug/plotforge-cli chapter write \
  --project "/absolute/project/path" \
  --number 78 \
  --dry-run
```

Returns JSON with:

```json
{
  "success": true,
  "dryRun": true,
  "chapterNumber": 78,
  "version": 1,
  "blueprint": {},
  "prompt": "..."
}
```

### Let AI Write Next Chapter

```bash
src-tauri/target/debug/plotforge-cli chapter next \
  --project "/absolute/project/path" \
  --guidance "承接上一章，不要跳时间线"
```

Behavior:

1. Reads project config.
2. Reads chapter blueprints.
3. Detects next chapter from highest draft/finalized chapter plus one.
4. Reads current chapter blueprint.
5. Reads character states.
6. Reads previous chapter ending.
7. Reads following 5 blueprints for future context.
8. Calls the default model with `llm:generate`.
9. Saves the generated content as a new draft.

Optional generation controls:

```bash
--model "<model-id>"
--temperature 0.75
--max-tokens 16000
```

### Let AI Write Specific Chapter

```bash
src-tauri/target/debug/plotforge-cli chapter write \
  --project "/absolute/project/path" \
  --number 78 \
  --guidance "节奏紧一点，结尾留下强钩子"
```

The chapter must already have a blueprint.

## Current Useful Project Paths

```bash
/Users/xuguopeng/Documents/徐徐如声/徐徐如声/创作库/小说/我出钱修路，全村拦我
/Users/xuguopeng/Documents/徐徐如声/徐徐如声/创作库/小说/沧墟
/Users/xuguopeng/Documents/徐徐如声/徐徐如声/创作库/小说/以缝针镇禁区
/Users/xuguopeng/Documents/徐徐如声/徐徐如声/创作库/小说/主角
/Users/xuguopeng/Documents/徐徐如声/徐徐如声/创作库/小说/请回答1992
```

## Common Agent Workflows

### Check What To Work On

```bash
src-tauri/target/debug/plotforge-cli project status
```

Then pick a project. If `draftToChapter` is 77 and `blueprintToChapter` is 100, `chapter next` will try chapter 78.

### Safely Preview Next AI Chapter

```bash
src-tauri/target/debug/plotforge-cli chapter next \
  --project "/Users/xuguopeng/Documents/徐徐如声/徐徐如声/创作库/小说/我出钱修路，全村拦我" \
  --dry-run
```

Check `chapterNumber`, `version`, `blueprint.title`, and prompt size before real generation.

### Real Generate Next Chapter

```bash
src-tauri/target/debug/plotforge-cli chapter next \
  --project "/Users/xuguopeng/Documents/徐徐如声/徐徐如声/创作库/小说/我出钱修路，全村拦我"
```

### Read The Draft After Generation

The generation result contains a draft id. Use:

```bash
src-tauri/target/debug/plotforge-cli call \
  --project "/absolute/project/path" \
  db:draft-get-full \
  '[123]'
```

Replace `123` with the draft id.

## Low-Level Call Format

Use low-level calls when a high-level command does not exist.

```bash
src-tauri/target/debug/plotforge-cli call <channel> '[json-array-args]'
```

If the channel needs a project:

```bash
src-tauri/target/debug/plotforge-cli call \
  --project "/absolute/project/path" \
  <channel> \
  '[json-array-args]'
```

The JSON array maps to the frontend `ipc.invoke(channel, ...args)` argument order.

## Low-Level Channels

### Config

```bash
src-tauri/target/debug/plotforge-cli call config:get
src-tauri/target/debug/plotforge-cli call config:set '[{"theme":"system"}]'
src-tauri/target/debug/plotforge-cli call config:get-vela-home
```

Warning: `config:get` can expose sensitive local settings.

### Project

```bash
src-tauri/target/debug/plotforge-cli call project:create '[{"name":"测试小说","path":"/parent/dir","genre":"都市","targetAudience":"男频读者"}]'
src-tauri/target/debug/plotforge-cli call project:open '["/absolute/project/path"]'
src-tauri/target/debug/plotforge-cli call project:save '["project-id",{"name":"新名字","path":"/absolute/project/path"}]'
src-tauri/target/debug/plotforge-cli call project:update-config '["project-id",{"name":"新名字","path":"/absolute/project/path"}]'
src-tauri/target/debug/plotforge-cli call project:recent-list
```

### File System

```bash
src-tauri/target/debug/plotforge-cli call fs:read-file '["/absolute/file.md"]'
src-tauri/target/debug/plotforge-cli call fs:write-file '["/absolute/file.md","content"]'
src-tauri/target/debug/plotforge-cli call fs:list-dir '["/absolute/dir"]'
src-tauri/target/debug/plotforge-cli call fs:mkdir '["/absolute/dir"]'
src-tauri/target/debug/plotforge-cli call fs:check-exists '["/absolute/path"]'
src-tauri/target/debug/plotforge-cli call fs:read-json '["/absolute/file.json"]'
src-tauri/target/debug/plotforge-cli call fs:write-json '["/absolute/file.json",{"ok":true}]'
```

### Project Core Database

```bash
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:project-core-get
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:project-core-update '[{"projectName":"测试小说","genre":"都市","targetAudience":"男频读者"}]'
```

Common `db:project-core-update` fields:

```json
{
  "projectName": "",
  "genre": "",
  "subGenre": "",
  "targetAudience": "",
  "totalChapters": 100,
  "wordsPerChapter": 3000,
  "plotStructure": "three_act",
  "narrativePov": "third_limited",
  "writingStyle": "",
  "referenceWorks": "",
  "globalGuidance": "",
  "goldenFinger": "",
  "premise": "",
  "worldbuilding": "",
  "charactersArch": "",
  "synopsis": "",
  "characterStates": ""
}
```

### Blueprints

```bash
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:blueprint-get-all
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:blueprint-get '[1]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:blueprint-upsert '[{"chapterNumber":1,"title":"第一章","role":"开篇","purpose":"","keyEvents":"","characters":[],"suspenseHook":"","userGuidance":"","notes":"","notesUpdatedAt":""}]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:blueprint-upsert-many '[[{"chapterNumber":1,"title":"第一章","role":"","purpose":"","keyEvents":"","characters":[],"suspenseHook":"","userGuidance":"","notes":"","notesUpdatedAt":""}]]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:blueprint-update-notes '[1,"新的章节要点"]'
```

### Characters

```bash
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:character-get-all
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:character-upsert '[{"name":"主角","role":"protagonist","gender":"","age":"","appearance":"","personality":"","background":"","abilities":"","motivation":"","relationships":"","arc":"","notes":"","currentState":{"location":"","powerLevel":"","physicalState":"","mentalState":"","keyItems":"","recentEvents":"","updatedAtChapter":0}}]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:character-save-all '[[{"name":"主角","role":"protagonist"}]]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:character-delete '["主角"]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:character-update-state '["主角",{"location":"","powerLevel":"","physicalState":"","mentalState":"","keyItems":"","recentEvents":"","updatedAtChapter":1}]'
```

### Drafts

```bash
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:draft-create '[{"chapterNumber":1,"version":1,"source":"write","content":"正文","wordCount":2}]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:draft-list '[1]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:draft-get-meta '[123]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:draft-get-full '[123]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:draft-get-latest '[1]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:draft-get-finalized '[1]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:draft-get-max-finalized-chapter
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:draft-next-version '[1]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:draft-update-status '[123,"finalized",3000]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:draft-update-content '[123,"新的正文",4]'
```

### Revisions

```bash
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:revision-create '[{"baseDraftId":123,"revisionIndex":1,"revisionType":"refine","userPrompt":"","reviewSourceId":null,"content":"润色稿","wordCount":3}]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:revision-list '[123]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:revision-get-pending '[123]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:revision-get-full '[456]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:revision-next-index '[123]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:revision-mark-merged '[456,789]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:revision-mark-discarded '[456]'
```

### Reviews

```bash
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:review-create '[{"baseDraftId":123,"reviewIndex":1,"content":"审稿报告"}]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:review-list '[123]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:review-get-latest '[123]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:review-get-full '[456]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:review-next-index '[123]'
```

### Post Process

```bash
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:post-process-create-run '[{"triggerSourceType":"chapter","triggerSourceId":"1","sourceLabel":"第1章","steps":[{"key":"check","label":"检查","critical":true}]}]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:post-process-get-latest-run '["chapter","1"]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:post-process-get-steps '["run-id"]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:post-process-mark-step-ok '["run-id","check"]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:post-process-mark-step-failed '["run-id","check","错误信息"]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:post-process-is-all-passed '["chapter","1"]'
```

### LLM Logs And Summary

```bash
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:log-llm-call '[{"modelId":"model","modelName":"model","purpose":"generation","promptTokens":0,"completionTokens":0,"totalTokens":0,"durationMs":0,"success":true,"errorMessage":""}]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:get-llm-stats
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:get-llm-history '[30]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:save-summary-snapshot '[1,"角色状态摘要"]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" db:get-latest-summary
```

### Knowledge Base

```bash
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" kb:import-document '["/absolute/file.txt"]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" kb:import-folder '["/absolute/folder"]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" kb:import-text '["文本内容","note.txt","/absolute/project/path"]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" kb:search '["主角 金手指",5]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" kb:search-with-scope '["主角",1,10,5]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" kb:list-documents
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" kb:remove-document '["doc-id"]'
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" kb:stats
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" kb:get-vectorless-count
src-tauri/target/debug/plotforge-cli call --project "/absolute/project/path" kb:backfill-vectors
```

### LLM

```bash
src-tauri/target/debug/plotforge-cli call llm:list-models
src-tauri/target/debug/plotforge-cli call llm:test-connection '[{"id":"model-id"}]'
src-tauri/target/debug/plotforge-cli call llm:generate '[{"modelId":"","messages":[{"role":"user","content":"写一句测试文本"}],"temperature":0.7,"maxTokens":1000}]'
```

Warning: `llm:list-models` can expose API keys.

### MCP

```bash
src-tauri/target/debug/plotforge-cli call mcp:load-config
src-tauri/target/debug/plotforge-cli call mcp:get-config-path
src-tauri/target/debug/plotforge-cli call mcp:get-servers-status
src-tauri/target/debug/plotforge-cli call mcp:list-tools
src-tauri/target/debug/plotforge-cli call mcp:list-resources
src-tauri/target/debug/plotforge-cli call mcp:connect '[{"id":"server-id","command":"node","args":[]}]'
src-tauri/target/debug/plotforge-cli call mcp:disconnect '["server-id"]'
src-tauri/target/debug/plotforge-cli call mcp:disconnect-all
src-tauri/target/debug/plotforge-cli call mcp:call-tool '["server-id","tool-name",{}]'
```

## Deep Link

Format:

```bash
src-tauri/target/debug/plotforge-cli deeplink 'plotforge://call?channel=<channel>&args=<json-array>&project=<path>'
```

Examples:

```bash
src-tauri/target/debug/plotforge-cli deeplink 'plotforge://call?channel=config:get'
src-tauri/target/debug/plotforge-cli deeplink 'plotforge://call?channel=project:recent-list'
```

URL-encoded args example:

```bash
src-tauri/target/debug/plotforge-cli deeplink 'plotforge://call?channel=kb:search&project=/absolute/project/path&args=%5B%22%E4%B8%BB%E8%A7%92%22%2C5%5D'
```

## Unsupported In CLI

These require the desktop UI / event loop:

```text
dialog:*
import:* when it opens file pickers
llm:generate-stream
```

Use `llm:generate` for non-streaming model calls from CLI.

## Safety Rules For AI Agents

1. Prefer high-level commands over raw database calls.
2. Always run `chapter next --dry-run` before real AI generation.
3. Do not call `config:get` or `llm:list-models` unless needed; their output can contain secrets.
4. Do not edit `.vela/project.db` directly.
5. Do not overwrite draft content unless the user explicitly asks.
6. Use absolute paths.
7. After creating or writing content, report the project path, chapter number, draft id, version, and word count.
