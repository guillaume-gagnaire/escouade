<script setup lang="ts">
import { pathIn } from '~/data/language';
import { SUBMIT_TEXTS } from '~/data/submit';

// The submission, as the preview draws it (design/store/, #submit): the steps on the left (repository, version,
// checks, publish), and on the right the step being done. The repository and the tag are picked by plain links and
// forms (they work without script, under the strict Content-Security-Policy); the checks are read again every second
// while they run, and say, check by check, what failed, where, why and how to fix it (docs/STORE.md, §3).
type Status = 'pass' | 'fail' | 'hold' | 'skip' | 'run';
interface Words {
  fr: string;
  en: string;
}
interface Check {
  key: string;
  status: Status;
  detail: Words;
  problems: { path?: string; why: Words; fix: Words }[];
}
interface VersionData {
  id: number;
  state: 'checking' | 'error' | 'rejected' | 'held' | 'ready' | 'published';
  error: string | null;
  origin: string;
  repository: string;
  tag: string;
  commit: string;
  tagDate: string | null;
  pluginId: string | null;
  name: string | null;
  version: string | null;
  archive: { sha256: string; size: number } | null;
  fingerprint: string | null;
  checks: Check[];
  first: boolean;
  previous: string | null;
  plugin: { categories: string[]; keywords: string[]; autoPublish: boolean } | null;
  suggested: { categories: string[]; keywords: string[] };
  review: boolean;
  tagMovedTo: string | null;
}
interface Repo {
  fullName: string;
  private: boolean;
  suspended: boolean;
}
interface SubmitData {
  on: boolean;
  repos: Repo[];
  recent: { id: number; repository: string; tag: string; state: string; pluginId: string | null }[];
}
interface TagsData {
  repository: string;
  tags: { name: string; commit: string; release: boolean; date: string | null }[];
}

const NAMES: Record<string, Words> = {
  manifest: { fr: 'Manifeste', en: 'Manifest' },
  version: { fr: 'Version', en: 'Version' },
  id: { fr: 'Identifiant', en: 'Identifier' },
  repository: { fr: 'Dépôt', en: 'Repository' },
  license: { fr: 'Licence', en: 'License' },
  readme: { fr: 'README', en: 'README' },
  screenshots: { fr: 'Captures', en: 'Screenshots' },
  archive: { fr: 'Archive', en: 'Archive' },
  permissions: { fr: 'Permissions', en: 'Permissions' },
  binaries: { fr: 'Binaires', en: 'Binaries' },
};
const CATEGORIES = ['harness', 'ticket-sources', 'forges', 'themes', 'notifications', 'permissions', 'tools', 'languages'];

const { lang } = useLang();
const route = useRoute();
const text = computed(() => SUBMIT_TEXTS[lang.value]);
const here = computed(() => pathIn(lang.value, '/submit'));
const query = (k: string) => (typeof route.query[k] === 'string' ? (route.query[k] as string) : '');
const versionParam = query('version');
const repoParam = query('repo');
const wantsPublish = query('step') === 'publish';
const notice = computed(() => text.value.notices[query('notice')] ?? null);
const docs = computed(() =>
  lang.value === 'fr' ? 'https://escouade.dev/docs/plugins/publishing' : 'https://escouade.dev/en/docs/plugins/publishing',
);

const { data: me } = await useMe();
const signedIn = computed(() => !!me.value?.account);
const signIn = computed(
  () => `/api/auth/github?to=${encodeURIComponent(versionParam ? `${here.value}?version=${versionParam}` : here.value)}`,
);
const install = computed(() => `/api/auth/github?install=1&to=${encodeURIComponent(here.value)}`);

const mode = versionParam ? 'version' : repoParam ? 'tags' : 'repos';
const version = await useFetch<VersionData>(`/api/submissions/${encodeURIComponent(versionParam)}`, {
  key: `version-${versionParam}`,
  immediate: signedIn.value && mode === 'version',
});
const submit = await useFetch<SubmitData>('/api/submit', { key: 'submit', immediate: signedIn.value && mode === 'repos' });
const tags = await useFetch<TagsData>('/api/submit/tags', {
  key: `tags-${repoParam}`,
  query: { repo: repoParam },
  immediate: signedIn.value && mode === 'tags',
});
const v = computed(() => version.data.value);
const off = computed(() => [version.error.value, submit.error.value, tags.error.value].some((e) => e?.statusCode === 503));

