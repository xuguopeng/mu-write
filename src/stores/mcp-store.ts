/**
 * MCP Store — 前端 MCP 状态管理
 *
 * 管理 MCP 服务器的连接状态、可用 Tool 列表、配置加载等。
 * 通过 IPC 与主进程的 mcpManager 通信。
 */

import { create } from 'zustand'
import { ipc } from '../services/ipc-client'
import type {
  MCPServerConfig,
  MCPConnectionStatus,
  MCPToolDesc,
  MCPResourceDesc,
} from '../shared/types/mcp'

interface MCPServerStatus {
  id: string
  name: string
  status: MCPConnectionStatus
  toolCount: number
  error?: string
}

type MCPServerConfigData = MCPServerConfig
type MCPToolData = MCPToolDesc
type MCPResourceData = MCPResourceDesc

// ===== Store 状态 =====

interface MCPState {
  /** 服务器状态列表 */
  servers: MCPServerStatus[]
  /** 所有 MCP Tool */
  tools: MCPToolData[]
  /** 所有 MCP 资源 */
  resources: MCPResourceData[]
  /** 配置文件路径 */
  configPath: string | null
  /** 加载中 */
  loading: boolean
  /** 错误 */
  error: string | null

  // ===== Actions =====
  /** 初始化（加载配置 + 自动连接） */
  init: () => Promise<void>
  /** 刷新服务器状态 */
  refreshStatus: () => Promise<void>
  /** 连接单个服务器 */
  connectServer: (config: MCPServerConfigData) => Promise<void>
  /** 断开单个服务器 */
  disconnectServer: (serverId: string) => Promise<void>
  /** 断开所有服务器 */
  disconnectAll: () => Promise<void>
  /** 刷新 Tool 列表 */
  refreshTools: () => Promise<void>
  /** 保留兼容入口：MCP 工具仅通过面板和 CLI 调用 */
  registerMCPToolsToRegistry: () => void
}

export const useMCPStore = create<MCPState>()((set, get) => ({
  servers: [],
  tools: [],
  resources: [],
  configPath: null,
  loading: false,
  error: null,

  init: async () => {
    set({ loading: true, error: null })
    try {
      // 获取配置文件路径
      const configPath = await ipc.invoke('mcp:get-config-path')
      set({ configPath })

      // 加载配置
      const result = await ipc.invoke('mcp:load-config')
      if (!result.success) {
        set({ loading: false })
        return // 配置文件不存在不是错误
      }

      // 自动连接所有配置的服务器
      for (const config of result.configs as Array<Record<string, unknown>>) {
        try {
          await ipc.invoke('mcp:connect', config)
        } catch (e) {
          console.warn(`[MCP] 连接 ${config.id} 失败:`, e)
        }
      }

      // 刷新状态
      await get().refreshStatus()
      await get().refreshTools()

      set({ loading: false })
    } catch (error) {
      set({ loading: false, error: String(error) })
    }
  },

  refreshStatus: async () => {
    try {
      const servers = await ipc.invoke('mcp:get-servers-status')
      set({ servers: servers as unknown as MCPServerStatus[] })
    } catch (error) {
      console.error('[MCP] 刷新状态失败:', error)
    }
  },

  connectServer: async (config) => {
    const result = await ipc.invoke('mcp:connect', config as unknown as Record<string, unknown>)
    if (!result.success) {
      set({ error: result.error ?? '连接失败' })
      return
    }
    await get().refreshStatus()
    await get().refreshTools()
  },

  disconnectServer: async (serverId) => {
    await ipc.invoke('mcp:disconnect', serverId)
    await get().refreshStatus()
    await get().refreshTools()
  },

  disconnectAll: async () => {
    await ipc.invoke('mcp:disconnect-all')
    set({ servers: [], tools: [], resources: [] })
  },

  refreshTools: async () => {
    try {
      const tools = await ipc.invoke('mcp:list-tools') as unknown[]
      const resources = await ipc.invoke('mcp:list-resources') as unknown[]
      set({ tools: tools as MCPToolData[], resources: resources as MCPResourceData[] })
    } catch {
      // 静默处理
    }
  },

  registerMCPToolsToRegistry: () => {},
}))
