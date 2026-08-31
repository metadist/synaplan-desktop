<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import PlaceholderPanel from '@/components/PlaceholderPanel.vue'
import { DOCS } from '@/constants'

const { t } = useI18n()
</script>

<template>
  <PlaceholderPanel :title="t('doctor.title')" :body="t('doctor.body')" :doc-url="DOCS.tools" />
</template>
