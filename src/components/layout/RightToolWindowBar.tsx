import { Sparkles } from 'lucide-react'
import { useLayoutStore } from '../../stores/layout-store'
import { useWorkflowStore } from '../../stores/workflow-store'

/**
 * 右侧工具窗口栏（RightToolWindowBar）
 * JetBrains 风格：30px 宽，纯图标，激活时右侧 2px 竖线
 * 仅保留 AI 工作流输出面板；聊天 Agent 已移除。
 */
export default function RightToolWindowBar() {
  const aiPanelOpen = useLayoutStore(s => s.aiPanelOpen)
  const toggleAIPanel = useLayoutStore(s => s.toggleAIPanel)
  const openRightPanel = useLayoutStore(s => s.openRightPanel)
  const currentRun = useWorkflowStore((s) => s.currentRun)

  /** 工作流活跃时给 AI 输出按钮显示脉冲 */
  const showPulse = currentRun && (currentRun.status === 'running' || currentRun.status === 'waiting')

  const handleClick = () => {
    if (!aiPanelOpen) {
      openRightPanel()
    } else {
      toggleAIPanel()
    }
  }

  return (
    <div
      className="no-select flex flex-col items-center justify-start h-full py-0.5 gap-0.5"
      style={{
        width: 'var(--width-right-bar)',  /* 30px */
        backgroundColor: 'var(--color-activity-bar)',
        borderLeft: '1px solid var(--color-border)',
        flexShrink: 0,
      }}
    >
      {/* AI 输出面板按钮 */}
      <button
        onClick={handleClick}
        title="AI 输出"
        className="tool-btn relative"
        style={{
          height: 30,
          boxShadow: aiPanelOpen
            ? 'inset -2px 0 0 var(--color-activity-indicator)'
            : 'none',
          color: aiPanelOpen
            ? 'var(--color-activity-icon-active)'
            : 'var(--color-activity-icon)',
        }}
      >
        <Sparkles size={15} strokeWidth={aiPanelOpen ? 2 : 1.5} />
        {/* 工作流活跃时的脉冲指示点 */}
        {showPulse && !aiPanelOpen && (
          <span
            className="absolute top-1 right-1 w-1.5 h-1.5 rounded-full animate-pulse"
            style={{ backgroundColor: 'var(--color-accent)' }}
          />
        )}
      </button>
    </div>
  )
}
