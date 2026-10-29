<script setup lang="ts">
import { pathIn } from '~/data/language';

// A plugin's page (design/store/, #plugin; docs/STORE.md §6): its name, author and trust, « Ouvrir dans Escouade »
// (the deep link of STORE-11, which opens its page in the app and never installs anything) and its repository;
// its README and documentation (rendered at its check and cut down, server/utils/readme.ts), its versions; on the
// right what it will be able to do in words, the review, where it comes from (repository, commit, fingerprint,
// signature), the facts, and « Signaler ». Escouade's own have the same page, with the mark « Escouade ».
const route = useRoute();
const { lang, words, to } = useCatalog();
const id = computed(() => String(route.params.id ?? ''));
const { data } = await useFetch(() => `/api/catalog/plugins/${encodeURIComponent(id.value)}`, { key: `plugin-${id.value}` });
if (!data.value) throw createError({ statusCode: 404, statusMessage: 'No such plugin', fatal: true });
const page = computed(() => data.value!);
const e = computed(() => page.value.entry);
useHead(() => ({ title: e.value.name[lang.value] }));

const tab = computed(() => (['docs', 'versions'].includes(String(route.query.tab)) ? String(route.query.tab) : 'readme'));
const tabLink = (t: string) => ({ query: t === 'readme' ? {} : { tab: t } });
const docs = computed(() => page.value.latest?.docs ?? null);
const other = computed(() => (lang.value === 'fr' ? 'en' : 'fr'));
const readme = computed(() => docs.value?.readme[lang.value] ?? docs.value?.readme[other.value] ?? null);
const docPage = computed(() => {
  const pages = docs.value?.pages ?? [];
  return pages.find((p) => p.path === route.query.page) ?? pages[0] ?? null;
});
const short = (sha: string) => sha.slice(0, 7);
const fingerprint = (sha: string) => `sha256:${sha.slice(0, 4)}…${sha.slice(-4)}`;
const deepLink = computed(() => `escouade://plugin/${encodeURIComponent(e.value.id)}`);
const repoUrl = computed(() => (page.value.latest ? `https://github.com/${page.value.latest.repository}` : null));
const SYSTEMS: Record<string, string> = { windows: 'Windows', macos: 'macOS', linux: 'Linux' };
const systems = computed(() => {
  const s = page.value.latest?.systems ?? [];
  const names = [...new Set(s.map((p) => SYSTEMS[p.split('-')[0]!] ?? p))];
  return names.length ? names.join(', ') : 'Windows, macOS, Linux';
});
const hint = computed(() =>
  page.value.builtin ? words.value.plugin.builtinHint : `Escouade ${page.value.latest?.escouade ?? ''} · ${systems.value}`,
);
const copied = ref(false);
async function copyRepo() {
  if (!repoUrl.value) return;
  await navigator.clipboard?.writeText(repoUrl.value).catch(() => {});
  copied.value = true;
  setTimeout(() => (copied.value = false), 1600);
}
</script>

