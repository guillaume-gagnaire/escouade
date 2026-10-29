import { langOfPath, type Lang } from '~/data/language';
import { LINKS, TEXTS } from '~/data/text';

/** The language of the page being shown, taken from its address, with its texts and the site's links. */
export function useLang() {
  const route = useRoute();
  const lang = computed<Lang>(() => langOfPath(route.path));
  const text = computed(() => TEXTS[lang.value]);
  const links = computed(() => LINKS[lang.value]);
  return { lang, text, links };
}
