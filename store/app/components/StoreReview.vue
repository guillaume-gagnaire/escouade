<script setup lang="ts">
import { REVIEW_TEXTS } from '~/data/review';
import { pathIn } from '~/data/language';
import type { Icon } from '~~/shared/tile';

// The review queue, the administrator's alone (docs/STORE.md §4.1; the preview's #review, choice 171): « À relire »,
// « Signalements », « Noms proches », « Faites »; the versions with why (first version, update and its diff), their
// installs and their wait; on the right the version chosen: the command to copy, what changed (the new permissions in
// amber), the report once the script sent it, « Demander des changements » and « Refuser ». No « Vérifier » button:
// the signature is made on the user's machine, the store only takes it in. Every row is a link and every action a
// plain form: the page works without script under the strict Content-Security-Policy.
type Words = { fr: string; en: string };
interface Perm {
  icon: 'run' | 'gavel' | 'tool' | 'bolt' | 'net' | 'key' | 'leaf';
  tone: 'strong' | 'red' | '';
  text: Words;
  mono?: string;
  isNew: boolean;
}
interface QVersion {
  versionId: number;
  id: string;
  version: string;
  name: Words;
  mark: string;
  hue: number;
  icon: Icon | null;
  author: string | null;
  previous: string | null;
  base: { version: string; signedAt: string } | null;
  installs: number;
  publishedAt: string;
  permissions: Perm[];
  diff: { added: number; removed: number; files: number; paths: string[] } | null;
  report: { verdict: string | null; at: string; text: string } | null;
  reports: number;
  state: 'verified' | 'changes' | 'refused' | null;
  message: string | null;
  reviewedAt: string | null;
  command: string;
}
interface QReport {
  reportId: number;
  id: string;
  name: Words;
  mark: string;
  hue: number;
  version: string | null;
  reason: string;
  details: string;
  createdAt: string;
  open: number;
  malicious: number;
}
interface QHeld {
  versionId: number;
  id: string;
  version: string | null;
  name: Words;
  mark: string;
  hue: number;
  repository: string;
  author: string | null;
  why: Words;
  createdAt: string;
}
interface Queue {
  toReview: QVersion[];
  reports: QReport[];
  held: QHeld[];
  done: QVersion[];
  counts: { toReview: number; reports: number; held: number; total: number };
  keys: number;
  revocations: { serial: number } | null;
}

const { lang } = useLang();
const route = useRoute();
const t = computed(() => REVIEW_TEXTS[lang.value]);
const here = computed(() => pathIn(lang.value, '/review'));
const { data, error } = await useFetch<Queue>('/api/review/queue', { key: 'review-queue' });
const TABS = ['queue', 'reports', 'names', 'done'] as const;
type Tab = (typeof TABS)[number];
const tab = computed<Tab>(() => ((TABS as readonly string[]).includes(String(route.query.tab)) ? (route.query.tab as Tab) : 'queue'));
const picked = computed(() => Number(route.query.v));
const versions = computed(() => (tab.value === 'done' ? data.value?.done : data.value?.toReview) ?? []);
const chosen = computed(() => versions.value.find((v) => v.versionId === picked.value) ?? versions.value[0] ?? null);
const report = computed(() => data.value?.reports.find((r) => r.reportId === picked.value) ?? data.value?.reports[0] ?? null);
const held = computed(() => data.value?.held.find((h) => h.versionId === picked.value) ?? data.value?.held[0] ?? null);
const link = (tb: Tab, v?: number) => ({ path: here.value, query: { ...(tb === 'queue' ? {} : { tab: tb }), ...(v ? { v } : {}) } });
const name = (w: Words) => w[lang.value] || w.en || w.fr;
const signIn = computed(() => `/api/auth/github?to=${encodeURIComponent(here.value)}`);
const account = computed(() => pathIn(lang.value, '/account'));
const notice = computed(() => t.value.notices[typeof route.query.notice === 'string' ? route.query.notice : ''] ?? null);
const num = (n: number) => n.toLocaleString(lang.value === 'fr' ? 'fr-FR' : 'en-GB');
const date = (iso: string) =>
  new Date(iso).toLocaleDateString(lang.value === 'fr' ? 'fr-FR' : 'en-GB', { day: 'numeric', month: 'short', timeZone: 'UTC' });
