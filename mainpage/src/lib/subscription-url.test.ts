import { describe, expect, it } from 'vitest'

import { buildSubscriptionUrl, type SubscriptionUrlInput } from './subscription-url'

const input = (overrides: Partial<SubscriptionUrlInput> = {}): SubscriptionUrlInput => ({
  origin: 'https://demo.test',
  source: 'https://example.com/sub',
  template: 'general.yaml',
  interval: '',
  standbyEnabled: false,
  standby: '',
  proxyRules: true,
  ...overrides,
})

describe('buildSubscriptionUrl', () => {
  it('builds a minimal same-origin subscription URL', () => {
    expect(buildSubscriptionUrl(input({ source: 'a|b' }))).toEqual({
      ok: true,
      url: 'https://demo.test/sub?url=a%7Cb&template=general.yaml',
    })
  })

  it('trims source and template values', () => {
    expect(buildSubscriptionUrl(input({ source: '  a|b  ', template: ' general.yaml ' }))).toEqual({
      ok: true,
      url: 'https://demo.test/sub?url=a%7Cb&template=general.yaml',
    })
  })

  it('adds every enabled optional parameter', () => {
    expect(buildSubscriptionUrl(input({
      interval: '3600',
      standbyEnabled: true,
      standby: 'standby-a\nstandby-b',
      proxyRules: false,
    }))).toEqual({
      ok: true,
      url: 'https://demo.test/sub?url=https%3A%2F%2Fexample.com%2Fsub&template=general.yaml&interval=3600&urlstandby=standby-a%0Astandby-b&npr=1',
    })
  })

  it('omits an entered standby value when standby mode is disabled', () => {
    const result = buildSubscriptionUrl(input({ standby: 'standby-a' }))
    expect(result.ok && result.url).not.toContain('urlstandby')
  })

  it('rejects an empty source', () => {
    expect(buildSubscriptionUrl(input({ source: '  ' }))).toEqual({
      ok: false,
      field: 'source',
      message: '请输入订阅或分享链接',
    })
  })

  it('rejects a missing template', () => {
    expect(buildSubscriptionUrl(input({ template: null }))).toEqual({
      ok: false,
      field: 'template',
      message: '请选择可用模板',
    })
  })

  it.each(['0', '-1', '1.5', 'abc', '  '])('rejects invalid interval %j', (interval) => {
    expect(buildSubscriptionUrl(input({ interval }))).toEqual({
      ok: false,
      field: 'interval',
      message: '更新间隔必须为正整数',
    })
  })
})
