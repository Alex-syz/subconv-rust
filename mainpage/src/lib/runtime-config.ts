interface RuntimeConfigResponse {
  ok: boolean
  status: number
  json: () => Promise<unknown>
}

type RuntimeConfigFetcher = () => Promise<RuntimeConfigResponse>
type Sleep = (milliseconds: number) => Promise<unknown>

export type RuntimeConfigResult =
  | { ok: true; templates: string[]; selected: string }
  | { ok: false; message: string }

const retryDelays = [500, 1500] as const
const errorMessage = '模板配置加载失败，请刷新页面后重试'

const defaultSleep: Sleep = (milliseconds) => new Promise((resolve) => {
  window.setTimeout(resolve, milliseconds)
})

function parseRuntimeConfig(payload: unknown): RuntimeConfigResult {
  if (!payload || typeof payload !== 'object') {
    return { ok: false, message: errorMessage }
  }

  const config = payload as Record<string, unknown>
  const templates = config.availableTemplates
  if (!Array.isArray(templates) || templates.length === 0 || templates.some((item) => typeof item !== 'string')) {
    return { ok: false, message: errorMessage }
  }

  const defaultTemplate = typeof config.defaultTemplate === 'string' ? config.defaultTemplate : null
  return {
    ok: true,
    templates,
    selected: defaultTemplate && templates.includes(defaultTemplate) ? defaultTemplate : templates[0],
  }
}

export async function loadRuntimeConfig(
  fetcher: RuntimeConfigFetcher = () => fetch('/config'),
  sleep: Sleep = defaultSleep,
): Promise<RuntimeConfigResult> {
  for (let attempt = 0; attempt < 3; attempt += 1) {
    try {
      const response = await fetcher()
      if (!response.ok) {
        throw new Error(`Runtime config request failed with ${response.status}`)
      }

      const result = parseRuntimeConfig(await response.json())
      if (result.ok) {
        return result
      }
      throw new Error('Runtime config payload is invalid')
    }
    catch {
      const delay = retryDelays[attempt]
      if (delay !== undefined) {
        await sleep(delay)
      }
    }
  }

  return { ok: false, message: errorMessage }
}
