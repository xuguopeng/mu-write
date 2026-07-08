export interface MCPServerConfig {
  id: string
  name: string
  transport: 'stdio' | 'sse'
  command?: string
  args?: string[]
  env?: Record<string, string>
  url?: string
}

export interface MCPConfig {
  mcpServers: Record<string, {
    command?: string
    args?: string[]
    env?: Record<string, string>
    url?: string
  }>
}

export interface MCPToolDesc {
  name: string
  description: string
  inputSchema: Record<string, unknown>
  serverId: string
}

export interface MCPResourceDesc {
  uri: string
  name: string
  description?: string
  mimeType?: string
  serverId: string
}

export type MCPConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'error'