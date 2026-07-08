import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import {
  X, Check, Save, Globe, Cpu,
  Type, Settings2, ChevronDown, MessageSquare, SlidersHorizontal,
} from 'lucide-react'
import PromptSettings from './PromptSettings'
import PresetProviderSettings from './PresetProviderSettings'
import { useLLMStore, isUsableModel } from '../../stores/llm-store'
import { useThemeStore, FONT_OPTIONS, type FontId } from '../../stores/theme-store'
import { Button } from '../ui/Button'
import { Input } from '../ui/Input'
import { Label } from '../ui/Label'
import { NativeSelect } from '../ui/NativeSelect'
import { cn } from '../../lib/utils'
import { ipc } from '../../services/ipc-client'
import { Switch } from '../ui/Switch'

const appVersion = import.meta.env.VITE_APP_VERSION ?? '1.0.0'

type SettingsSection = 'llm' | 'model-config' | 'proxy' | 'editor' | 'prompts' | 'about'

interface SectionItem {
  id: SettingsSection
  label: string
  icon: ReactNode
  description: string
}

const SECTIONS: SectionItem[] = [
  { id: 'llm', label: '模型', icon: <Cpu size={16} />, description: '配置各个服务商的 API Key、开关和预设模型' },
  { id: 'model-config', label: '模型配置', icon: <SlidersHorizontal size={16} />, description: '选择默认 AI 生成模型和默认向量模型' },
  { id: 'proxy', label: '网络代理', icon: <Globe size={16} />, description: '配置 HTTP / SOCKS5 代理，用于访问受限 API' },
  { id: 'editor', label: '编辑器', icon: <Type size={16} />, description: '字体大小、自动保存等编辑器偏好设置' },
  { id: 'prompts', label: '提示词模板', icon: <MessageSquare size={16} />, description: '自定义 AI 创作各环节使用的提示词模板' },
  { id: 'about', label: '关于', icon: <Settings2 size={16} />, description: '应用信息' },
]

interface SettingsModalProps {
  open: boolean
  onClose: () => void
}

export default function SettingsModal({ open, onClose }: SettingsModalProps) {
  const [section, setSection] = useState<SettingsSection>('llm')

  if (!open) return null

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center"
      style={{ backgroundColor: 'rgba(0,0,0,0.6)', backdropFilter: 'blur(4px)' }}
      onClick={(e) => e.target === e.currentTarget && onClose()}
    >
      <div
        className="relative flex w-[880px] h-[600px] rounded-2xl overflow-hidden shadow-2xl"
        style={{
          backgroundColor: 'var(--color-editor-bg)',
          border: '1px solid var(--color-border)',
        }}
      >
        <aside
          className="flex flex-col w-52 flex-shrink-0 py-5 gap-1"
          style={{
            backgroundColor: 'var(--color-sidebar)',
            borderRight: '1px solid var(--color-border)',
          }}
        >
          <div className="flex items-center gap-2 px-4 mb-4">
            <Settings2 size={16} style={{ color: 'var(--color-accent)' }} />
            <span className="text-sm font-semibold" style={{ color: 'var(--color-text)' }}>
              设置
            </span>
          </div>

          {SECTIONS.map((s) => (
            <button
              key={s.id}
              onClick={() => setSection(s.id)}
              className={cn(
                'flex items-center gap-2.5 mx-2 px-3 py-2.5 rounded-lg text-left text-sm transition-colors',
                section === s.id
                  ? 'bg-[var(--color-accent)] text-white'
                  : 'text-[var(--color-text-secondary)] hover:bg-[var(--color-hover)] hover:text-[var(--color-text)]',
              )}
            >
              {s.icon}
              {s.label}
            </button>
          ))}
        </aside>

        <main className="flex-1 flex flex-col overflow-hidden">
          <div
            className="flex items-center justify-between px-6 py-4 flex-shrink-0"
            style={{ borderBottom: '1px solid var(--color-border)' }}
          >
            <div>
              <h2 className="text-base font-semibold" style={{ color: 'var(--color-text)' }}>
                {SECTIONS.find((s) => s.id === section)?.label}
              </h2>
              <p className="text-xs mt-0.5" style={{ color: 'var(--color-text-muted)' }}>
                {SECTIONS.find((s) => s.id === section)?.description}
              </p>
            </div>
            <button
              onClick={onClose}
              className="flex items-center justify-center w-7 h-7 rounded-lg transition-colors hover:bg-[var(--color-hover)]"
              style={{ color: 'var(--color-text-muted)' }}
            >
              <X size={16} />
            </button>
          </div>

          <div className="flex-1 overflow-y-auto px-6 py-5">
            {section === 'llm' && <PresetProviderSettings />}
            {section === 'model-config' && <ModelConfigSection />}
            {section === 'proxy' && <ProxySection />}
            {section === 'editor' && <EditorSection />}
            {section === 'prompts' && <PromptSettings />}
            {section === 'about' && <AboutSection />}
          </div>
        </main>
      </div>
    </div>
  )
}

