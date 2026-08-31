import type { ModelInfo } from '@/services/tauri'

export interface ModelGroup {
  provider: string
  models: ModelInfo[]
}

// `/v1/models` has no capability field, so we filter by id heuristics: hide
// embeddings, speech, image, video, and vision-only models, plus dev stubs.
const NON_CHAT =
  /(embed|whisper|voxtral|tts|vectorize|bge|image|imagen|flux|sdxl|banana|reve|grok-imagine|veo|video|kling|sound|pic2|piper|emoji|soul|\/dop\/|higgsfield|text-embedding)/i

/** True if the model id looks like a text/chat model (best-effort). */
export function isChatModel(id: string): boolean {
  const lower = id.toLowerCase()
  if (lower.startsWith('stub') || lower.startsWith('test')) {
    return false
  }
  return !NON_CHAT.test(lower)
}

/** Dedupe, keep chat models only, and group by provider (sorted). */
export function chatModelGroups(models: ModelInfo[]): ModelGroup[] {
  const seen = new Set<string>()
  const byProvider = new Map<string, ModelInfo[]>()
  for (const m of models) {
    if (!isChatModel(m.id) || seen.has(m.id)) {
      continue
    }
    seen.add(m.id)
    const list = byProvider.get(m.provider) ?? []
    list.push(m)
    byProvider.set(m.provider, list)
  }
  return [...byProvider.entries()]
    .map(([provider, ms]) => ({
      provider,
      models: ms.sort((a, b) => a.id.localeCompare(b.id)),
    }))
    .sort((a, b) => a.provider.localeCompare(b.provider))
}

/**
 * Pick a reasonable default model. Prefers ids that commonly work across
 * instances; falls back to the first available chat model.
 */
export function defaultChatModel(groups: ModelGroup[]): string {
  const all = groups.flatMap((g) => g.models.map((m) => m.id))
  const prefs = [
    'gpt-4o-mini',
    'claude-haiku',
    'claude-sonnet',
    'grok-4',
    'gpt-oss',
    'mistral',
    'gemini-2.5-flash',
  ]
  for (const p of prefs) {
    const hit = all.find((id) => id.toLowerCase().includes(p))
    if (hit) {
      return hit
    }
  }
  return all[0] ?? ''
}
