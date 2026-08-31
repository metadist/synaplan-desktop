<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import PlaceholderPanel from '@/components/PlaceholderPanel.vue'

const { t } = useI18n()
</script>

<template>
  <PlaceholderPanel :title="t('skills.title')" :body="t('skills.body')" />
</template>
