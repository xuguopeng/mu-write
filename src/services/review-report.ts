/** 审稿问题条目（JSON 格式） */
export interface ReviewIssue {
  category: string
  severity: 'error' | 'warning' | 'pass'
  description: string
  /** 引用的原文片段（有问题时提供） */
  quote?: string
}

/** AI 返回的 JSON 审稿结构 */
interface ReviewJSON {
  items: Array<{
    category: string
    severity: string
    description: string
    quote?: string
  }>
  summary: string
}

export interface ParsedReviewReport {
  issues: ReviewIssue[]
  summary: string
  parseFailed: boolean
}

/** 标准化 severity 值 */
export function normalizeSeverity(raw: string): ReviewIssue['severity'] {
  const s = raw.toLowerCase().trim()
  if (s === 'error' || s === 'critical' || s === 'severe') return 'error'
  if (s === 'warning' || s === 'warn' || s === 'minor') return 'warning'
  return 'pass'
}

/** 尝试从文本中提取 JSON（兼容 ```json 包裹） */
export function extractJSON(text: string): string | null {
  const trimmed = text.trim()
  if (trimmed.startsWith('{')) return trimmed

  const codeBlockMatch = trimmed.match(/```(?:json)?\s*\n?([\s\S]*?)```/)
  if (codeBlockMatch) return codeBlockMatch[1].trim()

  const firstBrace = trimmed.indexOf('{')
  const lastBrace = trimmed.lastIndexOf('}')
  if (firstBrace !== -1 && lastBrace > firstBrace) {
    return trimmed.slice(firstBrace, lastBrace + 1)
  }

  return null
}

/** 解析审稿报告（优先 JSON，回退到旧版文本解析） */
export function parseReviewReport(text: string): ParsedReviewReport {
  const jsonStr = extractJSON(text)
  if (jsonStr) {
    try {
      const data = JSON.parse(jsonStr) as ReviewJSON
      if (data.items && Array.isArray(data.items)) {
        const issues: ReviewIssue[] = data.items.map(item => ({
          category: item.category || '综合检查',
          severity: normalizeSeverity(item.severity),
          description: item.description || '',
          quote: item.quote || undefined,
        }))
        return { issues, summary: data.summary || '', parseFailed: false }
      }
    } catch {
      return { issues: [], summary: '解析失败', parseFailed: true }
    }
  }

  return { ...parseLegacyReport(text), parseFailed: false }
}

/** 旧版文本解析器（兼容历史审稿报告） */
function parseLegacyReport(text: string): { issues: ReviewIssue[]; summary: string } {
  const issues: ReviewIssue[] = []
  const lines = text.split('\n')
  let currentCategory = '综合检查'
  const summaryLines: string[] = []
  let inSummary = false

  for (const line of lines) {
    const trimmed = line.trim()
    if (!trimmed) continue

    const headingMatch = trimmed.match(/^#{2,3}\s+(.+)/)
    if (headingMatch) {
      const heading = headingMatch[1].replace(/[*_]/g, '')
      if (/总体评价|总结|总评/.test(heading)) {
        inSummary = true
      } else {
        inSummary = false
        currentCategory = heading
      }
      continue
    }

    if (inSummary) {
      summaryLines.push(trimmed.replace(/^[-*]\s*/, ''))
      continue
    }

    let severity: ReviewIssue['severity'] = 'pass'
    if (trimmed.includes('🔴')) severity = 'error'
    else if (trimmed.includes('🟡')) severity = 'warning'
    else if (trimmed.includes('🟢') || trimmed.includes('✅')) severity = 'pass'
    else if (trimmed.startsWith('-') || trimmed.startsWith('*')) severity = 'warning'
    else continue

    const cleanDesc = trimmed
      .replace(/^[-*]\s*/, '')
      .replace(/[🔴🟡🟢✅]\s*/u, '')
      .replace(/\*\*/g, '')

    if (cleanDesc) {
      issues.push({ category: currentCategory, severity, description: cleanDesc })
    }
  }

  return { issues, summary: summaryLines.join(' ') }
}

export function reviewNeedsRefine(reportText: string): {
  needsRefine: boolean
  reason: 'parse_failed' | 'issues_found' | 'passed'
  errorCount: number
  warningCount: number
  passCount: number
} {
  const parsed = parseReviewReport(reportText)
  const errorCount = parsed.issues.filter(i => i.severity === 'error').length
  const warningCount = parsed.issues.filter(i => i.severity === 'warning').length
  const passCount = parsed.issues.filter(i => i.severity === 'pass').length

  if (parsed.parseFailed || parsed.summary.includes('解析失败')) {
    return { needsRefine: true, reason: 'parse_failed', errorCount, warningCount, passCount }
  }
  if (errorCount > 0 || warningCount > 0) {
    return { needsRefine: true, reason: 'issues_found', errorCount, warningCount, passCount }
  }
  return { needsRefine: false, reason: 'passed', errorCount, warningCount, passCount }
}