// The checks are read again every second while they run.
let timer: ReturnType<typeof setInterval> | undefined;
onMounted(() => {
  if (v.value?.state !== 'checking') return;
  timer = setInterval(async () => {
    await version.refresh();
    if (v.value?.state !== 'checking') clearInterval(timer);
  }, 1000);
});
onBeforeUnmount(() => clearInterval(timer));

const passed = computed(() => v.value?.checks.filter((c) => c.status === 'pass' || c.status === 'hold').length ?? 0);
const short = (sha: string | null | undefined) => (sha ? sha.slice(0, 7) : '');
const fingerprint = (sha: string) => `sha256:${sha.slice(0, 4)}…${sha.slice(-4)}`;
const day = (iso: string | null) =>
  iso ? new Date(iso).toLocaleDateString(lang.value === 'fr' ? 'fr-FR' : 'en-GB', { day: 'numeric', month: 'short', year: 'numeric' }) : '';
/** A text cut at its backticks: the odd parts are code. */
const ticks = (s: string) => s.split('`');
const step = computed(() => {
  if (mode === 'repos') return 0;
  if (mode === 'tags') return 1;
  return v.value && (wantsPublish || v.value.state === 'published') && v.value.state !== 'rejected' ? 3 : 2;
});
const stepNote = (n: number) => {
  if (n === 0) return v.value?.repository ?? (mode === 'tags' ? repoParam : '');
  if (n === 1) return v.value?.tag ?? '';
  if (n === 2) return v.value ? text.value.stepNotes.checks(passed.value) : '';
  return text.value.stepNotes.publish;
};
const categories = computed(() => v.value?.plugin?.categories ?? v.value?.suggested.categories ?? []);

useHead(() => ({
  title: v.value ? `${text.value.checks.title} · ${v.value.name ?? v.value.repository} ${v.value.tag}` : text.value.repos.title,
  meta: [{ name: 'robots', content: 'noindex' }],
}));
</script>

