import { invoke as tauriInvoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

interface VelaAPI {
  invoke: (channel: string, ...args: unknown[]) => Promise<unknown>
  on: (channel: string, callback: (...args: unknown[]) => void) => () => void
  once: (channel: string, callback: (...args: unknown[]) => void) => void
  send: (channel: string, ...args: unknown[]) => void
  setZoomLevel: (level: number) => void
  setZoomFactor: (factor: number) => void
  getZoomLevel: () => number
}

function getElectronAPI(): VelaAPI | null {
  return (window as unknown as { velaAPI?: VelaAPI }).velaAPI ?? null
}

async function invokeTauri(channel: string, args: unknown[]) {
  return tauriInvoke('vela_invoke', { channel, args })
}

export const ipc = {
  invoke: async <T = any>(channel: string, ...args: unknown[]): Promise<T> => {
    const electronAPI = getElectronAPI()
    if (electronAPI) return electronAPI.invoke(channel, ...args) as Promise<T>
    return invokeTauri(channel, args) as Promise<T>
  },

  on: <T = any>(channel: string, callback: (data: T) => void): (() => void) => {
    const electronAPI = getElectronAPI()
    if (electronAPI) return electronAPI.on(channel, callback as (...args: unknown[]) => void)

    let unlisten: (() => void) | null = null
    listen<T>(channel, (event) => callback(event.payload)).then((fn) => {
      unlisten = fn
    }).catch(() => {})

    return () => unlisten?.()
  },

  once: <T = any>(channel: string, callback: (data: T) => void) => {
    const off = ipc.on<T>(channel, (data) => {
      off()
      callback(data)
    })
  },

  send: (_channel: string, ..._args: unknown[]) => {},

  get isElectron(): boolean {
    return !!getElectronAPI()
  },

  get isTauri(): boolean {
    return !getElectronAPI() && '__TAURI_INTERNALS__' in window
  },

  setZoomLevel: (_level: number) => {},
  setZoomFactor: (_factor: number) => {},
  getZoomLevel: () => 0,
}
