import { create } from 'zustand'
import { ipc } from '../services/ipc-client'
import type { ModelProfile, LLMResponse, TokenUsage } from '../shared/ipc-channels'

export const isUsableModel = (model: ModelProfile | undefined, onlyEmbedding = false) => {
  if (!model?.enabled || !model.apiKey?.trim()) return false
  const hasEmbedding = model.purposes?.includes('embedding') ?? false
  return onlyEmbedding ? hasEmbedding : !hasEmbedding
}

const firstUsableModelId = (models: ModelProfile[], onlyEmbedding = false) => (
  models.find((model) => isUsableModel(model, onlyEmbedding))?.id ?? null
)

const resolveUsableModelId = (models: ModelProfile[], modelId: string | null | undefined, onlyEmbedding = false) => (
  isUsableModel(models.find((model) => model.id === modelId), onlyEmbedding)
    ? modelId ?? null
    : firstUsableModelId(models, onlyEmbedding)
)

/** 流式生成的回调 */
interface StreamCallbacks {
  onChunk?: (chunk: string) => void
  onDone?: (fullText: string, usage?: TokenUsage) => void
  onError?: (error: string) => void
}

interface LLMState {
  /** 已配置的模型列表 */
  models: ModelProfile[]
  /** 当前默认生成模型 ID */
  defaultModelId: string | null
  /** 当前默认向量模型 ID */
  defaultEmbeddingModelId: string | null
  /** 正在进行的活跃请求 */
  activeRequests: Map<string, { status: 'running' | 'done' | 'error'; text: string }>
  /** 是否已加载模型配置 */
  loaded: boolean

  // ===== Actions =====
  /** 初始化（加载模型列表 + 默认模型 ID） */
  init: () => Promise<void>
  /** 加载模型列表 */
  loadModels: () => Promise<void>
  /** 保存模型 */
  saveModel: (model: ModelProfile) => Promise<boolean>
  /** 删除模型 */
  deleteModel: (modelId: string) => Promise<boolean>
  /** 设置默认生成模型（持久化到 ~/.vela/config.json） */
  setDefaultModel: (modelId: string | null) => void
  /** 设置默认向量模型（持久化到 ~/.vela/config.json） */
  setDefaultEmbeddingModel: (modelId: string | null) => void
  /** 非流式生成 */
  generate: (
    messages: Array<{ role: 'system' | 'user' | 'assistant'; content: string }>,
    modelId?: string,
    options?: { responseFormat?: { type: string }; thinking?: boolean }
  ) => Promise<LLMResponse>
  /** 流式生成 */
  generateStream: (
    messages: Array<{ role: 'system' | 'user' | 'assistant'; content: string }>,
    callbacks: StreamCallbacks,
    modelId?: string,
    options?: { responseFormat?: { type: string }; thinking?: boolean }
  ) => Promise<string>
  /** 取消生成 */
  cancelGeneration: (requestId: string) => Promise<void>
  /** 测试模型连接 */
  testConnection: (model: ModelProfile) => Promise<{ success: boolean; error?: string }>
}