<template>
  <div v-if="!signedIn" class="wrap guest">
    <h1 class="page-title">{{ text.guest.title }}</h1>
    <p class="lead">{{ text.guest.lead }}</p>
    <a class="btn primary" :href="signIn">{{ text.guest.signIn }}</a>
    <p class="sub small">
      <a :href="docs">{{ text.checks.docs }}</a>
    </p>
  </div>

  <div v-else class="wrap submit-layout">
    <ol class="stepper">
      <li v-for="(s, n) in text.steps" :key="s" :class="{ done: n < step, on: n === step }">
        <span class="d">{{ n + 1 }}</span
        ><span
          >{{ s }}<small>{{ stepNote(n) }}</small></span
        >
      </li>
    </ol>

    <section>
      <p v-if="off" class="notice warn">{{ text.off }}</p>

      <!-- 1. The repository -->
      <template v-else-if="mode === 'repos'">
        <h1 class="page-title">
          {{ text.repos.title }} <span class="more">{{ text.repos.more }}</span>
        </h1>
        <p v-if="notice" class="notice" :class="notice[0]" role="status">{{ notice[1] }}</p>
        <p class="sub lead">{{ text.repos.lead }}</p>
        <ul v-if="submit.data.value?.repos.length" class="choices">
          <li v-for="r in submit.data.value.repos" :key="r.fullName" :class="{ off: r.private || r.suspended }">
            <svg class="i" viewBox="0 0 16 16" aria-hidden="true">
              <circle cx="4.5" cy="3.5" r="1.6" />
              <circle cx="4.5" cy="12.5" r="1.6" />
              <circle cx="11.5" cy="6" r="1.6" />
              <path d="M4.5 5.1V10.9M11.5 7.6C11.5 9.6 8 9.4 4.5 10.9" />
            </svg>
            <span class="mono name">{{ r.fullName }}</span>
            <span class="tag" :class="{ bad: r.private || r.suspended }">{{
              r.suspended ? text.repos.suspended : r.private ? text.repos.private : text.repos.public
            }}</span>
            <span class="grow"></span>
            <a v-if="!r.private && !r.suspended" class="btn sm" :href="`${here}?repo=${encodeURIComponent(r.fullName)}`">{{
              text.repos.pick
            }}</a>
          </li>
        </ul>
        <p v-else class="sub">{{ text.repos.none }}</p>
        <div class="foot-acts">
          <a class="btn" :class="{ primary: !submit.data.value?.repos.length }" :href="install">{{
            submit.data.value?.repos.length ? text.repos.change : text.repos.install
          }}</a>
          <a class="sub" :href="docs">{{ text.checks.docs }}</a>
        </div>
        <template v-if="submit.data.value?.recent.length">
          <h2 class="h2 recent-title">{{ text.repos.recent }}</h2>
          <ul class="recent">
            <li v-for="s in submit.data.value.recent" :key="s.id">
              <a :href="`${here}?version=${s.id}`"
                ><b>{{ s.pluginId ?? s.repository }}</b> <span class="mono">{{ s.tag }}</span></a
              >
              <span class="state" :class="s.state">{{ text.states[s.state] ?? s.state }}</span>
            </li>
          </ul>
        </template>
      </template>

      <!-- 2. The version -->
      <template v-else-if="mode === 'tags'">
        <h1 class="page-title">
          {{ text.tags.title }} <span class="more">{{ text.tags.more(repoParam) }}</span>
        </h1>
        <p v-if="notice" class="notice" :class="notice[0]" role="status">{{ notice[1] }}</p>
        <p class="sub lead">{{ text.tags.lead }}</p>
        <p v-if="tags.error.value" class="notice bad">{{ text.tags.failed }}</p>
        <form v-else-if="tags.data.value?.tags.length" method="post" action="/api/submissions">
          <input type="hidden" name="repo" :value="tags.data.value.repository" />
          <input type="hidden" name="lang" :value="lang" />
          <ul class="choices">
            <li v-for="(t, n) in tags.data.value.tags" :key="t.name">
              <label class="pick-tag">
                <input type="radio" name="tag" :value="t.name" :checked="n === 0" required />
                <span class="mono name">{{ t.name }}</span>
                <span class="mono dim">→ {{ short(t.commit) }}</span>
                <span v-if="t.release" class="tag">{{ text.tags.release }}</span>
                <span class="grow"></span>
                <small v-if="t.date" class="dim">{{ day(t.date) }}</small>
              </label>
            </li>
          </ul>
          <div class="foot-acts">
            <button class="btn primary" type="submit">{{ text.tags.submit }}</button>
            <a class="sub" :href="here">{{ text.checks.change }}</a>
          </div>
        </form>
        <p v-else class="notice warn">{{ text.tags.none }}</p>
      </template>

      <!-- 3 and 4. The checks, then publishing -->
      <template v-else-if="v">
        <h1 class="page-title">
          {{ step === 3 ? text.publish.title : text.checks.title }}
          <span class="more">{{
            step === 3 ? `${v.name ?? v.repository} ${v.tag}` : text.checks.more(v.name ?? v.repository, v.tag)
          }}</span>
        </h1>
        <p v-if="notice" class="notice" :class="notice[0]" role="status">{{ notice[1] }}</p>
        <p v-if="v.tagMovedTo" class="notice warn">{{ text.checks.moved(short(v.tagMovedTo)) }}</p>
        <div class="picked">
          <div class="pick">
            <div class="k">
              {{ text.checks.repo }} <a :href="here">{{ text.checks.change }}</a>
            </div>
            <div class="v">
              <svg class="i" viewBox="0 0 16 16" aria-hidden="true">
                <circle cx="4.5" cy="3.5" r="1.6" />
                <circle cx="4.5" cy="12.5" r="1.6" />
                <circle cx="11.5" cy="6" r="1.6" />
                <path d="M4.5 5.1V10.9M11.5 7.6C11.5 9.6 8 9.4 4.5 10.9" /></svg
              >{{ v.repository }}
            </div>
            <div class="s">{{ v.origin === 'webhook' ? text.checks.fromHook : text.checks.repoNote }}</div>
          </div>
          <div class="pick">
            <div class="k">
              {{ text.checks.version }} <a :href="`${here}?repo=${encodeURIComponent(v.repository)}`">{{ text.checks.change }}</a>
            </div>
            <div class="v">
              <svg class="i" viewBox="0 0 16 16" aria-hidden="true">
                <path d="M2.5 2.5H8L13.5 8L8 13.5L2.5 8Z" />
                <circle cx="5.5" cy="5.5" r="0.8" /></svg
              >{{ v.tag }} <span class="mono commit">→ {{ short(v.commit) }}</span>
            </div>
            <div class="s">
              <template v-if="v.tagDate">{{ text.checks.release(day(v.tagDate)) }} · </template
              ><template v-if="v.archive"
                >{{ text.checks.frozen }} : <span class="mono">{{ fingerprint(v.archive.sha256) }}</span></template
              ><template v-else>{{ text.checks.waiting }}</template>
            </div>
          </div>
        </div>

        <template v-if="step === 2">
          <div class="checks" aria-live="polite">
            <div v-for="c in v.checks" :key="c.key" class="check" :class="c.status">
              <span class="st"
                ><svg v-if="c.status === 'pass'" class="i" viewBox="0 0 16 16" aria-hidden="true">
                  <path d="M3.5 8.5L6.5 11.5L12.5 4.5" /></svg
                ><svg v-else-if="c.status === 'fail'" class="i" viewBox="0 0 16 16" aria-hidden="true">
                  <path d="M4.5 4.5L11.5 11.5M11.5 4.5L4.5 11.5" /></svg
                ><svg v-else-if="c.status === 'hold'" class="i" viewBox="0 0 16 16" aria-hidden="true">
                  <path d="M8 4.5V9M8 11.5V11.6" /></svg
              ></span>
              <div>
                <b class="name">{{ NAMES[c.key]?.[lang] ?? c.key }}</b>
                <span v-if="c.status !== 'fail' && c.detail[lang]" class="x">{{ c.detail[lang] }}</span>
                <div v-for="(p, n) in c.status === 'fail' || c.status === 'hold' ? c.problems : []" :key="n" class="why" :class="c.status">
                  <b
                    ><template v-for="(part, i) in ticks(p.why[lang])" :key="i"
                      ><code v-if="i % 2">{{ part }}</code
                      ><template v-else>{{ part }}</template></template
                    ></b
                  >
                  <span v-if="p.path && !p.why[lang].includes(p.path)" class="mono path">{{ p.path }}</span>
                  <div class="fix">
                    <template v-for="(part, i) in ticks(p.fix[lang])" :key="i"
                      ><code v-if="i % 2">{{ part }}</code
                      ><template v-else>{{ part }}</template></template
                    >
                  </div>
                </div>
              </div>
              <span class="w"></span>
            </div>
          </div>
          <div class="foot-acts">
            <template v-if="v.state === 'ready'">
              <a class="btn primary" :href="`${here}?version=${v.id}&step=publish`">{{ text.checks.continue }}</a>
            </template>
            <template v-else-if="v.state === 'error'">
              <form method="post" :action="`/api/submissions/${v.id}/retry`">
                <input type="hidden" name="lang" :value="lang" />
                <button class="btn primary" type="submit">{{ text.checks.retry }}</button>
              </form>
              <span class="sub">{{ text.checks.errorNote }}</span>
            </template>
            <template v-else>
              <button class="btn primary" disabled>{{ text.checks.continue }}</button>
              <span class="sub">{{
                v.state === 'rejected' ? text.checks.failedNote : v.state === 'held' ? text.checks.heldNote : text.checks.runningNote
              }}</span>
              <a v-if="v.state === 'checking'" class="sub" :href="`${here}?version=${v.id}`">{{ text.checks.refresh }}</a>
            </template>
          </div>
          <p v-if="v.state === 'rejected'" class="sub docs">
            <a :href="docs">{{ text.checks.docs }}</a>
          </p>
        </template>

        <template v-else>
          <div v-if="v.state === 'published'" class="out">
            <p class="notice ok">
              <b>{{ text.publish.done }}</b> {{ text.publish.doneNote }}
            </p>
            <p v-if="v.fingerprint" class="sub">
              {{ text.publish.fingerprint }} : <span class="mono">sha256:{{ v.fingerprint }}</span>
            </p>
            <div class="foot-acts">
              <NuxtLink class="btn" :to="pathIn(lang, '/account')">{{ text.publish.mine }}</NuxtLink>
            </div>
          </div>
          <form v-else class="publish-form" method="post" :action="`/api/submissions/${v.id}/publish`">
            <input type="hidden" name="lang" :value="lang" />
            <p class="sub lead">{{ text.publish.lead }}</p>
            <fieldset v-if="v.first" class="field">
              <legend>{{ text.publish.categories }}</legend>
              <div class="cats">
                <label v-for="c in CATEGORIES" :key="c" class="chip">
                  <input type="checkbox" name="categories" :value="c" :checked="categories.includes(c)" />
                  <span>{{ text.categoryNames[c] }}</span>
                </label>
              </div>
            </fieldset>
            <div v-if="v.first" class="field">
              <label for="keywords">{{ text.publish.keywords }}</label>
              <input id="keywords" class="input" name="keywords" maxlength="200" :value="v.suggested.keywords.join(', ')" />
              <small class="dim">{{ text.publish.keywordsHint }}</small>
            </div>
            <label class="box"><input type="checkbox" name="review" checked /> {{ text.publish.review }}</label>
            <label class="box"
              ><input type="checkbox" name="autoPublish" :checked="v.plugin?.autoPublish ?? false" /> {{ text.publish.auto }}</label
            >
            <p v-if="v.fingerprint" class="sub">
              {{ text.publish.fingerprint }} : <span class="mono">sha256:{{ v.fingerprint }}</span>
            </p>
            <div class="foot-acts">
              <button class="btn primary" type="submit">{{ text.publish.submit }}</button>
              <a class="sub" :href="`${here}?version=${v.id}`">{{ text.steps[2] }}</a>
            </div>
          </form>
        </template>
      </template>

      <p v-else class="notice bad">{{ text.notices['not-found']?.[1] }}</p>
    </section>
  </div>
