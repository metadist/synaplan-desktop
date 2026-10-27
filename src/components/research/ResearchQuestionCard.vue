<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ResearchQuestion } from '@/lib/researchRun'

defineProps<{ question: ResearchQuestion }>()

const emit = defineEmits<{
  answer: [payload: { choice?: string; text?: string }]
  skip: []
}>()

const { t } = useI18n()
const own = ref('')
</script>

<template>
  <section class="card" data-testid="research-question">
    <h2>{{ t('research.questionTitle') }}</h2>
    <p>{{ question.text }}</p>
    <div class="choices">
      <button
        v-for="option in question.options"
        :key="option"
        class="btn btn-ghost"
        type="button"
        @click="emit('answer', { choice: option })"
      >
        {{ option }}
      </button>
    </div>
    <form
      v-if="question.allowText"
      class="own"
      @submit.prevent="own.trim() && emit('answer', { text: own.trim() })"
    >
      <input
        v-model="own"
        class="input"
        type="text"
        :placeholder="t('research.ownAnswer')"
        data-testid="research-own-answer"
      />
      <button class="btn btn-primary" type="submit" :disabled="!own.trim()">
        {{ t('research.answer') }}
      </button>
    </form>
    <button class="btn-link" type="button" data-testid="research-skip" @click="emit('skip')">
      {{ t('research.skip') }}
    </button>
  </section>
</template>

<style scoped>
.card {
  margin: 0.8rem 1.2rem;
  padding: 0.9rem 1rem;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--bg-card);
}
.choices,
.own {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
}
</style>
