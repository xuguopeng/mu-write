import { getCurrent, onOpenUrl, register } from '@tauri-apps/plugin-deep-link'
import { ipc } from './ipc-client'
import { useLayoutStore } from '../stores/layout-store'

const SCHEME = 'plotforge'

interface DeepLinkRequest {
  channel: string
  args: unknown[]
  project?: string
}

export async function initDeepLinkService() {
  if (!ipc.isTauri) return () => {}

  register(SCHEME).catch(() => {
    // macOS/iOS use bundle registration from tauri.conf.json; runtime register is unsupported there.
  })

  const currentUrls = await getCurrent().catch(() => null)
  if (currentUrls) {
    for (const url of currentUrls) {
      handleDeepLink(url).catch((error) => console.warn('[DeepLink] failed:', error))
    }
  }

  const unlisten = await onOpenUrl((urls) => {
    for (const url of urls) {
      handleDeepLink(url).catch((error) => console.warn('[DeepLink] failed:', error))
    }
  }).catch(() => null)

  return () => {
    if (unlisten) unlisten()
  }
}

async function handleDeepLink(rawUrl: string) {
  const request = parseDeepLink(rawUrl)
  if (!request) return

  if (request.project) {
    const opened = await ipc.invoke<{ success: boolean; error?: string }>('project:open', request.project)
    if (!opened?.success) {
      throw new Error(opened?.error ?? `无法打开项目：${request.project}`)
    }
  }

  const result = await ipc.invoke(request.channel, ...request.args)
  console.log('[DeepLink] call result:', request.channel, result)
  useLayoutStore.getState().openRightPanel()
}

function parseDeepLink(rawUrl: string): DeepLinkRequest | null {
  let url: URL
  try {
    url = new URL(rawUrl)
  } catch {
    return null
  }

  if (url.protocol !== `${SCHEME}:` || url.hostname !== 'call') return null

  const channel = url.searchParams.get('channel')?.trim()
  if (!channel) return null

  return {
    channel,
    args: parseJsonArgs(url.searchParams.get('args')),
    project: url.searchParams.get('project') ?? undefined,
  }
}

function parseJsonArgs(raw: string | null): unknown[] {
  if (!raw?.trim()) return []
  const parsed = JSON.parse(raw) as unknown
  return Array.isArray(parsed) ? parsed : [parsed]
}