function ModelConfigSection() {
  const models = useLLMStore((s) => s.models)
  const defaultModelId = useLLMStore((s) => s.defaultModelId)
  const defaultEmbeddingModelId = useLLMStore((s) => s.defaultEmbeddingModelId)
  const loaded = useLLMStore((s) => s.loaded)
  const loadModels = useLLMStore((s) => s.loadModels)
  const setDefaultModel = useLLMStore((s) => s.setDefaultModel)
  const setDefaultEmbeddingModel = useLLMStore((s) => s.setDefaultEmbeddingModel)

  useEffect(() => {
    if (!loaded) loadModels()
  }, [loaded, loadModels])

  const generationModels = useMemo(
    () => models.filter((model) => isUsableModel(model)),
    [models],
  )
  const embeddingModels = useMemo(
    () => models.filter((model) => isUsableModel(model, true)),
    [models],
  )

  return (
    <div className="max-w-[560px] space-y-5">
      <div className="rounded-xl px-4 py-3 text-xs" style={{ backgroundColor: 'var(--color-hover)', color: 'var(--color-text-muted)' }}>
        这里只选择默认模型。可选项来自“模型”页中已启用且已填写 API Key 的服务商。
      </div>

      <ModelSelectCard
        title="AI 生成模型"
        description="用于章节生成、改写、摘要和知识库向量化。"
        value={defaultModelId ?? ''}
        models={generationModels}
        emptyText="暂无可用 AI 生成模型，请先在“模型”页启用服务商并填写 API Key。"
        onChange={(modelId) => setDefaultModel(modelId || null)}
      />

      <ModelSelectCard
        title="向量模型"
        description="用于知识库检索和内容向量化。"
        value={defaultEmbeddingModelId ?? ''}
        models={embeddingModels}
        emptyText="暂无可用向量模型，请先在“模型”页启用包含向量模型的服务商。"
        onChange={(modelId) => setDefaultEmbeddingModel(modelId || null)}
      />
    </div>
  )
}

function ModelSelectCard({
  title,
  description,
  value,
  models,
  emptyText,
  onChange,
}: {
  title: string
  description: string
  value: string
  models: Array<{ id: string; name: string; provider: string; modelName: string }>
  emptyText: string
  onChange: (modelId: string) => void
}) {
  const selectedValue = models.some((model) => model.id === value) ? value : ''

  return (
    <div
      className="space-y-3 p-4 rounded-xl"
      style={{ border: '1px solid var(--color-border)', backgroundColor: 'var(--color-panel)' }}
    >
      <div>
        <p className="text-sm font-medium" style={{ color: 'var(--color-text)' }}>{title}</p>
        <p className="text-xs mt-0.5" style={{ color: 'var(--color-text-muted)' }}>{description}</p>
      </div>

      <NativeSelect
        value={selectedValue}
        onChange={(event) => onChange(event.target.value)}
        disabled={models.length === 0}
        className="h-9"
      >
        <option value="">{models.length === 0 ? '暂无可选模型' : '请选择模型'}</option>
        {models.map((model) => (
          <option key={model.id} value={model.id}>
            {model.name || `${model.provider} · ${model.modelName}`}
          </option>
        ))}
      </NativeSelect>

      <p className="text-[0.68rem]" style={{ color: 'var(--color-text-muted)' }}>
        {models.length === 0 ? emptyText : `当前可选 ${models.length} 个模型。`}
      </p>
    </div>
  )
}

