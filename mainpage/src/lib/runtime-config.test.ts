import { describe, expect, it, vi } from 'vitest'

import { loadRuntimeConfig } from './runtime-config'

const response = (body: unknown, ok = true) => ({
  ok,
  status: ok ? 200 : 503,
  json: async () => body,
})

describe('loadRuntimeConfig', () => {
  it('selects the configured default when it is available', async () => {
    const fetcher = vi.fn(async () => response({
      defaultTemplate: 'meta.yaml',
      availableTemplates: ['general.yaml', 'meta.yaml'],
    }))

    await expect(loadRuntimeConfig(fetcher)).resolves.toEqual({
      ok: true,
      templates: ['general.yaml', 'meta.yaml'],
      selected: 'meta.yaml',
    })
  })

  it('falls back to the first template when the default is unavailable', async () => {
    const fetcher = vi.fn(async () => response({
      defaultTemplate: 'missing.yaml',
      availableTemplates: ['general.yaml', 'meta.yaml'],
    }))

    const result = await loadRuntimeConfig(fetcher)
    expect(result).toEqual({
      ok: true,
      templates: ['general.yaml', 'meta.yaml'],
      selected: 'general.yaml',
    })
  })

  it('retries transient failures using the specified delays', async () => {
    let attempts = 0
    const fetcher = vi.fn(async () => {
      attempts += 1
      if (attempts < 3) throw new Error('offline')
      return response({ availableTemplates: ['general.yaml'] })
    })
    const sleep = vi.fn(async () => undefined)

    const result = await loadRuntimeConfig(fetcher, sleep)

    expect(result).toEqual({ ok: true, templates: ['general.yaml'], selected: 'general.yaml' })
    expect(fetcher).toHaveBeenCalledTimes(3)
    expect(sleep.mock.calls).toEqual([[500], [1500]])
  })

  it.each([
    { availableTemplates: [] },
    { availableTemplates: [42] },
    { defaultTemplate: 'general.yaml' },
  ])('rejects malformed payload %# after three attempts', async (body) => {
    const fetcher = vi.fn(async () => response(body))
    const sleep = vi.fn(async () => undefined)

    const result = await loadRuntimeConfig(fetcher, sleep)

    expect(result).toEqual({ ok: false, message: '模板配置加载失败，请刷新页面后重试' })
    expect(fetcher).toHaveBeenCalledTimes(3)
    expect(sleep.mock.calls).toEqual([[500], [1500]])
  })

  it('treats unsuccessful HTTP responses as retryable failures', async () => {
    const fetcher = vi.fn(async () => response({}, false))
    const sleep = vi.fn(async () => undefined)

    await expect(loadRuntimeConfig(fetcher, sleep)).resolves.toEqual({
      ok: false,
      message: '模板配置加载失败，请刷新页面后重试',
    })
    expect(fetcher).toHaveBeenCalledTimes(3)
  })
})