</template>

<style scoped>
.page-title {
  font-size: clamp(26px, 2.6vw, 34px);
  font-weight: 620;
  letter-spacing: -0.03em;
  line-height: 1.15;
  text-wrap: balance;
}
.page-title .more {
  color: var(--muted);
}
.h2 {
  font-size: 17px;
  font-weight: 620;
  letter-spacing: -0.015em;
}
.sub {
  color: var(--muted);
}
.dim {
  color: var(--dim);
}
.mono {
  font-family: var(--mono);
  font-size: 0.86em;
}
.lead {
  max-width: 64ch;
  margin: 14px 0 0;
  font-size: 14.5px;
}
.small {
  font-size: 13px;
}
.notice {
  margin: 20px 0 0;
  padding: 12px 14px;
  border: 1px solid var(--line-strong);
  border-radius: 10px;
  background: var(--surface);
  font-size: 14px;
}
.notice.ok {
  border-color: color-mix(in oklch, var(--ok) 40%, transparent);
}
.notice.warn {
  border-color: color-mix(in oklch, var(--warn) 45%, transparent);
  background: var(--warn-bg);
}
.notice.bad {
  border-color: color-mix(in oklch, var(--danger) 45%, transparent);
  background: var(--danger-bg);
}
.tag {
  display: inline-flex;
  align-items: center;
  height: 22px;
  padding: 0 8px;
  border: 1px solid var(--line-strong);
  border-radius: 6px;
  font-size: 12px;
  font-weight: 550;
}
.tag.bad {
  border-color: color-mix(in oklch, var(--danger) 45%, transparent);
  background: var(--danger-bg);
  color: var(--danger);
}
.btn.sm {
  height: 30px;
  padding: 0 11px;
  border-radius: 7px;
  font-size: 13px;
  font-weight: 600;
}
.btn[disabled] {
  opacity: 0.45;
  cursor: not-allowed;
}
.grow {
  flex: 1;
}

