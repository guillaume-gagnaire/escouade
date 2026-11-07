<script lang="ts">
  import { t } from '../../lib/i18n';
  import Group from './Group.svelte';
  import VectorsModelRow from './VectorsModelRow.svelte';

  // « Recherche par le sens » for every project of this machine (ESC-310: its row was in the retired index's section).
</script>

<Group title={t('vectors.allTitle')} anchor="vectors">
  <VectorsModelRow />
</Group>
