<script lang="ts">
  import type { SettingsPage } from '../../lib/settings-pages';
  import Group from './Group.svelte';

  // Group in « Avancé », folded at the foot of a page, around a row of its own.
  let { advanced, hides, title }: { advanced?: SettingsPage; hides?: string[]; title?: string } = $props();
</script>

<Group {advanced} {hides} {title} note="Ce qui sert rarement."><p>la rangée cachée</p></Group>