function ProxySection() {
  const [proxy, setProxy] = useState<{ enabled: boolean; type: 'http' | 'socks5'; host: string; port: number }>({
    enabled: false,
    type: 'http',
    host: '',
    port: 7890,
  })
  const [saving, setSaving] = useState(false)
  const [saved, setSaved] = useState(false)

  useEffect(() => {
    ipc.invoke('config:get').then((cfg) => {
      if (cfg?.proxy) {
        setProxy({
          enabled: cfg.proxy.enabled ?? false,
          type: cfg.proxy.type ?? 'http',
          host: cfg.proxy.host ?? '',
          port: cfg.proxy.port ?? 7890,
        })
      }
    }).catch(() => { })
  }, [])

  const handleSave = async () => {
    setSaving(true)
    await ipc.invoke('config:set', { proxy }).catch(() => { })
    setSaving(false)
    setSaved(true)
    setTimeout(() => setSaved(false), 2000)
  }

  return (
    <div className="max-w-[480px] space-y-5">
      <div
        className="flex items-center justify-between p-4 rounded-xl"
        style={{ border: '1px solid var(--color-border)', backgroundColor: 'var(--color-panel)' }}
      >
        <div>
          <p className="text-sm font-medium" style={{ color: 'var(--color-text)' }}>启用代理</p>
          <p className="text-xs mt-0.5" style={{ color: 'var(--color-text-muted)' }}>
            所有 AI API 请求将通过代理发送
          </p>
        </div>
        <Switch
          checked={proxy.enabled}
          onCheckedChange={(checked) => setProxy({ ...proxy, enabled: checked })}
          aria-label="启用代理"
        />
      </div>

      {proxy.enabled && (
        <div
          className="space-y-3 p-4 rounded-xl"
          style={{ border: '1px solid var(--color-border)', backgroundColor: 'var(--color-panel)' }}
        >
          <div>
            <Label>代理类型</Label>
            <NativeSelect
              value={proxy.type}
              onChange={(e) => setProxy({ ...proxy, type: e.target.value as 'http' | 'socks5' })}
            >
              <option value="http">HTTP</option>
              <option value="socks5">SOCKS5</option>
            </NativeSelect>
          </div>
          <div className="grid grid-cols-[1fr_120px] gap-3">
            <div>
              <Label>主机地址</Label>
              <Input
                value={proxy.host}
                onChange={(e) => setProxy({ ...proxy, host: e.target.value })}
                placeholder="127.0.0.1"
              />
            </div>
            <div>
              <Label>端口</Label>
              <Input
                type="number"
                value={proxy.port}
                onChange={(e) => setProxy({ ...proxy, port: (e.target.value === '' ? '' : parseInt(e.target.value, 10)) as number })}
                onBlur={() => {
                  const v = Number(proxy.port)
                  if (!v) setProxy({ ...proxy, port: 7890 })
                }}
              />
            </div>
          </div>
        </div>
      )}

      <Button onClick={handleSave} disabled={saving}>
        {saved ? <Check size={13} /> : <Save size={13} />}
        {saved ? '已保存' : saving ? '保存中...' : '保存代理配置'}
      </Button>
    </div>
  )
}

