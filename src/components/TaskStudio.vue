<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  STUDIO_TILE_LIMIT,
  cardForSkill,
  hasStudioCopy,
  readySkills,
  toggleStudioPick,
  type SkillFilter,
  type TaskCard,
} from '@/composables/useTaskStudio'

const props = defineProps<{
  cards: TaskCard[]
  skills: SkillFilter[]
}>()

const emit = defineEmits<{
  pick: [card: TaskCard]
  save: [skills: string[]]
}>()

const { t, te } = useI18n()

const choosing = ref(false)
const draft = ref<string[]>([])

const choosable = computed(() => readySkills(props.skills))

watch(choosing, (open) => {
  if (open) {
    draft.value = props.cards.map((c) => c.skill)
  }
})

function titleOf(card: TaskCard): string {
  const key = `chat.studio.cards.${card.id}.title`
  return te(key) ? t(key) : card.skill
}

function leadOf(card: TaskCard): string {
  const key = `chat.studio.cards.${card.id}.lead`
  if (te(key)) {
    return t(key)
  }
  return props.skills.find((s) => s.name === card.skill)?.description || ''
}

function optionCard(skill: SkillFilter): TaskCard {
  return cardForSkill(skill)
}

function toggle(skill: string): void {
  draft.value = toggleStudioPick(draft.value, skill)
}

function save(): void {
  emit('save', [...draft.value])
  choosing.value = false
}

function cardTitleKey(card: TaskCard): string {
  return hasStudioCopy(card) ? `chat.studio.cards.${card.id}.title` : ''
}
</script>

<template>
  <section class="studio" data-testid="task-studio">
    <header class="studio-hero">
      <h2 class="studio-title">{{ t('chat.studio.heroTitle') }}</h2>
      <p class="studio-lead">{{ t('chat.studio.heroLead') }}</p>
    </header>

    <div class="studio-grid">
      <button
        v-for="(card, index) in cards"
        :key="card.skill"
        class="studio-card"
        :data-index="index"
        :data-task="card.id"
        type="button"
        @click="emit('pick', card)"
      >
        <span class="studio-card-title">{{ titleOf(card) }}</span>
        <span class="studio-card-lead">{{ leadOf(card) }}</span>
      </button>
    </div>

    <button
      v-if="choosable.length > 0"
      class="btn btn-ghost choose"
      type="button"
      data-testid="btn-choose-tiles"
      @click="choosing = true"
    >
      {{ t('chat.studio.chooseTiles') }}
    </button>

    <div
      v-if="choosing"
      class="chooser"
      role="dialog"
      aria-modal="true"
      data-testid="tile-chooser"
      @click.self="choosing = false"
    >
      <div class="chooser-card">
        <h3 class="chooser-title">{{ t('chat.studio.chooseTitle') }}</h3>
        <p class="chooser-hint">{{ t('chat.studio.chooseHint') }}</p>
        <p class="chooser-count muted">
          {{ t('chat.studio.chooseCount', { count: draft.length }) }}
        </p>
        <ul class="chooser-list">
          <li v-for="skill in choosable" :key="skill.name">
            <label class="chooser-option" :data-skill="skill.name">
              <input
                type="checkbox"
                :checked="draft.includes(skill.name)"
                :disabled="!draft.includes(skill.name) && draft.length >= STUDIO_TILE_LIMIT"
                @change="toggle(skill.name)"
              />
              <span>
                <span class="chooser-name">{{
                  cardTitleKey(optionCard(skill))
                    ? t(`chat.studio.cards.${optionCard(skill).id}.title`)
                    : skill.name
                }}</span>
                <span class="chooser-lead">{{
                  leadOf(optionCard(skill)) || skill.description
                }}</span>
              </span>
            </label>
          </li>
        </ul>
        <div class="chooser-actions">
          <button class="btn btn-ghost" type="button" @click="choosing = false">
            {{ t('common.cancel') }}
          </button>
          <button class="btn btn-primary" type="button" data-testid="btn-save-tiles" @click="save">
            {{ t('chat.studio.chooseSave') }}
          </button>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.studio {
  margin: auto;
  width: min(100%, 720px);
  display: flex;
  flex-direction: column;
  gap: 1.1rem;
  padding: 0.4rem 0 0.4rem;
}

.studio-hero {
  padding: 1rem 1.15rem;
  border-radius: calc(var(--radius) * 1.2);
  background: var(--studio-hero);
  border: 1px solid var(--studio-hero-border);
}

.studio-title {
  font-size: 1.35rem;
  letter-spacing: -0.03em;
  line-height: 1.2;
}

.studio-lead {
  margin: 0.4rem 0 0;
  color: var(--txt-secondary);
  font-size: 0.92rem;
  max-width: 36rem;
}

.studio-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.7rem;
}

.studio-card {
  appearance: none;
  text-align: left;
  cursor: pointer;
  font: inherit;
  color: var(--txt);
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 1rem 1rem 1.05rem;
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  min-height: 8.2rem;
  box-shadow:
    var(--shadow),
    inset 4px 0 0 var(--studio-stripe);
  transition:
    border-color 0.12s ease,
    transform 0.12s ease;
}

.studio-card[data-index='0'] {
  --studio-stripe: var(--studio-stripe-outlook);
}

.studio-card[data-index='1'] {
  --studio-stripe: var(--studio-stripe-documents);
}

.studio-card[data-index='2'] {
  --studio-stripe: var(--studio-stripe-data);
}

.studio-card:hover {
  border-color: var(--studio-stripe);
  transform: translateY(-1px);
}

.studio-card:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}

.studio-card-title {
  font-size: 1rem;
  font-weight: 650;
  letter-spacing: -0.02em;
}

.studio-card-lead {
  font-size: 0.82rem;
  color: var(--txt-secondary);
  line-height: 1.45;
}

.choose {
  align-self: flex-start;
  font-size: 0.82rem;
  padding: 0.4rem 0.75rem;
}

.chooser {
  position: fixed;
  inset: 0;
  background: color-mix(in srgb, var(--bg) 70%, transparent);
  backdrop-filter: blur(3px);
  display: grid;
  place-items: center;
  padding: 1.5rem;
  z-index: 20;
}

.chooser-card {
  max-width: 440px;
  width: 100%;
  max-height: min(80vh, 560px);
  overflow: auto;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: calc(var(--radius) * 1.4);
  padding: 1.4rem;
  box-shadow: var(--shadow-lg);
}

.chooser-title {
  font-size: 1.1rem;
}

.chooser-hint,
.chooser-count {
  margin: 0.35rem 0 0;
  font-size: 0.85rem;
}

.chooser-list {
  list-style: none;
  margin: 0.9rem 0 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}

.chooser-option {
  display: flex;
  align-items: flex-start;
  gap: 0.6rem;
  padding: 0.55rem 0.6rem;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
}

.chooser-option:has(input:checked) {
  border-color: var(--accent);
  background: var(--accent-soft);
}

.chooser-name {
  display: block;
  font-weight: 600;
  font-size: 0.88rem;
}

.chooser-lead {
  display: block;
  font-size: 0.76rem;
  color: var(--txt-secondary);
  margin-top: 0.15rem;
}

.chooser-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.55rem;
  margin-top: 1.1rem;
}

@media (max-width: 640px) {
  .studio-grid {
    grid-template-columns: 1fr;
  }
}
</style>
