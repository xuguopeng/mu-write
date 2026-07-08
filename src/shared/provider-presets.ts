/**
 * 服务商预设配置 — 共享类型定义
 * 渲染进程与主进程共同使用，持久化在 ~/.vela/provider-presets.json
 */

/** 单个模型的预设 — name + 该模型的输出 token 上限 */
export interface ModelPreset {
  name: string
  maxTokens: number
}

/** 单个服务商的预设配置 */
export interface ProviderPreset {
  /** 服务商唯一标识（内置值如 openai/deepseek，用户可自定义如 my-proxy） */
  provider: string
  /** 界面显示名称，缺省时使用 provider ID */
  displayName?: string
  /** 默认 API 地址 */
  baseUrl: string
  /** 默认调用协议：openai 兼容 / anthropic 兼容 */
  protocol: string
  /** 支持的生成模型列表（含各自的 maxTokens） */
  models: ModelPreset[]
  /** 支持的向量模型列表（embedding 模型不需要 maxTokens） */
  embeddingModels: string[]
}

/** 内置默认预设（首次启动时写入持久化文件） */
export const BUILTIN_PRESETS: ProviderPreset[] = [
  {
    provider: 'openai',
    displayName: 'OpenAI',
    baseUrl: 'https://api.openai.com/v1',
    protocol: 'openai',
    models: [
      { name: 'gpt-4o', maxTokens: 16384 },
      { name: 'gpt-4o-mini', maxTokens: 16384 },
      { name: 'gpt-4-turbo', maxTokens: 4096 },
      { name: 'gpt-3.5-turbo', maxTokens: 4096 },
    ],
    embeddingModels: ['text-embedding-3-small', 'text-embedding-3-large', 'text-embedding-ada-002'],
  },
  {
    provider: 'deepseek',
    displayName: 'DeepSeek',
    baseUrl: 'https://api.deepseek.com/v1',
    protocol: 'openai',
    models: [
      { name: 'deepseek-chat', maxTokens: 65536 },
      { name: 'deepseek-reasoner', maxTokens: 65536 },
    ],
    embeddingModels: [],
  },
  {
    provider: 'qwen',
    displayName: '通义千问 / 阿里云百炼',
    baseUrl: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
    protocol: 'openai',
    models: [
      { name: 'qwen-turbo', maxTokens: 8192 },
      { name: 'qwen-plus', maxTokens: 32768 },
      { name: 'qwen-max', maxTokens: 8192 },
      { name: 'qwen-long', maxTokens: 32768 },
    ],
    embeddingModels: ['text-embedding-v4', 'text-embedding-v3'],
  },
  {
    provider: 'bigmodel',
    displayName: '智谱 GLM',
    baseUrl: 'https://open.bigmodel.cn/api/paas/v4',
    protocol: 'openai',
    models: [
      { name: 'glm-4-plus', maxTokens: 32768 },
      { name: 'glm-4-air', maxTokens: 32768 },
      { name: 'glm-4-flash', maxTokens: 32768 },
      { name: 'glm-4-long', maxTokens: 32768 },
    ],
    embeddingModels: ['embedding-3'],
  },
  {
    provider: 'moonshot',
    displayName: 'Moonshot / Kimi',
    baseUrl: 'https://api.moonshot.cn/v1',
    protocol: 'openai',
    models: [
      { name: 'moonshot-v1-8k', maxTokens: 8192 },
      { name: 'moonshot-v1-32k', maxTokens: 32768 },
      { name: 'moonshot-v1-128k', maxTokens: 32768 },
    ],
    embeddingModels: [],
  },
  {
    provider: 'minimax',
    displayName: 'MiniMax',
    baseUrl: 'https://api.minimax.chat/v1',
    protocol: 'openai',
    models: [
      { name: 'abab6.5s-chat', maxTokens: 24576 },
      { name: 'abab6.5g-chat', maxTokens: 8192 },
      { name: 'abab6.5t-chat', maxTokens: 8192 },
    ],
    embeddingModels: [],
  },
  {
    provider: 'baichuan',
    displayName: '百川智能',
    baseUrl: 'https://api.baichuan-ai.com/v1',
    protocol: 'openai',
    models: [
      { name: 'Baichuan4', maxTokens: 32768 },
      { name: 'Baichuan3-Turbo', maxTokens: 8192 },
      { name: 'Baichuan3-Turbo-128k', maxTokens: 32768 },
    ],
    embeddingModels: [],
  },
  {
    provider: 'yi',
    displayName: '零一万物 Yi',
    baseUrl: 'https://api.lingyiwanwu.com/v1',
    protocol: 'openai',
    models: [
      { name: 'yi-large', maxTokens: 32768 },
      { name: 'yi-medium', maxTokens: 16384 },
      { name: 'yi-lightning', maxTokens: 16384 },
    ],
    embeddingModels: [],
  },
  {
    provider: 'siliconflow',
    displayName: '硅基流动',
    baseUrl: 'https://api.siliconflow.cn/v1',
    protocol: 'openai',
    models: [
      { name: 'deepseek-ai/DeepSeek-V3', maxTokens: 65536 },
      { name: 'deepseek-ai/DeepSeek-R1', maxTokens: 65536 },
      { name: 'Qwen/Qwen2.5-72B-Instruct', maxTokens: 32768 },
      { name: 'THUDM/glm-4-9b-chat', maxTokens: 32768 },
    ],
    embeddingModels: ['BAAI/bge-m3', 'BAAI/bge-large-zh-v1.5'],
  },
  {
    provider: 'stepfun',
    displayName: '阶跃星辰 StepFun',
    baseUrl: 'https://api.stepfun.com/v1',
    protocol: 'openai',
    models: [
      { name: 'step-1-8k', maxTokens: 8192 },
      { name: 'step-1-32k', maxTokens: 32768 },
      { name: 'step-1-128k', maxTokens: 32768 },
    ],
    embeddingModels: [],
  },
  {
    provider: 'iflytek',
    displayName: '讯飞星火',
    baseUrl: 'https://spark-api-open.xf-yun.com/v1',
    protocol: 'openai',
    models: [
      { name: 'spark-4.0', maxTokens: 32768 },
      { name: 'spark-3.5-max', maxTokens: 32768 },
      { name: 'spark-lite', maxTokens: 8192 },
    ],
    embeddingModels: [],
  },
  {
    provider: 'doubao',
    displayName: '火山方舟 / 豆包',
    baseUrl: 'https://ark.cn-beijing.volces.com/api/v3',
    protocol: 'openai',
    models: [
      { name: 'doubao-pro-32k', maxTokens: 32768 },
      { name: 'doubao-pro-128k', maxTokens: 32768 },
      { name: 'doubao-lite-32k', maxTokens: 32768 },
    ],
    embeddingModels: ['doubao-embedding-text-240715'],
  },
  {
    provider: 'baidu',
    displayName: '百度千帆 / 文心',
    baseUrl: 'https://qianfan.baidubce.com/v2',
    protocol: 'openai',
    models: [
      { name: 'ernie-4.0-turbo-8k', maxTokens: 8192 },
      { name: 'ernie-3.5-8k', maxTokens: 8192 },
      { name: 'ernie-speed-8k', maxTokens: 8192 },
    ],
    embeddingModels: ['bge-large-zh', 'embedding-v1'],
  },
  {
    provider: 'hunyuan',
    displayName: '腾讯混元',
    baseUrl: 'https://api.hunyuan.cloud.tencent.com/v1',
    protocol: 'openai',
    models: [
      { name: 'hunyuan-turbo', maxTokens: 32768 },
      { name: 'hunyuan-standard', maxTokens: 32768 },
      { name: 'hunyuan-lite', maxTokens: 8192 },
    ],
    embeddingModels: [],
  },
  {
    provider: 'gemini',
    displayName: 'Google Gemini',
    baseUrl: 'https://generativelanguage.googleapis.com/v1beta/openai',
    protocol: 'openai',
    models: [
      { name: 'gemini-2.5-pro', maxTokens: 65536 },
      { name: 'gemini-2.5-flash', maxTokens: 65536 },
      { name: 'gemini-2.0-flash', maxTokens: 8192 },
    ],
    embeddingModels: ['text-embedding-004'],
  },
  {
    provider: 'minimax-anthropic',
    displayName: 'MiniMax（Anthropic 协议）',
    baseUrl: 'https://api.minimaxi.com/anthropic',
    protocol: 'anthropic',
    models: [
      { name: 'MiniMax-M2.7', maxTokens: 65536 },
      { name: 'MiniMax-M2.7-highspeed', maxTokens: 65536 },
      { name: 'MiniMax-M2.5', maxTokens: 65536 },
      { name: 'MiniMax-M2.5-highspeed', maxTokens: 65536 },
      { name: 'MiniMax-M2.1', maxTokens: 65536 },
      { name: 'MiniMax-M2.1-highspeed', maxTokens: 65536 },
      { name: 'MiniMax-M2', maxTokens: 65536 },
    ],
    embeddingModels: [],
  },
  {
    provider: 'bigmodel-anthropic',
    displayName: '智谱 GLM Coding（Anthropic 协议）',
    baseUrl: 'https://open.bigmodel.cn/api/anthropic',
    protocol: 'anthropic',
    models: [
      { name: 'glm-5.1', maxTokens: 65536 },
      { name: 'glm-5', maxTokens: 65536 },
      { name: 'glm-4.7', maxTokens: 65536 },
    ],
    embeddingModels: [],
  },
  {
    provider: 'ollama',
    displayName: 'Ollama（本地）',
    baseUrl: 'http://localhost:11434/v1',
    protocol: 'openai',
    models: [
      { name: 'llama3.3', maxTokens: 4096 },
      { name: 'llama3.2', maxTokens: 4096 },
      { name: 'qwen2.5', maxTokens: 8192 },
      { name: 'qwen2.5-coder', maxTokens: 8192 },
      { name: 'mistral', maxTokens: 4096 },
      { name: 'phi4', maxTokens: 4096 },
      { name: 'gemma3', maxTokens: 8192 },
    ],
    embeddingModels: ['nomic-embed-text', 'mxbai-embed-large', 'bge-m3'],
  },
]
