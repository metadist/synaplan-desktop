import type { Assistant, AssistantModels, ProjectModels } from '@/services/tauri'

/** A recipe model that is not the project's binding for the same use. */
export interface WorldMismatch {
  /** Project slot id shown in the Models view. */
  slot: 'chat' | 'vision' | 'embed'
  /** The recipe's catalog key, or `null` for "workspace default". */
  recipe: string | null
}

const PAIRS: ReadonlyArray<[WorldMismatch['slot'], keyof AssistantModels]> = [
  ['chat', 'chat'],
  ['vision', 'vision'],
  ['embed', 'vectorize'],
]

/**
 * Compare an Assistant's published models to the project's bindings (C15).
 *
 * The project's models always stay on the wire; this only tells the user that
 * the recipe was written for other models. A recipe key that differs from the
 * project's is a mismatch, and so is a `null` key ("workspace default") when
 * the project made its own choice — that default is not this project. When both
 * sides are empty there is nothing to compare. Returns `[]` when the workspace
 * did not send the recipe models at all (nothing can be said, so nothing is).
 */
export function worldMismatches(assistant: Assistant, models: ProjectModels): WorldMismatch[] {
  const recipe = assistant.models
  if (!recipe) {
    return []
  }
  const out: WorldMismatch[] = []
  for (const [slot, recipeKey] of PAIRS) {
    const theirs = recipe[recipeKey]
    const ours = models[slot]
    if (theirs === null || theirs === '') {
      if (ours !== '') {
        out.push({ slot, recipe: null })
      }
      continue
    }
    if (theirs !== ours) {
      out.push({ slot, recipe: theirs })
    }
  }
  return out
}
