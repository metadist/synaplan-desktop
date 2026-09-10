import { useI18n } from 'vue-i18n'
import type { Project } from '@/services/tauri'

/**
 * The name to show for a project. The built-in first project is stored as
 * `Personal` with `kind: personal` and translated at display time, so a German
 * UI shows "Persönlich" without renaming the folder on disk.
 */
export function useProjectName() {
  const { t } = useI18n()
  return (project: Pick<Project, 'kind' | 'name'> | null | undefined): string => {
    if (!project) {
      return ''
    }
    return project.kind === 'personal' ? t('projects.personal') : project.name
  }
}
