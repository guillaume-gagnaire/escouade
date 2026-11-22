<script lang="ts">
  // « Kanban » (MO-1, direction B): its empty state. MO-7 brings the project chosen with its space, its columns in
  // segments and the gestures of a line.
  import { t } from '$shared/lib/i18n';
  import Empty from './Empty.svelte';
</script>

<Empty icon="kanban" title={t('mobile.kanban.choose')} text={t('mobile.kanban.chooseText')} />