<template>
  <div class="wrap">
    <section class="plug-head">
      <PluginTile :mark="e.mark" :hue="e.hue" size="lg" />
      <div class="who">
        <h1>{{ e.name[lang] }} <TrustMark :trust="e.trust" :version="e.version" large /></h1>
        <div class="line">
          <NuxtLink v-if="e.builtin" class="me" :to="to.author(e)!"><LogoMark :size="22" />Escouade</NuxtLink>
          <NuxtLink v-else-if="e.author" class="me" :to="to.author(e)!"
            ><AuthorAvatar :login="e.author.login" :github-id="e.author.githubId" />{{ e.author.login }}</NuxtLink
          >
          <span v-else>{{ words.deleted }}</span>
          <NuxtLink v-for="c in e.categories" :key="c" :to="to.search({ cat: c })">{{ words.categories[c] }}</NuxtLink>
          <span v-if="e.version" class="mono">v{{ e.version }}</span>
          <span v-if="!e.builtin" class="num">{{ words.installs(e.installs) }}</span>
          <span v-if="page.latest?.license">{{ page.latest.license }}</span>
        </div>
        <p class="desc">{{ e.description[lang] }}</p>
      </div>
      <div class="act">
        <a class="btn primary" :href="deepLink"><StoreIcon name="ext" />{{ words.plugin.open }}</a>
        <button v-if="repoUrl" type="button" class="btn" @click="copyRepo">
          <StoreIcon :name="copied ? 'check' : 'copy'" />{{ copied ? words.plugin.copied : words.plugin.copy }}
        </button>
        <a v-else-if="page.builtin" class="btn" :href="page.builtin.docs[lang]"><StoreIcon name="ext" />{{ words.plugin.builtinDocs }}</a>
        <span class="hint">{{ hint }}</span>
      </div>
    </section>

    <nav v-if="!page.builtin" class="tabs">
      <NuxtLink :class="{ on: tab === 'readme' }" :to="tabLink('readme')">{{ words.plugin.tabs.readme }}</NuxtLink>
      <NuxtLink :class="{ on: tab === 'docs' }" :to="tabLink('docs')"
        >{{ words.plugin.tabs.docs }}<span class="n">{{ docs?.pages.length ?? 0 }}</span></NuxtLink
      >
      <NuxtLink :class="{ on: tab === 'versions' }" :to="tabLink('versions')"
        >{{ words.plugin.tabs.versions }}<span class="n">{{ page.versions.length }}</span></NuxtLink
      >
    </nav>

    <div class="plug-body" :class="{ own: page.builtin }">
      <article v-if="page.builtin" class="readme">
        <div class="md">
          <p>{{ words.plugin.builtinNote }}</p>
          <p>
            <a class="arrow" :href="page.builtin.docs[lang]">{{ words.plugin.builtinDocs }}</a>
          </p>
        </div>
      </article>
      <article v-else-if="tab === 'readme'" class="readme">
        <!-- The README as the store rendered it at the check: cut down to a short list of tags, without script, style
             or event (server/utils/readme.ts); the page's policy would refuse them all the same. -->
        <div v-if="readme" class="md" v-html="readme"></div>
        <p v-else class="sub">{{ words.plugin.noReadme }}</p>
        <figure v-if="page.latest?.screenshots.length" class="shots">
          <img v-for="s in page.latest.screenshots" :key="s.src" :src="s.src" :width="s.width" :height="s.height" alt="" loading="lazy" />
          <figcaption>{{ words.plugin.screenshots }}</figcaption>
        </figure>
      </article>
      <article v-else-if="tab === 'docs'" class="readme">
        <nav v-if="(docs?.pages.length ?? 0) > 1" class="doc-pages">
          <NuxtLink
            v-for="p in docs!.pages"
            :key="p.path"
            :class="{ on: p.path === docPage?.path }"
            :to="{ query: { tab: 'docs', page: p.path } }"
            >{{ p.title }}</NuxtLink
          >
        </nav>
        <div v-if="docPage" class="md" v-html="docPage.html"></div>
        <p v-else class="sub">{{ words.plugin.noDocs }}</p>
      </article>
      <div v-else class="versions">
        <table class="t">
          <thead>
            <tr>
              <th>{{ words.plugin.versions.version }}</th>
              <th>{{ words.plugin.versions.state }}</th>
              <th class="hide-sm">{{ words.plugin.versions.commit }}</th>
              <th class="hide-sm">{{ words.plugin.versions.escouade }}</th>
              <th class="r">{{ words.plugin.versions.date }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="v in page.versions" :key="v.version">
              <td class="mono">v{{ v.version }}</td>
              <td>
                <span v-if="v.disabled" class="gone" :title="words.plugin.disabledTitle">{{ words.plugin.disabled }}</span>
                <TrustMark v-else :trust="v.trust" :version="v.version" />
                <small v-if="v.tagMoved" class="moved">{{ words.plugin.moved }}</small>
              </td>
              <td class="mono hide-sm" :title="v.sha256 ? `sha256:${v.sha256}` : undefined">{{ v.tag }} → {{ short(v.commit) }}</td>
              <td class="mono hide-sm">{{ v.escouade }}</td>
              <td class="r">{{ v.publishedAt ? words.date(v.publishedAt) : '' }}</td>
            </tr>
          </tbody>
        </table>
      </div>

      <aside class="aside">
        <div>
          <h3>{{ words.plugin.can }}</h3>
          <div v-for="(p, i) in page.permissions" :key="i" class="perm" :class="p.tone">
            <StoreIcon :name="p.icon" />
            <span
              >{{ p.text[lang]
              }}<template v-if="p.icon === 'net' && p.mono"
                >{{ ' ' }}<span class="host">{{ p.mono }}</span></template
              ><small v-if="p.icon === 'run'"
                ><template v-if="p.mono">{{ p.mono }}, </template>{{ p.detail?.[lang] }}</small
              ><small v-else-if="p.mono && p.icon !== 'net'" class="host">{{ p.mono }}</small></span
            >
          </div>
        </div>
        <div v-if="!page.builtin" class="review-note" :class="{ asked: e.trust === 'review' }">
          <b>{{ e.trust === 'review' ? words.trust.review : words.trust.unverified }}</b>
          <p>{{ e.trust === 'review' ? words.plugin.reviewAsked : words.plugin.reviewNot }}</p>
        </div>
        <div v-if="page.latest">
          <h3>{{ words.plugin.origin }}</h3>
          <ol class="chain">
            <li>
              <span class="dot"><StoreIcon name="git" /></span
              ><span
                ><span class="k">{{ words.plugin.repo }}</span
                ><a class="v" :href="repoUrl!" rel="nofollow noopener noreferrer">github.com/{{ page.latest.repository }}</a></span
              >
            </li>
            <li>
              <span class="dot"><StoreIcon name="tag" /></span
              ><span
                ><span class="k">{{ words.plugin.tag }}</span
                ><a class="v" :href="`${repoUrl}/tree/${page.latest.commit}`" rel="nofollow noopener noreferrer"
                  >{{ page.latest.tag }} → {{ short(page.latest.commit) }}</a
                ></span
              >
            </li>
            <li v-if="page.latest.sha256">
              <span class="dot"><StoreIcon name="box" /></span
              ><span
                ><span class="k">{{ words.plugin.fingerprint }}</span
                ><span class="v" :title="`sha256:${page.latest.sha256}`">{{ fingerprint(page.latest.sha256) }}</span></span
              >
            </li>
            <li>
              <span class="dot"><StoreIcon name="flag" /></span
              ><span
                ><span class="k">{{ words.plugin.signature }}</span
                ><span class="v">{{ words.plugin.unsigned }}</span></span
              >
            </li>
          </ol>
        </div>
        <dl class="facts">
          <template v-if="page.latest?.license"
            ><dt>{{ words.plugin.facts.license }}</dt>
            <dd>{{ page.latest.license }}</dd></template
          >
          <template v-if="e.updatedAt"
            ><dt>{{ words.plugin.facts.updated }}</dt>
            <dd>{{ words.date(e.updatedAt) }}</dd></template
          >
          <template v-if="page.latest?.escouade"
            ><dt>{{ words.plugin.facts.escouade }}</dt>
            <dd class="mono">{{ page.latest.escouade }}</dd></template
          >
          <dt>{{ words.plugin.facts.id }}</dt>
          <dd class="mono">{{ e.id }}</dd>
        </dl>
        <div class="report-line">
          <NuxtLink class="report" :to="pathIn(lang, `/plugins/${encodeURIComponent(e.id)}/report`)"
            ><StoreIcon name="flag" />{{ words.plugin.report }}</NuxtLink
          >
          <span v-if="page.latest?.reports" class="open">{{ words.plugin.reportsOpen(page.latest.reports) }}</span>
        </div>
      </aside>
    </div>
  </div>
</template>

<style scoped>
.plug-head {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  gap: 22px;
  align-items: start;
  padding: 44px 0 28px;
}
.plug-head h1 {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 14px;
  font-size: clamp(28px, 2.8vw, 36px);
  font-weight: 640;
  line-height: 1.1;
  letter-spacing: -0.035em;
}
.line {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px 16px;
  margin-top: 8px;
  color: var(--dim);
  font-size: 13.5px;
}
.line a {
  color: var(--dim);
}
.line a:hover {
  color: var(--text);
}
.line a.me {
  color: var(--soft);
}
.me {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-weight: 550;
}
.desc {
  max-width: 40em;
  margin-top: 12px;
  color: var(--muted);
  font-size: 16px;
}
.act {
  display: grid;
  justify-items: stretch;
  gap: 8px;
  min-width: 230px;
}
.act .hint {
  color: var(--dim);
  font-size: 12px;
  text-align: center;
}
.tabs {
  display: flex;
  gap: 24px;
  border-bottom: 1px solid var(--line);
}
.tabs a {
  margin-bottom: -1px;
  padding: 10px 0;
  border-bottom: 1.5px solid transparent;
  color: var(--muted);
  font-size: 14px;
  font-weight: 550;
}
.tabs a.on {
  border-color: var(--text);
  color: var(--text);
}
.tabs .n {
  margin-left: 4px;
  color: var(--dim);
  font: 11.5px var(--mono);
}
.plug-body {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 340px;
  gap: 56px;
  padding-top: 28px;
}
.plug-body.own {
  border-top: 1px solid var(--line);
}
.readme {
  min-width: 0;
  max-width: 68ch;
  color: var(--soft);
  font-size: 15.5px;
}
.sub {
  color: var(--muted);
}
/* The README, as the store rendered it: its tags drawn as the preview draws a plugin's presentation. */
.md :deep(h1),
.md :deep(h2) {
  margin: 30px 0 10px;
  color: var(--text);
  font-size: 19px;
  font-weight: 620;
  letter-spacing: -0.02em;
}
.md :deep(h1) {
  font-size: 22px;
}
.md :deep(h3),
.md :deep(h4),
.md :deep(h5),
.md :deep(h6) {
  margin: 22px 0 8px;
  color: var(--text);
  font-size: 16px;
  font-weight: 620;
}
.md :deep(> :first-child) {
  margin-top: 0;
}
.md :deep(p) {
  margin: 0 0 12px;
}
.md :deep(ul),
.md :deep(ol) {
  margin: 0 0 12px;
  padding-left: 18px;
}
.md :deep(li) {
  margin: 4px 0;
}
.md :deep(a[href]) {
  color: var(--brand-ink);
  text-decoration: underline;
  text-decoration-color: color-mix(in srgb, var(--brand-ink) 40%, transparent);
  text-underline-offset: 2px;
}
.md :deep(pre) {
  margin: 0 0 14px;
  padding: 14px 16px;
  overflow: auto;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--surface);
  color: var(--soft);
  font: 13px/1.6 var(--mono);
}
.md :deep(pre code) {
  padding: 0;
  border: 0;
  background: none;
  font-size: inherit;
}
.md :deep(blockquote) {
  margin: 0 0 12px;
  padding: 2px 14px;
  border-left: 3px solid var(--line-strong);
  color: var(--muted);
}
.md :deep(img) {
  max-width: 100%;
  height: auto;
  border-radius: 8px;
}
.md :deep(table) {
  width: 100%;
  margin: 0 0 14px;
  border-collapse: collapse;
  font-size: 14px;
}
.md :deep(th),
.md :deep(td) {
  padding: 8px 10px;
  border-bottom: 1px solid var(--line);
  text-align: left;
}
.md :deep(hr) {
  margin: 24px 0;
  border: 0;
  border-top: 1px solid var(--line);
}
.md :deep(details) {
  margin: 0 0 12px;
}
.shots {
  display: grid;
  gap: 14px;
  margin: 22px 0 0;
}
.shots img {
  max-width: 100%;
  height: auto;
  border: 1px solid var(--line-strong);
  border-radius: 12px;
  box-shadow: var(--shot);
}
.shots figcaption {
  color: var(--dim);
  font-size: 12px;
}
.doc-pages {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 20px;
}
.doc-pages a {
  padding: 4px 10px;
  border: 1px solid var(--line);
  border-radius: 7px;
  color: var(--muted);
  font-size: 13.5px;
}
.doc-pages a.on {
  border-color: var(--line-strong);
  background: var(--surface-2);
  color: var(--text);
}
.versions {
  min-width: 0;
}
table.t {
  width: 100%;
  border-collapse: collapse;
  font-size: 14px;
}
table.t th {
  padding: 0 12px 10px;
  border-bottom: 1px solid var(--line);
  color: var(--dim);
  font-size: 12.5px;
  font-weight: 600;
  text-align: left;
  white-space: nowrap;
}
table.t td {
  padding: 14px 12px;
  border-bottom: 1px solid var(--line);
  vertical-align: top;
}
table.t th:first-child,
table.t td:first-child {
  padding-left: 4px;
}
table.t .r {
  text-align: right;
}
.gone {
  display: inline-flex;
  align-items: center;
  height: 22px;
  padding: 0 8px;
  border: 1px solid color-mix(in oklch, var(--danger) 45%, transparent);
  border-radius: 6px;
  background: var(--danger-bg);
  color: var(--danger);
  font-size: 12px;
  font-weight: 550;
}
.moved {
  display: block;
  max-width: 28em;
  margin-top: 6px;
  color: var(--warn);
  font-size: 12.5px;
}
.aside {
  display: grid;
  align-content: start;
  gap: 28px;
}
.aside h3 {
  margin-bottom: 10px;
  color: var(--dim);
  font-size: 12.5px;
  font-weight: 600;
}
.perm {
  display: grid;
  grid-template-columns: 18px 1fr;
  gap: 10px;
  padding: 9px 0;
  border-top: 1px solid var(--line);
  color: var(--soft);
  font-size: 14px;
}
.perm:first-of-type {
  border-top: 0;
}
.perm svg.i {
  margin-top: 2px;
  color: var(--dim);
}
.perm.strong {
  color: var(--text);
  font-weight: 560;
}
.perm.strong svg.i {
  color: var(--text);
}
.perm.red,
.perm.red svg.i {
  color: var(--danger);
}
.perm small {
  display: block;
  color: var(--dim);
  font-size: 12.5px;
  font-weight: 400;
}
.perm .host {
  font-family: var(--mono);
  font-size: 12.5px;
}
.review-note {
  padding: 14px 16px;
  border: 1px solid var(--line-strong);
  border-radius: 10px;
  color: var(--soft);
  font-size: 13.5px;
}
.review-note.asked {
  border-style: dashed;
}
.review-note b {
  color: var(--text);
  font-weight: 600;
}
.review-note p {
  margin: 6px 0 0;
}
.chain {
  position: relative;
  margin: 0;
  padding: 0;
  list-style: none;
}
.chain::before {
  content: '';
  position: absolute;
  top: 12px;
  bottom: 12px;
  left: 8px;
  width: 1px;
  background: var(--line-strong);
}
.chain li {
  position: relative;
  display: grid;
  grid-template-columns: 18px 1fr;
  gap: 12px;
  padding: 6px 0;
  font-size: 13px;
}
.chain .dot {
  display: grid;
  place-items: center;
  width: 17px;
  height: 17px;
  margin-top: 1px;
  border: 1px solid var(--line-strong);
  border-radius: 50%;
  background: var(--bg);
  color: var(--muted);
}
.chain .dot svg.i {
  width: 10px;
  height: 10px;
  stroke-width: 2;
}
.chain .k {
  color: var(--dim);
}
.chain .v {
  display: block;
  color: var(--soft);
  font-family: var(--mono);
  font-size: 12px;
  word-break: break-all;
}
.facts {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 8px 16px;
  margin: 0;
  font-size: 13.5px;
}
.facts dt {
  color: var(--dim);
}
.facts dd {
  margin: 0;
  color: var(--soft);
  text-align: right;
  overflow-wrap: anywhere;
}
.report-line {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px 14px;
}
.report {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--dim);
  font-size: 13px;
}
.report:hover {
  color: var(--text);
}
.open {
  color: var(--warn);
  font-size: 12.5px;
}
.mono {
  font-family: var(--mono);
  font-size: 0.86em;
}
.num {
  font-variant-numeric: tabular-nums;
}
@media (max-width: 1180px) {
  .plug-body {
    grid-template-columns: minmax(0, 1fr) 300px;
    gap: 40px;
  }
}
@media (max-width: 720px) {
  .plug-head {
    grid-template-columns: auto minmax(0, 1fr);
    gap: 16px;
    padding-top: 28px;
  }
  .plug-head .act {
    grid-column: 1 / -1;
    min-width: 0;
  }
  .plug-body {
    grid-template-columns: 1fr;
  }
  .readme,
  .versions {
    order: 2;
  }
  .aside {
    order: 1;
  }
  .hide-sm {
    display: none;
  }
}
</style>
