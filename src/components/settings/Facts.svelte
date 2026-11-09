<script lang="ts">
  import { t } from '../../lib/i18n';
  import { revealHidden } from '../../lib/recipe';
  import { app } from '../../lib/state.svelte';

  // Where a command of a proposal runs: its shell (by its name, or its id when this machine has none such), its folder.
  let { shell, dir }: { shell: string; dir: string } = $props();

  const name = $derived(app.shells.find((s) => s.id === shell)?.label ?? t('settings.fields.shellNotFound', { name: revealHidden(shell) }));
</script>

<dl class="facts">
  <dt>{t('settings.fields.shell')}</dt>
  <dd>{name}</dd>
  <dt>{t('common.folder')}</dt>
  <dd class="mono">{dir}</dd>
</dl>
