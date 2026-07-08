import { useEffect, useMemo, useState } from 'react'
import { Eye, EyeOff, ExternalLink, Save, Zap } from 'lucide-react'
import { useLLMStore } from '../../stores/llm-store'
import type { ModelProfile } from '../../shared/ipc-channels'
import { BUILTIN_PRESETS, type ModelPreset, type ProviderPreset } from '../../shared/provider-presets'
import { Button } from '../ui/Button'
import { Input } from '../ui/Input'
import { Switch } from '../ui/Switch'

const isProviderModel = (model: ModelProfile, preset: ProviderPreset) => model.provider === preset.provider

const isModelUsable = (model: ModelProfile) => Boolean(model.enabled && model.apiKey?.trim())

const modelIdFor = (preset: ProviderPreset, modelName: string, onlyEmbedding: boolean) => (
  `${preset.provider}:${onlyEmbedding ? 'embedding' : 'chat'}:${modelName}`
)

const modelProfileFromPreset = ({
  preset,
  presetModel,
  apiKey,
  onlyEmbedding,
}: {
  preset: ProviderPreset
  presetModel: ModelPreset | string
  apiKey: string
  onlyEmbedding: boolean
}): ModelProfile => {
  const modelName = typeof presetModel === 'string' ? presetModel : presetModel.name
  const maxTokens = typeof presetModel === 'string' ? 0 : presetModel.maxTokens

  return {
    id: modelIdFor(preset, modelName, onlyEmbedding),
    name: `${preset.displayName ?? preset.provider} · ${modelName}`,
    provider: preset.provider,
    protocol: (preset.protocol ?? 'openai') as 'openai' | 'gemini' | 'anthropic',
    modelName,
    apiKey,
    baseUrl: preset.baseUrl,
    temperature: 0.7,
    maxTokens: onlyEmbedding ? 0 : maxTokens,
    purposes: onlyEmbedding ? ['embedding'] : ['generation', 'refinement', 'summary'],
    enabled: true,
  }
}