export const useLLMStore = create<LLMState>()((set, get) => ({
  models: [],
  defaultModelId: null,
  defaultEmbeddingModelId: null,
  activeRequests: new Map(),
  loaded: false,

  init: async () => {
    if (get().loaded) return
    await get().loadModels()

    const [defaultId, defaultEmbeddingId] = await Promise.all([
      ipc.invoke<string | null>('llm:get-default-model').catch(() => null),
      ipc.invoke<string | null>('llm:get-default-embedding-model').catch(() => null),
    ])
    const models = get().models
    const nextDefaultId = resolveUsableModelId(models, defaultId)
    const nextDefaultEmbeddingId = resolveUsableModelId(models, defaultEmbeddingId, true)

    set({ defaultModelId: nextDefaultId, defaultEmbeddingModelId: nextDefaultEmbeddingId, loaded: true })
    if (nextDefaultId !== defaultId) ipc.invoke('llm:set-default-model', nextDefaultId).catch(() => {})
    if (nextDefaultEmbeddingId !== defaultEmbeddingId) ipc.invoke('llm:set-default-embedding-model', nextDefaultEmbeddingId).catch(() => {})
  },

  loadModels: async () => {
    const models = await ipc.invoke<ModelProfile[]>('llm:list-models').catch(() => [] as ModelProfile[])
    const nextDefaultId = resolveUsableModelId(models, get().defaultModelId)
    const nextDefaultEmbeddingId = resolveUsableModelId(models, get().defaultEmbeddingModelId, true)
    set({ models, defaultModelId: nextDefaultId, defaultEmbeddingModelId: nextDefaultEmbeddingId, loaded: true })
  },

  saveModel: async (model) => {
    const result = await ipc.invoke('llm:save-model', model)
    if (result.success) {
      await get().loadModels()
    }
    return result.success
  },

  deleteModel: async (modelId) => {
    const result = await ipc.invoke('llm:delete-model', modelId)
    if (result.success) {
      await get().loadModels()
    }
    return result.success
  },

  setDefaultModel: (modelId) => {
    const models = get().models
    const nextModelId = resolveUsableModelId(models, modelId)
    set({ defaultModelId: nextModelId })
    ipc.invoke('llm:set-default-model', nextModelId).catch(() => {})
  },

  setDefaultEmbeddingModel: (modelId) => {
    const models = get().models
    const nextModelId = resolveUsableModelId(models, modelId, true)
    set({ defaultEmbeddingModelId: nextModelId })
    ipc.invoke('llm:set-default-embedding-model', nextModelId).catch(() => {})
  },

  generate: async (messages, modelId, options) => {
    const mid = resolveUsableModelId(get().models, modelId ?? get().defaultModelId)
    if (!mid) return { success: false, content: '', error: '未配置可用模型，请先启用服务商并填写 API Key' }
    const model = get().models.find((item) => item.id === mid)
    if (!isUsableModel(model)) {
      return { success: false, content: '', error: '当前模型未启用或未填写 API Key' }
    }
    if (mid !== get().defaultModelId && !modelId) get().setDefaultModel(mid)
    return ipc.invoke('llm:generate', {
      modelId: mid,
      messages,
      responseFormat: options?.responseFormat as { type: 'json_object' | 'text' } | undefined,
      thinking: options?.thinking
    })
  },

  generateStream: async (messages, callbacks, modelId, options) => {
    const mid = resolveUsableModelId(get().models, modelId ?? get().defaultModelId)
    if (!mid) {
      callbacks.onError?.('未配置可用模型，请先启用服务商并填写 API Key')
      return ''
    }
    const model = get().models.find((item) => item.id === mid)
    if (!isUsableModel(model)) {
      callbacks.onError?.('当前模型未启用或未填写 API Key')
      return ''
    }
    if (mid !== get().defaultModelId && !modelId) get().setDefaultModel(mid)

    const requestId = crypto.randomUUID()

    // 注册流式事件监听
    const unsubChunk = ipc.on('llm:stream-chunk', (data) => {
      if (data.requestId === requestId) {
        callbacks.onChunk?.(data.chunk)
      }
    })

    const unsubDone = ipc.on('llm:stream-done', (data) => {
      if (data.requestId === requestId) {
        callbacks.onDone?.(data.fullText, data.usage)
        cleanup()
      }
    })

    const unsubError = ipc.on('llm:stream-error', (data) => {
      if (data.requestId === requestId) {
        callbacks.onError?.(data.error)
        cleanup()
      }
    })

    const cleanup = () => {
      unsubChunk()
      unsubDone()
      unsubError()
      const reqs = new Map(get().activeRequests)
      reqs.delete(requestId)
      set({ activeRequests: reqs })
    }

    // 标记活跃请求
    const reqs = new Map(get().activeRequests)
    reqs.set(requestId, { status: 'running', text: '' })
    set({ activeRequests: reqs })

    // 发起流式请求
    await ipc.invoke('llm:generate-stream', requestId, {
      modelId: mid,
      messages,
      stream: true,
      responseFormat: options?.responseFormat as { type: 'json_object' | 'text' } | undefined,
      thinking: options?.thinking
    })

    return requestId
  },

  cancelGeneration: async (requestId) => {
    await ipc.invoke('llm:cancel', requestId)
  },

  testConnection: async (model) => {
    return ipc.invoke('llm:test-connection', model)
  },
}))