/* The preview's submission. */
.submit-layout {
  display: grid;
  grid-template-columns: 240px minmax(0, 1fr);
  gap: 56px;
  padding-top: 36px;
}
.submit-layout > * {
  min-width: 0;
}
.stepper {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 4px;
  align-content: start;
}
.stepper li {
  display: grid;
  grid-template-columns: 24px 1fr;
  gap: 12px;
  padding: 10px 10px;
  border-radius: 9px;
  font-size: 14px;
  color: var(--muted);
}
.stepper li small {
  display: block;
  color: var(--dim);
  font-size: 12.5px;
  overflow: hidden;
  text-overflow: ellipsis;
}
.stepper li .d {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  border: 1px solid var(--line-strong);
  display: grid;
  place-items: center;
  font: 11.5px var(--mono);
  color: var(--muted);
}
.stepper li.done .d {
  background: var(--surface-2);
  color: var(--text);
}
.stepper li.on {
  background: var(--surface);
  color: var(--text);
  font-weight: 560;
}
.stepper li.on .d {
  border-color: var(--text);
  color: var(--text);
}
.picked {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  margin: 22px 0 30px;
}
.picked > * {
  min-width: 0;
}
.pick {
  padding: 14px 16px;
  border: 1px solid var(--line);
  border-radius: 12px;
  font-size: 14px;
}
.pick .k {
  color: var(--dim);
  font-size: 12.5px;
  font-weight: 600;
  display: flex;
  justify-content: space-between;
}
.pick .k a {
  color: var(--muted);
  font-weight: 500;
}
.pick .v {
  margin-top: 6px;
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
  overflow-wrap: anywhere;
}
.pick .v .commit {
  color: var(--dim);
  font-weight: 400;
}
.pick .s {
  color: var(--dim);
  font-size: 12.5px;
  margin-top: 2px;
}
.checks {
  border-top: 1px solid var(--line);
}
.check {
  display: grid;
  grid-template-columns: 22px minmax(0, 1fr) auto;
  gap: 12px;
  padding: 12px 4px;
  border-bottom: 1px solid var(--line);
  font-size: 14px;
  align-items: start;
}
.check .name {
  font-weight: 600;
}
.check .st {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  margin-top: 1px;
}
.check .st svg.i {
  width: 12px;
  height: 12px;
  stroke-width: 2.2;
}
.check.pass .st {
  color: var(--ok);
  background: color-mix(in oklch, var(--ok) 14%, transparent);
}
.check.fail .st {
  color: var(--danger);
  background: var(--danger-bg);
}
.check.hold .st {
  color: var(--warn);
  background: var(--warn-bg);
}
.check.skip .st {
  border: 1.5px dashed var(--line-strong);
}
.check.run .st {
  border: 1.5px solid var(--line-strong);
  border-top-color: var(--text);
  animation: spin 0.9s linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
.check .w {
  color: var(--dim);
  font-size: 12.5px;
  white-space: nowrap;
}
.check .x {
  margin-left: 0.3em;
  color: var(--dim);
  font-size: 13px;
}
.why {
  margin-top: 8px;
  padding: 12px 14px;
  border: 1px solid color-mix(in oklch, var(--danger) 35%, transparent);
  border-radius: 10px;
  background: var(--danger-bg);
  font-size: 13.5px;
  color: var(--soft);
  max-width: 60ch;
  overflow-wrap: anywhere;
}
.why b {
  color: var(--danger);
  font-weight: 600;
}
.why.hold {
  border-color: color-mix(in oklch, var(--warn) 40%, transparent);
  background: var(--warn-bg);
}
.why.hold b {
  color: var(--warn);
}
.why .path {
  margin-left: 6px;
  color: var(--dim);
}
.why .fix {
  margin-top: 6px;
  color: var(--muted);
}
.foot-acts {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 14px;
  margin-top: 24px;
}
.foot-acts .sub {
  font-size: 13px;
}
.foot-acts a.sub:hover,
.docs a:hover {
  color: var(--text);
}
.docs {
  margin-top: 12px;
  font-size: 13px;
}
.choices {
  display: grid;
  gap: 8px;
  margin: 22px 0 0;
  padding: 0;
  list-style: none;
}
.choices > li,
.pick-tag {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px;
  font-size: 14px;
}
.choices > li {
  padding: 12px 14px;
  border: 1px solid var(--line);
  border-radius: 12px;
}
.choices > li.off {
  color: var(--muted);
}
.pick-tag {
  width: 100%;
  cursor: pointer;
}
.choices .name {
  font-weight: 600;
}
.recent-title {
  margin-top: 40px;
}
.recent {
  display: grid;
  gap: 0;
  margin: 12px 0 0;
  padding: 0;
  list-style: none;
  border-top: 1px solid var(--line);
}
.recent li {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 4px;
  border-bottom: 1px solid var(--line);
  font-size: 14px;
}
.recent .mono {
  color: var(--dim);
}
.state {
  color: var(--dim);
  font-size: 13px;
}
.state.rejected,
.state.error {
  color: var(--danger);
}
.state.published,
.state.ready {
  color: var(--ok);
}
.state.held {
  color: var(--warn);
}
.publish-form {
  min-width: 0;
  display: grid;
  gap: 18px;
  max-width: 640px;
}
.field {
  display: grid;
  gap: 6px;
  margin: 0;
  padding: 0;
  border: 0;
}
.field > label,
.field > legend {
  padding: 0;
  color: var(--dim);
  font-size: 12.5px;
  font-weight: 600;
}
.cats {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 6px;
}
.chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 32px;
  padding: 0 12px;
  border: 1px solid var(--line-strong);
  border-radius: 99px;
  font-size: 13.5px;
  cursor: pointer;
}
.chip input {
  position: absolute;
  opacity: 0;
  pointer-events: none;
}
.chip {
  position: relative;
}
.chip:has(input:focus-visible) {
  outline: 2px solid var(--brand);
  outline-offset: 2px;
}
.chip:has(input:checked) {
  border-color: var(--brand);
  background: var(--seal-bg);
}
.input {
  height: 36px;
  padding: 0 11px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: var(--surface);
  font-size: 14px;
}
input.input:focus-visible {
  border-color: var(--brand);
  outline: none;
}
.box {
  display: flex;
  align-items: start;
  gap: 10px;
  font-size: 14px;
}
.box input,
.pick-tag input {
  accent-color: var(--brand);
}
.publish-form .mono,
.out .mono {
  overflow-wrap: anywhere;
}
.out {
  display: grid;
  gap: 12px;
}
.guest {
  display: grid;
  justify-items: start;
  gap: 18px;
  max-width: 640px;
  padding-top: 56px;
}
.guest .lead {
  color: var(--soft);
  font-size: 16px;
}
@media (max-width: 1180px) {
  .submit-layout {
    grid-template-columns: 200px minmax(0, 1fr);
    gap: 40px;
  }
}
@media (max-width: 720px) {
  .submit-layout,
  .picked {
    grid-template-columns: 1fr;
  }
  .stepper {
    grid-auto-flow: column;
    grid-auto-columns: minmax(0, 1fr);
  }
  .stepper li {
    grid-template-columns: 1fr;
    justify-items: center;
    text-align: center;
    gap: 4px;
    font-size: 12.5px;
  }
  .stepper li small {
    display: none;
  }
}
@media (prefers-reduced-motion: reduce) {
  .check.run .st {
    animation: none;
  }
}
</style>