export default function PresetProviderSettings() {
  const models = useLLMStore((s) => s.models)
  const defaultModelId = useLLMStore((s) => s.defaultModelId)
  const defaultEmbeddingModelId = useLLMStore((s) => s.defaultEmbeddingModelId)
  const loaded = useLLMStore((s) => s.loaded)
  const loadModels = useLLMStore((s) => s.loadModels)
  const saveModel = useLLMStore((s) => s.saveModel)
  const deleteModel = useLLMStore((s) => s.deleteModel)
  const setDefaultModel = useLLMStore((s) => s.setDefaultModel)
  const setDefaultEmbeddingModel = useLLMStore((s) => s.setDefaultEmbeddingModel)
  const [savingProvider, setSavingProvider] = useState<string | null>(null)
  const [visibleKeys, setVisibleKeys] = useState<Record<string, boolean>>({})
  const [drafts, setDrafts] = useState<Record<string, { apiKey: string; enabled: boolean }>>({})

  useEffect(() => {
    if (!loaded) loadModels()
  }, [loaded, loadModels])

  const presets = useMemo(
    () => BUILTIN_PRESETS.filter((preset) => preset.models.length > 0 || preset.embeddingModels.length > 0),
    [],
  )

  useEffect(() => {
    setDrafts((current) => {
      const next = { ...current }
      for (const preset of presets) {
        const existing = models.find((model) => isProviderModel(model, preset))
        if (!next[preset.provider]) {
          next[preset.provider] = {
            apiKey: existing?.apiKey ?? '',
            enabled: Boolean(existing?.enabled && existing?.apiKey?.trim()),
          }
        }
      }
      return next
    })
  }, [models, presets])

  const saveProvider = async (preset: ProviderPreset) => {
    const draft = drafts[preset.provider] ?? { apiKey: '', enabled: false }
    const apiKey = draft.apiKey.trim()
    const shouldEnable = draft.enabled && apiKey.length > 0
    setSavingProvider(preset.provider)

    const currentProviderModels = models.filter((model) => isProviderModel(model, preset))
    await Promise.all(currentProviderModels.map((model) => deleteModel(model.id)))

    if (shouldEnable) {
      for (const presetModel of preset.models) {
        await saveModel(modelProfileFromPreset({ preset, presetModel, apiKey, onlyEmbedding: false }))
      }
      for (const presetModel of preset.embeddingModels) {
        await saveModel(modelProfileFromPreset({ preset, presetModel, apiKey, onlyEmbedding: true }))
      }

      const firstChatModelName = preset.models[0]?.name
      const firstEmbeddingModelName = preset.embeddingModels[0]
      if (!defaultModelId && firstChatModelName) setDefaultModel(modelIdFor(preset, firstChatModelName, false))
      if (!defaultEmbeddingModelId && firstEmbeddingModelName) setDefaultEmbeddingModel(modelIdFor(preset, firstEmbeddingModelName, true))
    } else {
      const removedIds = new Set(currentProviderModels.map((model) => model.id))
      if (defaultModelId && removedIds.has(defaultModelId)) setDefaultModel(null)
      if (defaultEmbeddingModelId && removedIds.has(defaultEmbeddingModelId)) setDefaultEmbeddingModel(null)
    }

    await loadModels()
    setSavingProvider(null)
  }

  return (
    <div className="space-y-5">
      <div className="rounded-xl px-4 py-3 text-xs" style={{ backgroundColor: 'var(--color-hover)', color: 'var(--color-text-muted)' }}>
        服务商和模型已内置预设。打开开关、填写 Key 后点击“保存并拉取模型”，模型才会进入“模型配置”的下拉选择；关闭或未填 Key 时不可选。
      </div>

      <div className="space-y-3">
        {presets.map((preset) => {
          const draft = drafts[preset.provider] ?? { apiKey: '', enabled: false }
          const providerModels = models.filter((model) => isProviderModel(model, preset))
          const enabled = draft.enabled && draft.apiKey.trim().length > 0
          const modelCount = preset.models.length + preset.embeddingModels.length

          return (
            <div
              key={preset.provider}
              className="rounded-2xl p-4 space-y-3"
              style={{ backgroundColor: 'var(--color-panel)', border: '1px solid var(--color-border)' }}
            >
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <h3 className="text-sm font-semibold" style={{ color: 'var(--color-text)' }}>
                      {preset.displayName ?? preset.provider}
                    </h3>
                    {providerModels.some(isModelUsable) && (
                      <span className="text-[0.65rem] px-1.5 py-0.5 rounded-full bg-[var(--color-accent)] text-white">已启用</span>
                    )}
                  </div>
                  <div className="flex items-center gap-1.5 mt-1 text-xs" style={{ color: 'var(--color-text-muted)' }}>
                    <span className="truncate">{preset.baseUrl}</span>
                    <a href={preset.baseUrl} target="_blank" rel="noreferrer" className="text-[var(--color-accent)]">
                      <ExternalLink size={12} />
                    </a>
                  </div>
                </div>
                <Switch
                  checked={draft.enabled}
                  onCheckedChange={(checked) => setDrafts((state) => ({
                    ...state,
                    [preset.provider]: { ...draft, enabled: checked },
                  }))}
                  aria-label={`启用 ${preset.displayName ?? preset.provider}`}
                />
              </div>

              <div className="flex items-center gap-2">
                <div className="relative flex-1">
                  <Input
                    type={visibleKeys[preset.provider] ? 'text' : 'password'}
                    value={draft.apiKey}
                    onChange={(event) => setDrafts((state) => ({
                      ...state,
                      [preset.provider]: { ...draft, apiKey: event.target.value },
                    }))}
                    placeholder="sk-..."
                    className="pr-9"
                  />
                  <button
                    type="button"
                    onClick={() => setVisibleKeys((state) => ({ ...state, [preset.provider]: !state[preset.provider] }))}
                    className="absolute right-2.5 top-1/2 -translate-y-1/2 text-[var(--color-text-muted)] hover:text-[var(--color-text)] transition-colors"
                  >
                    {visibleKeys[preset.provider] ? <EyeOff size={14} /> : <Eye size={14} />}
                  </button>
                </div>
                <Button
                  onClick={() => saveProvider(preset)}
                  disabled={savingProvider === preset.provider || (draft.enabled && !draft.apiKey.trim())}
                >
                  {savingProvider === preset.provider ? <Zap size={13} /> : <Save size={13} />}
                  {savingProvider === preset.provider ? '保存中...' : '保存并拉取模型'}
                </Button>
              </div>

              <div className="flex flex-wrap gap-1.5">
                {[...preset.models.map((model) => model.name), ...preset.embeddingModels].map((modelName) => (
                  <span
                    key={modelName}
                    className="inline-flex items-center rounded-full px-2.5 py-1 text-[0.7rem] border"
                    style={{
                      borderColor: 'var(--color-border)',
                      color: 'var(--color-text-secondary)',
                    }}
                  >
                    {modelName}
                  </span>
                ))}
              </div>

              <div className="text-[0.68rem]" style={{ color: 'var(--color-text-muted)' }}>
                {enabled ? `可拉取 ${modelCount} 个预设模型。` : '开关关闭或未填写 Key 时，该服务商模型不会出现在选择列表。'}
              </div>
            </div>
          )
        })}
      </div>
    </div>
  )
}
