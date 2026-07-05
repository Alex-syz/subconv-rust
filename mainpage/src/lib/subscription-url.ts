export type SubscriptionUrlField = 'source' | 'template' | 'interval'

export interface SubscriptionUrlInput {
  origin: string
  source: string
  template: string | null
  interval: string
  standbyEnabled: boolean
  standby: string
  proxyRules: boolean
}

export type SubscriptionUrlResult =
  | { ok: true; url: string }
  | { ok: false; field: SubscriptionUrlField; message: string }

export function buildSubscriptionUrl(input: SubscriptionUrlInput): SubscriptionUrlResult {
  const source = input.source.trim()
  if (!source) {
    return { ok: false, field: 'source', message: '请输入订阅或分享链接' }
  }

  const template = input.template?.trim()
  if (!template) {
    return { ok: false, field: 'template', message: '请选择可用模板' }
  }

  if (input.interval !== '' && !/^[1-9]\d*$/.test(input.interval)) {
    return { ok: false, field: 'interval', message: '更新间隔必须为正整数' }
  }

  const url = new URL('/sub', input.origin)
  url.searchParams.set('url', source)
  url.searchParams.set('template', template)

  if (input.interval) {
    url.searchParams.set('interval', input.interval)
  }
  if (input.standbyEnabled && input.standby.trim()) {
    url.searchParams.set('urlstandby', input.standby.trim())
  }
  if (!input.proxyRules) {
    url.searchParams.set('npr', '1')
  }

  return { ok: true, url: url.toString() }
}
