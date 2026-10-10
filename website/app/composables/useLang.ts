import { catalogOf } from '~/data/catalogs';
import { langOfPath, type Lang } from '~/data/language';

/** The language of the page being shown, taken from its address (the router's path has no base URL), and its texts. */
export function useLang() {
  const route = useRoute();
  const lang = computed<Lang>(() => langOfPath(route.path));
  const text = computed(() => catalogOf(lang.value));
  return { lang, text };
}
