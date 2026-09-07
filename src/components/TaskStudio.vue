<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { groupTaskCards, type TaskCard, type TaskGroup } from '@/composables/useTaskStudio'

const props = defineProps<{ cards: TaskCard[] }>()

const emit = defineEmits<{
  pick: [card: TaskCard]
  later: []
}>()

const { t } = useI18n()

const sections = computed(() => groupTaskCards(props.cards))

function groupLabel(group: TaskGroup): string {
  switch (group) {
    case 'outlook':
      return t('chat.studio.groupOutlook')
    case 'documents':
      return t('chat.studio.groupDocuments')
    case 'data':
      return t('chat.studio.groupData')
  }
}
</script>

<template>
  <section class="studio" data-testid="task-studio">
    <header class="studio-hero">
      <h2 class="studio-title">{{ t('chat.studio.heroTitle') }}</h2>
      <p class="studio-lead">{{ t('chat.studio.heroLead') }}</p>
    </header>

    <div v-for="section in sections" :key="section.group" class="studio-section">
      <h3 class="studio-group">{{ groupLabel(section.group) }}</h3>
      <div class="studio-grid">
        <button
          v-for="card in section.cards"
          :key="card.id"
          class="studio-card"
          :data-group="section.group"
          :data-task="card.id"
          type="button"
          @click="emit('pick', card)"
        >
          <span class="studio-card-title">{{ t(`chat.studio.cards.${card.id}.title`) }}</span>
          <span class="studio-card-lead">{{ t(`chat.studio.cards.${card.id}.lead`) }}</span>
        </button>
      </div>
    </div>

    <aside class="studio-later">
      <div class="studio-later-copy">
        <h3 class="studio-later-title">{{ t('chat.studio.laterTitle') }}</h3>
        <p class="studio-later-body">{{ t('chat.studio.laterBody') }}</p>
      </div>
      <button class="btn btn-secondary later-cta" type="button" @click="emit('later')">
        {{ t('chat.studio.laterCta') }}
      </button>
    </aside>
  </section>
</template>

<style scoped>
.studio {
  margin: auto;
  width: min(100%, 720px);
  display: flex;
  flex-direction: column;
  gap: 1.35rem;
  padding: 0.4rem 0 0.6rem;
}

.studio-hero {
  padding: 1.15rem 1.25rem;
  border-radius: calc(var(--radius) * 1.2);
  background: var(--studio-hero);
  border: 1px solid var(--studio-hero-border);
}

.studio-title {
  font-size: 1.45rem;
  letter-spacing: -0.03em;
  line-height: 1.2;
}

.studio-lead {
  margin: 0.45rem 0 0;
  color: var(--txt-secondary);
  font-size: 0.95rem;
  max-width: 38rem;
}

.studio-section {
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
}

.studio-group {
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--txt-secondary);
}

.studio-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 0.65rem;
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
  padding: 0.85rem 0.9rem 0.9rem 0.95rem;
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  min-height: 5.4rem;
  box-shadow:
    var(--shadow),
    inset 4px 0 0 var(--studio-stripe);
  transition:
    border-color 0.12s ease,
    transform 0.12s ease,
    box-shadow 0.12s ease;
}

.studio-card[data-group='outlook'] {
  --studio-stripe: var(--studio-stripe-outlook);
}

.studio-card[data-group='documents'] {
  --studio-stripe: var(--studio-stripe-documents);
}

.studio-card[data-group='data'] {
  --studio-stripe: var(--studio-stripe-data);
}

.studio-card:hover {
  border-color: var(--studio-stripe);
  transform: translateY(-1px);
  box-shadow:
    var(--shadow),
    inset 4px 0 0 var(--studio-stripe);
}

.studio-card:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}

.studio-card-title {
  font-size: 0.95rem;
  font-weight: 650;
  letter-spacing: -0.015em;
}

.studio-card-lead {
  font-size: 0.8rem;
  color: var(--txt-secondary);
  line-height: 1.4;
}

.studio-later {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.95rem 1.05rem;
  border-radius: var(--radius);
  background: var(--studio-later-bg);
  border: 1px dashed var(--studio-later-border);
}

.studio-later-title {
  font-size: 0.95rem;
}

.studio-later-body {
  margin: 0.3rem 0 0;
  font-size: 0.82rem;
  color: var(--txt-secondary);
  max-width: 38rem;
}

.later-cta {
  flex-shrink: 0;
}

@media (max-width: 560px) {
  .studio-later {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