const now = Date.now();
function waited(iso: string): string {
  const min = Math.max(0, Math.round((now - Date.parse(iso)) / 60_000));
  if (min < 60) return t.value.ago.m(min);
  if (min < 48 * 60) return t.value.ago.h(Math.round(min / 60));
  return t.value.ago.d(Math.round(min / 1440));
}
const target = (v: { id: string; version: string | null }) => `${v.id}${v.version ? `@${v.version}` : ''}`;
const firstNew = (v: QVersion) => v.permissions.find((p) => p.isNew) ?? null;
const permText = (p: Perm) => `${p.text[lang.value]}${p.mono ? ` ${p.mono}` : ''}`;
const versionAction = (id: number, what: string) => `/api/review/versions/${id}/${what}`;
const reportAction = (id: number, what: string) => `/api/review/reports/${id}/${what}`;
const copied = ref('');
async function copy(text: string) {
  await navigator.clipboard?.writeText(text).catch(() => {});
  copied.value = text;
  setTimeout(() => (copied.value = ''), 1600);
}
useHead(() => ({ title: t.value.title, meta: [{ name: 'robots', content: 'noindex' }] }));
</script>

<template>
  <div v-if="data" class="wrap review-layout">
    <section>
      <h1 class="page-title">
        {{ t.title }} <span class="more">· {{ t.toRead(data.counts.total) }}</span>
      </h1>
      <p v-if="notice" class="notice" :class="notice[0]" role="status">{{ notice[1] }}</p>
      <p v-if="!data.keys" class="notice warn">{{ t.noKeys }}</p>
      <nav class="qtabs" :aria-label="t.title">
        <NuxtLink
          v-for="tb in TABS"
          :key="tb"
          class="chip"
          :class="{ on: tab === tb }"
          :to="link(tb)"
          :aria-current="tab === tb ? 'page' : undefined"
          >{{ t.tabs[tb]
          }}<span v-if="tb !== 'done'" class="n">{{
            tb === 'queue' ? data.counts.toReview : tb === 'reports' ? data.counts.reports : data.counts.held
          }}</span></NuxtLink
        >
      </nav>

      <table v-if="tab === 'queue' || tab === 'done'" class="t">
        <thead>
          <tr>
            <th>{{ t.head.version }}</th>
            <th class="hide-sm">{{ tab === 'done' ? t.head.state : t.head.why }}</th>
            <th class="r hide-sm">{{ t.head.installs }}</th>
            <th class="r">{{ tab === 'done' ? t.head.when : t.head.waits }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="v in versions" :key="v.versionId" :class="{ sel: chosen?.versionId === v.versionId }">
            <td>
              <NuxtLink class="pl" :to="link(tab, v.versionId)"
                ><PluginTile :mark="v.mark" :hue="v.hue" :icon="v.icon" size="sm" /><span
                  ><b>{{ name(v.name) }}</b> <span class="mono dim">v{{ v.version }}</span
                  ><small
                    >{{ v.author ?? v.id
                    }}<template v-if="tab === 'queue' && firstNew(v)">
                      · <span class="grown">{{ t.grown(permText(firstNew(v)!)) }}</span></template
                    ></small
                  ></span
                ></NuxtLink
              >
            </td>
            <td class="hide-sm">
              <template v-if="tab === 'done'"
                ><span class="tag" :class="{ ok: v.state === 'verified', warn: v.state === 'changes', bad: v.state === 'refused' }">{{
                  t.states[v.state ?? ''] ?? ''
                }}</span></template
              >
              <template v-else>
                {{ v.previous ? t.update(v.previous) : t.first }}
                <div v-if="v.diff" class="diffstat">
                  <span class="a">+{{ num(v.diff.added) }}</span
                  ><span class="d">−{{ num(v.diff.removed) }}</span
                  ><span class="dim">{{ t.files(v.diff.files) }}</span>
                </div>
              </template>
            </td>
            <td class="r num hide-sm">{{ num(v.installs) }}</td>
            <td class="r dim">{{ tab === 'done' && v.reviewedAt ? date(v.reviewedAt) : waited(v.publishedAt) }}</td>
          </tr>
          <tr v-if="!versions.length" class="empty">
            <td colspan="4">{{ tab === 'done' ? t.empty.done : t.empty.queue }}</td>
          </tr>
        </tbody>
      </table>

      <table v-else-if="tab === 'reports'" class="t">
        <thead>
          <tr>
            <th>{{ t.head.version }}</th>
            <th class="hide-sm">{{ t.head.why }}</th>
            <th class="r">{{ t.head.waits }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="r in data.reports" :key="r.reportId" :class="{ sel: report?.reportId === r.reportId }">
            <td>
              <NuxtLink class="pl" :to="link('reports', r.reportId)"
                ><PluginTile :mark="r.mark" :hue="r.hue" size="sm" /><span
                  ><b>{{ name(r.name) }}</b> <span v-if="r.version" class="mono dim">v{{ r.version }}</span
                  ><small>{{ r.id }}</small></span
                ></NuxtLink
              >
            </td>
            <td class="hide-sm">
              <span class="tag" :class="{ bad: r.reason === 'malicious', warn: r.reason !== 'malicious' }">{{ t.reason(r.reason) }}</span>
            </td>
            <td class="r dim">{{ waited(r.createdAt) }}</td>
          </tr>
          <tr v-if="!data.reports.length" class="empty">
            <td colspan="3">{{ t.empty.reports }}</td>
          </tr>
        </tbody>
      </table>

      <table v-else class="t">
        <thead>
          <tr>
            <th>{{ t.head.version }}</th>
            <th class="hide-sm">{{ t.head.why }}</th>
            <th class="r">{{ t.head.waits }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="h in data.held" :key="h.versionId" :class="{ sel: held?.versionId === h.versionId }">
            <td>
              <NuxtLink class="pl" :to="link('names', h.versionId)"
                ><PluginTile :mark="h.mark" :hue="h.hue" size="sm" /><span
                  ><b>{{ name(h.name) || h.id }}</b> <span v-if="h.version" class="mono dim">v{{ h.version }}</span
                  ><small>{{ h.author ?? h.repository }}</small></span
                ></NuxtLink
              >
            </td>
            <td class="hide-sm">
              <span class="tag warn">{{ t.held }}</span>
            </td>
            <td class="r dim">{{ waited(h.createdAt) }}</td>
          </tr>
          <tr v-if="!data.held.length" class="empty">
            <td colspan="3">{{ t.empty.names }}</td>
          </tr>
        </tbody>
      </table>

      <p class="foot-note dim">
        {{ data.revocations ? t.serial(data.revocations.serial) : t.noSerial }} · {{ t.token }}
        <NuxtLink :to="{ path: account, hash: '#tokens' }">{{ t.tokenLink }}</NuxtLink>
      </p>
    </section>

    <aside v-if="(tab === 'queue' || tab === 'done') && chosen" class="detail">
      <div class="pl">
        <PluginTile :mark="chosen.mark" :hue="chosen.hue" :icon="chosen.icon" /><span
          ><b>{{ name(chosen.name) }}</b> <span class="mono dim">v{{ chosen.version }}</span
          ><small
            >{{ chosen.author ?? chosen.id
            }}{{
              chosen.base ? ` · ${t.verifiedOn(chosen.base.version, date(chosen.base.signedAt))}` : chosen.previous ? '' : ` · ${t.first}`
            }}</small
          ></span
        >
      </div>
      <div>
        <h3 class="h3">{{ t.readOnMachine }}</h3>
        <div class="cmd">
          <span><span class="p">$ </span>{{ chosen.command }}</span
          ><button type="button" class="copy" :aria-label="t.copy" @click="copy(chosen.command)">
            <StoreIcon :name="copied === chosen.command ? 'check' : 'copy'" />
          </button>
        </div>
        <p class="sub">
          {{ t.orClaude(`/review-plugin ${target(chosen)}`) }}<template v-if="chosen.base"> {{ t.diffFrom(chosen.base.version) }}</template>
        </p>
      </div>
      <div>
        <h3 class="h3">{{ chosen.previous ? t.changed : t.asks }}</h3>
        <div v-for="(p, i) in chosen.permissions" :key="i" class="perm" :class="p.tone">
          <StoreIcon :name="p.icon" /><span
            >{{ p.text[lang]
            }}<template v-if="p.icon === 'net' && p.mono"
              >{{ ' ' }}<span class="host">{{ p.mono }}</span></template
            ><small v-if="p.mono && p.icon !== 'net'" class="host">{{ p.mono }}</small
            ><small v-if="chosen.previous" :class="{ grown: p.isNew }">{{
              p.isNew ? t.newIn(chosen.version) : t.already(chosen.base?.version ?? chosen.previous)
            }}</small></span
          >
        </div>
        <div v-if="chosen.diff" class="diffstat">
          <span class="a">+{{ num(chosen.diff.added) }}</span
          ><span class="d">−{{ num(chosen.diff.removed) }}</span
          ><span class="dim"
            >{{ t.files(chosen.diff.files)
            }}<template v-if="chosen.diff.paths.length">
              · {{ chosen.diff.paths.slice(0, 2).join(', ') }}{{ chosen.diff.paths.length > 2 ? '…' : '' }}</template
            ></span
          >
        </div>
      </div>
      <div>
        <h3 class="h3">{{ t.report }}</h3>
        <details v-if="chosen.report" class="flag">
          <summary>
            {{ t.reportAt(date(chosen.report.at), chosen.report.verdict) }}<small>{{ t.readReport }}</small>
          </summary>
          <pre class="report">{{ chosen.report.text }}</pre>
        </details>
        <div v-else class="flag">
          {{ t.noReport }}<small>{{ t.noReportSub }}</small>
        </div>
      </div>
      <p v-if="chosen.message" class="said">{{ chosen.message }}</p>
      <template v-if="tab === 'queue'">
        <div class="cmd quiet">
          <span>{{ t.signCmd(target(chosen)) }}</span
          ><button type="button" class="copy" :aria-label="t.copy" @click="copy(t.signCmd(target(chosen)))">
            <StoreIcon :name="copied === t.signCmd(target(chosen)) ? 'check' : 'copy'" />
          </button>
        </div>
        <div class="acts">
          <details class="decide">
            <summary class="btn sm">{{ t.changes }}</summary>
            <form method="post" :action="versionAction(chosen.versionId, 'changes')">
              <label :for="`m-c-${chosen.versionId}`">{{ t.message }}</label>
              <textarea :id="`m-c-${chosen.versionId}`" name="message" rows="4" minlength="10" maxlength="4000" required></textarea>
              <input type="hidden" name="lang" :value="lang" />
              <button class="btn sm primary" type="submit">{{ t.send }}</button>
            </form>
          </details>
          <details class="decide">
            <summary class="btn sm danger">{{ t.refuse }}</summary>
            <form method="post" :action="versionAction(chosen.versionId, 'refuse')">
              <label :for="`m-r-${chosen.versionId}`">{{ t.message }}</label>
              <textarea :id="`m-r-${chosen.versionId}`" name="message" rows="4" minlength="10" maxlength="4000" required></textarea>
              <input type="hidden" name="lang" :value="lang" />
              <button class="btn sm danger" type="submit">{{ t.refuse }}</button>
            </form>
          </details>
        </div>
      </template>
    </aside>

    <aside v-else-if="tab === 'reports' && report" class="detail">
      <div class="pl">
        <PluginTile :mark="report.mark" :hue="report.hue" /><span
          ><b>{{ t.reportOf(name(report.name), report.version) }}</b
          ><small>{{ t.openOn(report.open, report.malicious) }}</small></span
        >
      </div>
      <div>
        <span class="tag" :class="{ bad: report.reason === 'malicious', warn: report.reason !== 'malicious' }">{{
          t.reason(report.reason)
        }}</span>
        <p class="said">{{ report.details }}</p>
        <NuxtLink class="arrow" :to="pathIn(lang, `/plugins/${encodeURIComponent(report.id)}`)">{{ t.pluginPage }}</NuxtLink>
      </div>
      <div>
        <h3 class="h3">{{ t.withdraw }}</h3>
        <div class="cmd">
          <span><span class="p">$ </span>{{ t.withdrawCmd(target(report)) }}</span
          ><button type="button" class="copy" :aria-label="t.copy" @click="copy(t.withdrawCmd(target(report)))">
            <StoreIcon :name="copied === t.withdrawCmd(target(report)) ? 'check' : 'copy'" />
          </button>
        </div>
      </div>
      <form
        v-for="act in ['upheld', 'dismissed'] as const"
        :key="act"
        method="post"
        :action="reportAction(report.reportId, act)"
        class="close"
      >
        <label :for="`a-${act}`">{{ t.answer }}</label>
        <textarea :id="`a-${act}`" name="answer" rows="3" maxlength="4000"></textarea>
        <input type="hidden" name="lang" :value="lang" />
        <button class="btn sm" :class="{ danger: act === 'upheld' }" type="submit">{{ t[act] }}</button>
      </form>
    </aside>

    <aside v-else-if="tab === 'names' && held" class="detail">
      <div class="pl">
        <PluginTile :mark="held.mark" :hue="held.hue" /><span
          ><b>{{ name(held.name) || held.id }}</b> <span v-if="held.version" class="mono dim">v{{ held.version }}</span
          ><small>{{ held.id }}</small></span
        >
      </div>
      <div class="flag">
        {{ held.why[lang] }}<small>{{ t.repo }} : github.com/{{ held.repository }}</small>
      </div>
      <form method="post" :action="versionAction(held.versionId, 'release')">
        <input type="hidden" name="lang" :value="lang" />
        <button class="btn sm primary" type="submit">{{ t.release }}</button>
        <p class="sub">{{ t.releaseSub }}</p>
      </form>
      <details class="decide">
        <summary class="btn sm danger">{{ t.refuseHeld }}</summary>
        <form method="post" :action="versionAction(held.versionId, 'refuse-held')">
          <label :for="`m-h-${held.versionId}`">{{ t.message }}</label>
          <textarea :id="`m-h-${held.versionId}`" name="message" rows="4" minlength="10" maxlength="4000" required></textarea>
          <input type="hidden" name="lang" :value="lang" />
          <button class="btn sm danger" type="submit">{{ t.refuseHeld }}</button>
        </form>
      </details>
    </aside>
  </div>

  <div v-else class="wrap guest">
    <h1 class="page-title">{{ t.title }}</h1>
    <template v-if="error?.statusCode === 401">
      <p>{{ t.guest }}</p>
      <a class="btn primary" :href="signIn">{{ t.signIn }}</a>
    </template>
    <p v-else-if="error?.statusCode === 503">{{ t.off }}</p>
    <p v-else>{{ t.notAdmin }}</p>
  </div>
</template>

<style scoped>
.review-layout {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 400px;
  gap: 40px;
  padding-top: 32px;
  padding-bottom: 64px;
}
.review-layout > *,
.detail > * {
  min-width: 0;
}
.page-title {
  margin: 0;
  font-size: clamp(26px, 2.6vw, 34px);
  font-weight: 620;
  letter-spacing: -0.03em;
  line-height: 1.15;
  text-wrap: balance;
}
.page-title .more {
  color: var(--muted);
}
.notice {
  margin: 16px 0 0;
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
.qtabs {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin: 18px 0 14px;
}
.chip {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  height: 32px;
  padding: 0 12px;
  border: 1px solid var(--line);
  border-radius: 8px;
  color: var(--soft);
  font-size: 13.5px;
  font-weight: 520;
  white-space: nowrap;
  text-decoration: none;
}
.chip .n {
  color: var(--dim);
  font: 11.5px var(--mono);
}
.chip.on {
  border-color: var(--line-strong);
  background: var(--surface-2);
  color: var(--text);
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
table.t td.r,
table.t th.r {
  text-align: right;
}
table.t tr.sel td {
  background: var(--surface);
}
table.t tr.empty td {
  color: var(--dim);
}
.num {
  font-variant-numeric: tabular-nums;
}
.pl {
  display: flex;
  gap: 12px;
  align-items: center;
  color: inherit;
  text-decoration: none;
}
.pl b {
  font-weight: 600;
}
.pl small {
  display: block;
  color: var(--dim);
  font-size: 12.5px;
}
.mono {
  font-family: var(--mono);
  font-size: 0.86em;
}
.dim {
  color: var(--dim);
}
.grown {
  color: var(--warn);
  font-size: 12.5px;
}
.tag {
  display: inline-flex;
  align-items: center;
  height: 22px;
  padding: 0 8px;
  border: 1px solid var(--line-strong);
  border-radius: 6px;
  color: var(--muted);
  font-size: 12px;
  font-weight: 550;
  white-space: nowrap;
}
.tag.warn {
  border-color: color-mix(in oklch, var(--warn) 45%, transparent);
  background: var(--warn-bg);
  color: var(--warn);
}
.tag.bad {
  border-color: color-mix(in oklch, var(--danger) 45%, transparent);
  background: var(--danger-bg);
  color: var(--danger);
}
.tag.ok {
  border-color: color-mix(in oklch, var(--ok) 40%, transparent);
  color: var(--ok);
}
.diffstat {
  display: flex;
  gap: 10px;
  align-items: center;
  margin-top: 4px;
  font: 12.5px var(--mono);
}
.diffstat .a {
  color: var(--ok);
}
.diffstat .d {
  color: var(--danger);
}
.foot-note {
  margin: 18px 0 0;
  font-size: 13px;
}
.foot-note a {
  color: var(--brand-ink);
}
.detail {
  position: sticky;
  top: 84px;
  display: grid;
  gap: 18px;
  align-self: start;
  padding: 22px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
  font-size: 14px;
}
.h3 {
  margin: 0 0 8px;
  font-size: 14px;
  font-weight: 620;
  letter-spacing: -0.015em;
}
.cmd {
  display: flex;
  gap: 10px;
  align-items: center;
  padding: 10px 12px;
  border: 1px solid var(--line-strong);
  border-radius: 9px;
  background: var(--bg);
  color: var(--soft);
  font: 12.5px var(--mono);
}
.cmd.quiet {
  border-color: var(--line);
  color: var(--dim);
}
.cmd > span {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.cmd .p {
  color: var(--dim);
}
.copy {
  display: grid;
  place-items: center;
  padding: 2px;
  border: 0;
  background: none;
  color: var(--muted);
  cursor: pointer;
}
.sub {
  margin: 6px 0 0;
  color: var(--dim);
  font-size: 12.5px;
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
.perm :deep(svg.i) {
  margin-top: 2px;
  color: var(--dim);
}
.perm.strong {
  color: var(--text);
  font-weight: 560;
}
.perm.red,
.perm.red :deep(svg.i) {
  color: var(--danger);
}
.perm small {
  display: block;
  color: var(--dim);
  font-size: 12.5px;
  font-weight: 400;
}
.perm small.grown {
  color: var(--warn);
}
.perm .host {
  font-family: var(--mono);
  font-size: 12.5px;
}
.flag {
  padding: 10px 12px;
  border: 1px solid var(--line);
  border-radius: 9px;
  color: var(--soft);
  font-size: 13px;
}
.flag small {
  display: block;
  color: var(--dim);
}
details.flag summary {
  cursor: pointer;
}
.report {
  max-height: 420px;
  margin: 10px 0 0;
  overflow: auto;
  color: var(--soft);
  font: 12px/1.5 var(--mono);
  white-space: pre-wrap;
  word-break: break-word;
}
.said {
  margin: 8px 0;
  color: var(--soft);
  font-size: 13.5px;
  white-space: pre-line;
}
.arrow {
  color: var(--brand-ink);
  font-size: 13.5px;
}
.acts {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
details.decide {
  flex: 1 1 100%;
}
.acts details.decide {
  flex: 0 1 auto;
}
details.decide[open] {
  flex: 1 1 100%;
}
details.decide summary {
  list-style: none;
}
details.decide summary::-webkit-details-marker {
  display: none;
}
form {
  display: grid;
  gap: 8px;
  margin: 0;
}
details.decide form {
  margin-top: 10px;
}
label {
  color: var(--dim);
  font-size: 12.5px;
}
textarea {
  width: 100%;
  padding: 8px 10px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: var(--bg);
  color: var(--text);
  font: 13.5px var(--font, inherit);
  resize: vertical;
}
form .btn {
  justify-self: start;
}
.btn.sm {
  height: 30px;
  padding: 0 11px;
  border-radius: 7px;
  font-size: 13px;
}
.btn.danger {
  color: var(--danger);
}
.guest {
  display: grid;
  gap: 16px;
  justify-items: start;
  padding-top: 48px;
  padding-bottom: 96px;
}
@media (max-width: 1100px) {
  .review-layout {
    grid-template-columns: minmax(0, 1fr) 340px;
    gap: 28px;
  }
}
@media (max-width: 860px) {
  .review-layout {
    grid-template-columns: 1fr;
  }
  .detail {
    position: static;
  }
  .hide-sm {
    display: none;
  }
}
</style>