function FontSelect({
  value,
  onChange,
}: {
  value: FontId
  onChange: (id: FontId) => void
}) {
  const [open, setOpen] = useState(false)
  const ref = useRef<HTMLDivElement>(null)
  const current = FONT_OPTIONS.find((o) => o.id === value) ?? FONT_OPTIONS[0]

  useEffect(() => {
    if (!open) return
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false)
      }
    }
    document.addEventListener('mousedown', handler)
    return () => document.removeEventListener('mousedown', handler)
  }, [open])

  return (
    <div ref={ref} className="relative">
      <button
        onClick={() => setOpen((v) => !v)}
        className="flex items-center gap-2 w-full px-3 h-9 rounded-lg transition-colors text-left"
        style={{
          border: '1px solid var(--color-border)',
          backgroundColor: open ? 'var(--color-hover)' : 'var(--color-panel)',
          color: 'var(--color-text)',
        }}
      >
        <span className="flex-1 text-sm truncate" style={{ fontFamily: current.family }}>
          {current.label}
        </span>
        <span className="text-xs flex-shrink-0" style={{ color: 'var(--color-text-muted)' }}>
          {current.preview}
        </span>
        <ChevronDown
          size={13}
          className="flex-shrink-0 transition-transform"
          style={{
            color: 'var(--color-text-muted)',
            transform: open ? 'rotate(180deg)' : 'rotate(0deg)',
          }}
        />
      </button>

      {open && (
        <div
          className="absolute left-0 right-0 top-[calc(100%+4px)] z-50 rounded-xl overflow-hidden"
          style={{
            border: '1px solid var(--color-border)',
            backgroundColor: 'var(--color-panel)',
            boxShadow: 'var(--shadow-lg)',
          }}
        >
          {FONT_OPTIONS.map((opt) => (
            <button
              key={opt.id}
              onClick={() => { onChange(opt.id); setOpen(false) }}
              className="w-full text-left px-3 py-2.5 flex items-center gap-3 transition-colors hover:bg-[var(--color-hover)]"
              style={{
                backgroundColor: value === opt.id
                  ? 'color-mix(in srgb, var(--color-accent) 8%, transparent)'
                  : 'transparent',
              }}
            >
              <span
                className="w-3.5 h-3.5 rounded-full flex-shrink-0 flex items-center justify-center"
                style={{
                  backgroundColor: value === opt.id ? 'var(--color-accent)' : 'transparent',
                  border: value === opt.id ? 'none' : '1.5px solid var(--color-border)',
                }}
              >
                {value === opt.id && <span className="w-1.5 h-1.5 rounded-full bg-white" />}
              </span>

              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-1.5">
                  <span className="text-xs font-medium" style={{ color: 'var(--color-text)', fontFamily: opt.family }}>
                    {opt.label}
                  </span>
                  <span className="text-[0.65rem]" style={{ color: 'var(--color-text-muted)' }}>
                    {opt.labelEn}
                  </span>
                </div>
                <p className="text-[0.65rem] truncate mt-0.5" style={{ color: 'var(--color-text-muted)' }}>
                  {opt.desc}
                </p>
              </div>

              <span
                className="text-sm flex-shrink-0"
                style={{ fontFamily: opt.family, color: 'var(--color-text-secondary)' }}
              >
                {opt.preview}
              </span>
            </button>
          ))}
        </div>
      )}
    </div>
  )
}

function EditorSection() {
  const { writingFont, setWritingFont, uiFont, setUiFont } = useThemeStore()

  return (
    <div className="max-w-md space-y-5">
      <div className="space-y-1.5">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-xs font-semibold" style={{ color: 'var(--color-text)' }}>界面字体</p>
            <p className="text-[0.68rem] mt-0.5" style={{ color: 'var(--color-text-muted)' }}>
              左侧栏、菜单、对话框等 UI 区域
            </p>
          </div>
        </div>
        <FontSelect value={uiFont} onChange={setUiFont} />
      </div>

      <div className="space-y-1.5">
        <div>
          <p className="text-xs font-semibold" style={{ color: 'var(--color-text)' }}>写作字体</p>
          <p className="text-[0.68rem] mt-0.5" style={{ color: 'var(--color-text-muted)' }}>
            草稿、终稿、架构文档等正文区域
          </p>
        </div>
        <FontSelect value={writingFont} onChange={setWritingFont} />
      </div>

      <div
        className="flex items-start gap-2 px-3 py-2.5 rounded-lg text-xs"
        style={{ backgroundColor: 'var(--color-hover)', color: 'var(--color-text-muted)' }}
      >
        <span className="flex-shrink-0 mt-0.5" style={{ color: 'var(--color-text-muted)' }}>提示</span>
        <span>所有字体已内置在应用中，无需网络连接，切换后立即生效。</span>
      </div>
    </div>
  )
}

function AboutSection() {
  return (
    <div className="space-y-6 max-w-[600px] p-2">
      <div className="flex flex-col items-center justify-center py-8 rounded-xl space-y-2" style={{ backgroundColor: 'var(--color-sidebar)', border: '1px solid var(--color-border)' }}>
        <h1 className="text-2xl font-bold brand-gradient tracking-wider">爆文工坊</h1>
        <p className="text-sm opacity-80" style={{ color: 'var(--color-text)' }}>v{appVersion}</p>
        <p className="text-xs mt-2" style={{ color: 'var(--color-text-muted)' }}>为网文作者打造的 AI 连载生产工作台</p>
      </div>

      <div className="space-y-4 pt-2">
        <h3 className="text-sm font-semibold pb-2" style={{ borderBottom: '1px solid var(--color-border)', color: 'var(--color-text)' }}>
          应用信息
        </h3>
        <p className="text-xs leading-relaxed" style={{ color: 'var(--color-text-muted)' }}>
          这里保留版本、构建信息和基础说明。
        </p>
      </div>
    </div>
  )
}
