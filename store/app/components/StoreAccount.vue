<script setup lang="ts">
import { ACCOUNT_TEXTS } from '~/data/account';
import { pathIn } from '~/data/language';
import { hueOf, markOf, type Icon } from '~~/shared/tile';

// The author's account, as the preview draws it (design/store/, #account): « Mes plugins · login », the version
// found on a new tag that waits for a click (the banner), their plugins with their state, latest version and installs
// (a refused first version says why below its row), then the chosen plugin's management — its versions, taking one
// back, new tags, the list, the transfer, the deletion (docs/STORE.md, §2.2) — and the App's installations; on the
// right the chosen plugin's installs by week, the e-mail notices and « Mes données » (export, sign-out, deletion).
// Every action is a plain form: it works without script, under the strict Content-Security-Policy, and the server
// answers with the page and a notice.
interface Installation {
  id: number;
  owner: string;
  ownerType: string;
  all: boolean;
  suspended: boolean;
  repos: string[];
}
type Words = { fr: string; en: string };
interface Rejection {
  check: string;
  name: Words;
  detail: Words;
  path: string | null;
  why: Words | null;
  fix: Words | null;
  docs: Words;
}
interface PluginRow {
  id: string;
  name: string;
  icon: Icon | null;
  state: string;
  version: string | null;
  installs: number;
  recent: number;
  rejection: Rejection | null;
  submission: number | null;
  repository: string;
  unlistedAt: string | null;
  deletedAt: string | null;
  purgeOn: string | null;
  frozenUntil: string | null;
  transferredAt: string | null;
  transferredFrom: string | null;
  transferTo: string | null;
  autoPublish: boolean;
}
interface Pending {
  id: number;
  plugin: string;
  name: string;
  icon: Icon | null;
  version: string;
  tag: string;
  commit: string;
  createdAt: string;
  previousCommit: string | null;
  added: string[];
  frozenUntil: string | null;
}
interface VersionRow {
  id: number;
  version: string | null;
  tag: string;
  state: string;
  review: boolean;
  reviewState: string | null;
  reviewMessage: string | null;
  publishedAt: string | null;
  yankedAt: string | null;
  tagMovedAt: string | null;
  installs: number;
  updates: number;
}
interface Selected {
  id: string;
  repository: string;
  autoPublish: boolean;
  installs: number;
  updates: number;
  recent: number;
  weeks: { installs: number; updates: number }[];
  versions: VersionRow[];
}
interface AccountData {
  login: string;
  name: string | null;
  githubId: number;
  email: string | null;
  pendingEmail: string | null;
  installations: Installation[];
  mail: boolean;
  plugins: PluginRow[];
  pending: Pending[];
  offers: { id: string; from: string | null; at: string | null; icon: Icon | null }[];
  claims: { id: string; repository: string; at: string | null; icon: Icon | null }[];
  selected: Selected | null;
  now: string;
}

const { lang, text: base, links } = useLang();
// An administrator also holds the review's tokens here (docs/STORE.md §4.1).
const { data: me } = await useMe();
const route = useRoute();
const text = computed(() => ACCOUNT_TEXTS[lang.value]);
const a = computed(() => text.value.author);
const here = computed(() => pathIn(lang.value, '/account'));
const terms = computed(() => pathIn(lang.value, '/terms'));
const submit = computed(() => pathIn(lang.value, '/submit'));
const query = computed(() => (typeof route.query.plugin === 'string' ? { plugin: route.query.plugin } : {}));
const { data, error } = await useFetch<AccountData>('/api/account', { key: 'account', query });
const rows = computed(() => data.value?.plugins ?? []);
const chosen = computed(() => rows.value.find((p) => p.id === data.value?.selected?.id) ?? null);
const inTheWay = computed(() => rows.value.filter((p) => p.submission === null && !p.deletedAt).map((p) => p.id));
const num = (n: number) => n.toLocaleString(lang.value === 'fr' ? 'fr-FR' : 'en-GB');
const date = (iso: string | null) =>
  iso
    ? new Date(iso).toLocaleDateString(lang.value === 'fr' ? 'fr-FR' : 'en-GB', {
        day: 'numeric',
        month: 'long',
        year: 'numeric',
        timeZone: 'UTC',
      })
    : '';
/** « il y a 12 min », from the page's clock: the same words on the server and in the browser. */
function ago(iso: string): string {
  const min = Math.max(0, Math.round((Date.parse(data.value!.now) - Date.parse(iso)) / 60_000));
  const fr = lang.value === 'fr';
  if (min < 60) return fr ? `il y a ${min} min` : `${min} min ago`;
  const h = Math.round(min / 60);
  if (h < 48) return fr ? `il y a ${h} h` : `${h} h ago`;
  const d = Math.round(h / 24);
  return fr ? `il y a ${d} jours` : `${d} days ago`;
}
const short = (sha: string | null) => (sha ?? '').slice(0, 7);
/** A check's words cut at their `code`, to show it as code without v-html. */
const parts = (words: string | undefined) => (words ?? '').split('`').map((t, n) => ({ t, code: n % 2 === 1 }));
/** A permission's words as the banner shows them: the value alone (`agent.created`, `hooks.slack.com`). */
const permission = (w: string) => w.replace(/^[a-z]+:/, '');
const versionState = (v: VersionRow) =>
  v.state === 'published'
    ? v.yankedAt
      ? 'yanked'
      : v.reviewState === 'verified' || v.reviewState === 'changes' || v.reviewState === 'refused'
        ? v.reviewState
        : v.review
          ? 'review'
          : 'published'
    : v.state;
/** The bars of the weeks' chart, in the SVG's units (no style attribute under the CSP). */
const bars = computed(() => {
  const weeks = data.value?.selected?.weeks ?? [];
  const top = Math.max(1, ...weeks.map((w) => w.installs));
  return weeks.map((w, i) => {
    const h = w.installs ? Math.max(3, Math.round((w.installs / top) * 54)) : 1;
    return { x: i * 10, y: 54 - h, h };
  });
});
const action = (id: string, what: string) => `/api/account/plugins/${encodeURIComponent(id)}/${what}`;
const versionAction = (id: number, what: string) => `/api/account/versions/${id}/${what}`;
const off = computed(() => error.value?.statusCode === 503);
const notice = computed(() => {
  const key = typeof route.query.notice === 'string' ? route.query.notice : '';
  return text.value.notices[key] ?? null;
});
const signIn = computed(() => `/api/auth/github?to=${encodeURIComponent(here.value)}`);
const install = computed(() => `/api/auth/github?install=1&to=${encodeURIComponent(here.value)}`);
useHead(() => ({
  title: data.value ? `${text.value.mine} · ${data.value.login}` : text.value.guest.title,
  meta: [{ name: 'robots', content: 'noindex' }],
}));
</script>

<template>
  <div v-if="data" class="wrap acct-layout">
    <section>
      <h1 class="page-title">
        {{ text.mine }} <span class="more">· {{ data.login }}</span>
      </h1>
      <p v-if="notice" class="notice" :class="notice[0]" role="status">{{ notice[1] }}</p>

      <div v-for="o in data.offers" :key="`o${o.id}`" class="banner">
        <PluginTile :mark="markOf(o.id.split('.').pop() ?? o.id)" :hue="hueOf(o.id)" :icon="o.icon" size="sm" />
        <div>
          <b>{{ a.offer(o.from ?? '?', o.id) }}</b>
          <div class="sub">{{ a.offerSub }}</div>
        </div>
        <div class="acts">
          <form method="post" :action="action(o.id, 'refuse')">
            <input type="hidden" name="lang" :value="lang" />
            <button class="btn sm ghost" type="submit">{{ a.refuse }}</button>
          </form>
          <form method="post" :action="action(o.id, 'accept')">
            <input type="hidden" name="lang" :value="lang" />
            <button class="btn sm primary" type="submit">{{ a.accept }}</button>
          </form>
        </div>
      </div>
      <div v-for="c in data.claims" :key="`c${c.id}`" class="banner">
        <PluginTile :mark="markOf(c.id.split('.').pop() ?? c.id)" :hue="hueOf(c.id)" :icon="c.icon" size="sm" />
        <div>
          <b>{{ a.claim(c.repository, c.id) }}</b>
          <div class="sub">{{ a.claimSub }}</div>
        </div>
        <div class="acts">
          <form method="post" :action="action(c.id, 'claim')">
            <input type="hidden" name="lang" :value="lang" />
            <button class="btn sm primary" type="submit">{{ a.claimNow }}</button>
          </form>
        </div>
      </div>
      <div v-for="p in data.pending" :key="`p${p.id}`" class="banner">
        <PluginTile :mark="markOf(p.name)" :hue="hueOf(p.plugin)" :icon="p.icon" size="sm" />
        <div>
          <b>{{ p.name }} v{{ p.version }}</b> {{ a.pending.passed }}
          <div class="sub">
            {{ a.pending.pushed(ago(p.createdAt)) }} ·
            <span class="mono">{{ p.previousCommit ? `${short(p.previousCommit)} → ${short(p.commit)}` : short(p.commit) }}</span
            ><template v-if="p.added.length">
              · {{ a.pending.more(p.added.length) }}
              <span v-for="(w, n) in p.added" :key="w" class="mono"
                >{{ permission(w) }}{{ n < p.added.length - 1 ? ', ' : '' }}</span
              ></template
            ><template v-if="p.frozenUntil"> · {{ a.pending.frozen(date(p.frozenUntil)) }}</template>
          </div>
        </div>
        <div class="acts">
          <form method="post" :action="versionAction(p.id, 'dismiss')">
            <input type="hidden" name="lang" :value="lang" />
            <button class="btn sm ghost" type="submit">{{ a.pending.dismiss }}</button>
          </form>
          <form method="post" :action="versionAction(p.id, 'publish')">
            <input type="hidden" name="lang" :value="lang" />
            <button class="btn sm primary" type="submit" :disabled="!!p.frozenUntil">{{ a.pending.publish(p.version) }}</button>
          </form>
        </div>
      </div>

      <table class="t mine">
        <thead>
          <tr>
            <th v-for="(h, n) in text.columns" :key="h" :class="{ r: n > 2, 'hide-sm': n > 1 }">{{ h }}</th>
            <th class="act">
              <span class="sr">{{ a.manage('') }}</span>
            </th>
          </tr>
        </thead>
        <tbody>
          <tr v-if="!rows.length" class="empty">
            <td colspan="6">
              <div>
                <b>{{ text.plugins.none }}</b>
                <span>{{ text.plugins.soon }}</span>
                <NuxtLink class="btn sm primary" :to="submit">{{ text.plugins.submit }}</NuxtLink>
              </div>
            </td>
          </tr>
          <template v-for="r in rows" :key="r.id">
            <tr :class="{ sel: r.id === data.selected?.id }">
              <td>
                <div class="pl">
                  <PluginTile :mark="markOf(r.name)" :hue="hueOf(r.id)" :icon="r.icon" size="sm" />
                  <span
                    ><b>{{ r.name }}</b
                    ><small class="mono">{{ r.id }}</small></span
                  >
                </div>
              </td>
              <td>
                <VerifiedSeal v-if="r.state === 'verified'" :version="r.version ?? undefined" />
                <span v-else class="tag" :class="a.states[r.state]?.[0]">{{ a.states[r.state]?.[1] ?? r.state }}</span>
              </td>
              <td class="mono hide-sm">{{ r.version ? `v${r.version}` : '—' }}</td>
              <td class="r num hide-sm">{{ num(r.installs) }}</td>
              <td class="r num hide-sm dim">{{ r.submission === null ? (r.recent ? `+${num(r.recent)}` : '0') : '—' }}</td>
              <td class="act">
                <NuxtLink
                  v-if="r.submission === null"
                  class="btn sm ghost icon"
                  :to="{ path: here, query: { plugin: r.id }, hash: '#manage' }"
                  :aria-label="a.manage(r.name)"
                  :title="a.manage(r.name)"
                  ><svg class="i" viewBox="0 0 16 16" aria-hidden="true">
                    <circle cx="3.5" cy="8" r="1.1" />
                    <circle cx="8" cy="8" r="1.1" />
                    <circle cx="12.5" cy="8" r="1.1" /></svg
                ></NuxtLink>
                <NuxtLink
                  v-else
                  class="btn sm ghost icon"
                  :to="{ path: submit, query: { version: r.submission } }"
                  :aria-label="a.finish"
                  :title="a.finish"
                  ><svg class="i" viewBox="0 0 16 16" aria-hidden="true"><path d="M6 3.5L10.5 8L6 12.5" /></svg
                ></NuxtLink>
              </td>
            </tr>
            <tr v-if="r.rejection" class="why-row">
              <td colspan="6">
                <div class="why">
                  <b>{{ a.rejected(r.rejection.name[lang], r.version ?? '') }}</b>
                  <template v-for="(p, n) in parts(r.rejection.why?.[lang] ?? r.rejection.detail[lang])" :key="n"
                    ><code v-if="p.code">{{ p.t }}</code
                    ><template v-else>{{ p.t }}</template></template
                  >
                  <template v-if="r.rejection.path">
                    (<span class="mono">{{ r.rejection.path }}</span
                    >)</template
                  >
                  <div class="fix">
                    <template v-for="(p, n) in parts(r.rejection.fix?.[lang])" :key="n"
                      ><code v-if="p.code">{{ p.t }}</code
                      ><template v-else>{{ p.t }}</template></template
                    >
                    <a class="arrow" :href="r.rejection.docs[lang]">{{ a.checks }}</a>
                  </div>
                </div>
              </td>
            </tr>
          </template>
        </tbody>
      </table>
      <p class="sub note">{{ text.installsNote }}</p>

      <section v-if="data.selected && chosen" id="manage" class="manage" aria-labelledby="manage-title">
        <div class="manage-head">
          <PluginTile :mark="markOf(chosen.name)" :hue="hueOf(chosen.id)" :icon="chosen.icon" />
          <div>
            <h2 id="manage-title" class="h2">{{ chosen.name }}</h2>
            <small class="mono">{{ chosen.id }}</small>
          </div>
        </div>
        <p class="sub">{{ a.from(chosen.repository) }}</p>
        <p v-if="chosen.transferredAt && chosen.transferredFrom" class="sub">
          {{ a.transferred(chosen.transferredFrom, date(chosen.transferredAt)) }}
        </p>
        <p v-if="chosen.frozenUntil" class="notice warn">{{ a.frozenUntil(date(chosen.frozenUntil)) }}</p>
        <div v-if="chosen.deletedAt" class="notice bad deleted">
          <span>{{ a.deletedOn(date(chosen.purgeOn)) }}</span>
          <form method="post" :action="action(chosen.id, 'restore')">
            <input type="hidden" name="lang" :value="lang" />
            <button class="btn sm" type="submit">{{ a.restore }}</button>
          </form>
        </div>

        <table class="t versions">
          <thead>
            <tr>
              <th v-for="(h, n) in a.versions" :key="n" :class="{ r: n > 2, 'hide-sm': n === 2 || n === 4 }">{{ h }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="v in data.selected.versions" :key="v.id">
              <td class="mono">{{ v.version ? `v${v.version}` : v.tag }}</td>
              <td>
                <span
                  class="tag"
                  :class="{
                    bad: v.state === 'rejected' || v.state === 'error',
                    warn: v.state === 'ready' || v.state === 'held',
                    review: versionState(v) === 'review',
                    ok: versionState(v) === 'verified',
                  }"
                  >{{ a.vstates[versionState(v)] ?? v.state }}</span
                >
                <span v-if="v.tagMovedAt" class="tag warn moved">{{ a.moved }}</span>
                <small v-if="v.reviewMessage && (v.reviewState === 'changes' || v.reviewState === 'refused')" class="review-said">{{
                  v.reviewMessage
                }}</small>
              </td>
              <td class="hide-sm dim nowrap">{{ date(v.publishedAt) || '—' }}</td>
              <td class="r num">{{ num(v.installs) }}</td>
              <td class="r num hide-sm dim">{{ num(v.updates) }}</td>
              <td class="act">
                <details v-if="v.state === 'published' && !v.yankedAt && v.version" class="yank">
                  <summary class="btn sm ghost">{{ a.yank }}</summary>
                  <form method="post" :action="versionAction(v.id, 'yank')">
                    <p>{{ a.yankWhat(v.version) }}</p>
                    <input type="hidden" name="lang" :value="lang" />
                    <button class="btn sm danger" type="submit">{{ a.yankNow(v.version) }}</button>
                  </form>
                </details>
              </td>
            </tr>
          </tbody>
        </table>

        <div v-if="!chosen.deletedAt" class="rows">
          <div class="row">
            <div>
              <b>{{ a.auto.title }}</b>
              <p>{{ data.selected.autoPublish ? a.auto.on : a.auto.off }}</p>
            </div>
            <form method="post" :action="action(chosen.id, data.selected.autoPublish ? 'auto-off' : 'auto-on')">
              <input type="hidden" name="lang" :value="lang" />
              <button class="btn sm" type="submit">{{ data.selected.autoPublish ? a.auto.turnOff : a.auto.turnOn }}</button>
            </form>
          </div>
          <div class="row">
            <div>
              <b>{{ a.list.title }}</b>
              <p>{{ chosen.unlistedAt ? a.list.off : a.list.on }}</p>
            </div>
            <form method="post" :action="action(chosen.id, chosen.unlistedAt ? 'relist' : 'unlist')">
              <input type="hidden" name="lang" :value="lang" />
              <button class="btn sm" type="submit">{{ chosen.unlistedAt ? a.list.relist : a.list.unlist }}</button>
            </form>
          </div>
          <div class="row">
            <div>
              <b>{{ a.transfer.title }}</b>
              <p>{{ chosen.transferTo ? a.transfer.waiting(chosen.transferTo) : a.transfer.about }}</p>
            </div>
            <form v-if="chosen.transferTo" method="post" :action="action(chosen.id, 'cancel-transfer')">
              <input type="hidden" name="lang" :value="lang" />
              <button class="btn sm" type="submit">{{ a.transfer.cancel }}</button>
            </form>
            <form v-else method="post" class="inline" :action="action(chosen.id, 'transfer')">
              <label class="sr" for="transfer-login">{{ a.transfer.label }}</label>
              <input
                id="transfer-login"
                class="input"
                name="login"
                required
                maxlength="40"
                autocomplete="off"
                :placeholder="a.transfer.label"
              />
              <input type="hidden" name="lang" :value="lang" />
              <button class="btn sm" type="submit">{{ a.transfer.send }}</button>
            </form>
          </div>
          <div class="row">
            <div>
              <b>{{ a.remove.title }}</b>
              <p>{{ a.remove.about }}</p>
            </div>
            <details class="delete">
              <summary class="btn sm danger">{{ a.remove.open }}</summary>
              <form method="post" :action="action(chosen.id, 'delete')">
                <label for="confirm-plugin">{{ a.remove.confirm(chosen.id) }}</label>
                <input id="confirm-plugin" class="input" name="confirm" required autocomplete="off" :placeholder="a.remove.label" />
                <input type="hidden" name="lang" :value="lang" />
                <button class="btn sm danger" type="submit">{{ a.remove.now }}</button>
              </form>
            </details>
          </div>
        </div>
      </section>

      <section class="app" aria-labelledby="app-title">
        <div class="app-head">
          <h2 id="app-title" class="h2">{{ text.app.title }}</h2>
          <a class="btn sm" :href="install">{{ data.installations.length ? text.app.change : text.app.install }}</a>
        </div>
        <p v-if="!data.installations.length" class="sub">{{ text.app.none }}</p>
        <ul v-else class="installs">
          <li v-for="i in data.installations" :key="i.id">
            <div class="who">
              <b>{{ i.owner }}</b>
              <small v-if="i.ownerType === 'Organization'">{{ text.app.org }}</small>
              <span v-if="i.suspended" class="tag bad">{{ text.app.suspended }}</span>
              <span class="grow"></span>
              <small>{{ i.all ? text.app.all : text.app.repos(i.repos.length) }}</small>
            </div>
            <ul v-if="i.repos.length" class="repos">
              <li v-for="r in i.repos" :key="r" class="mono">{{ r }}</li>
            </ul>
          </li>
        </ul>
      </section>
    </section>

    <aside>
      <div v-if="data.selected && chosen" class="side-card stats">
        <h3 class="h2">{{ a.stats.title(chosen.name) }}</h3>
        <div>
          <span class="stat-n num">{{ num(data.selected.installs) }}</span>
          <span class="sub small">{{ a.stats.recent(num(data.selected.recent)) }}</span>
        </div>
        <svg
          class="bars"
          viewBox="0 0 119 54"
          preserveAspectRatio="none"
          role="img"
          :aria-label="a.stats.chart(num(data.selected.weeks.at(-1)?.installs ?? 0))"
        >
          <rect v-for="(b, n) in bars" :key="n" :x="b.x" :y="b.y" width="9" :height="b.h" rx="1" :class="{ last: n === bars.length - 1 }" />
        </svg>
        <div class="axis">
          <span>{{ a.stats.first }}</span
          ><span>{{ a.stats.last }}</span>
        </div>
        <p>{{ a.stats.updates(num(data.selected.updates)) }}</p>
      </div>

      <div class="side-card">
        <h3 class="h2">{{ text.email.title }}</h3>
        <div v-if="data.email" class="field">
          <label>{{ text.email.label }}</label>
          <div class="input">
            <span class="addr">{{ data.email }}</span
            ><span class="ok">{{ text.email.verified }}</span>
          </div>
        </div>
        <form class="field" method="post" action="/api/account/email">
          <label v-if="!data.email" for="email">{{ text.email.label }}</label>
          <input
            id="email"
            class="input"
            type="email"
            name="email"
            required
            maxlength="254"
            autocomplete="email"
            :placeholder="text.email.placeholder"
          />
          <input type="hidden" name="lang" :value="lang" />
          <button class="btn sm" type="submit">{{ data.email ? text.email.change : text.email.send }}</button>
        </form>
        <p v-if="data.pendingEmail" class="pending">{{ text.email.pending(data.pendingEmail) }}</p>
        <p>{{ text.email.about }}</p>
        <p v-if="!data.mail" class="warn">{{ text.email.off }}</p>
        <form v-if="data.email" method="post" action="/api/account/email">
          <input type="hidden" name="action" value="remove" />
          <input type="hidden" name="lang" :value="lang" />
          <button class="btn sm ghost" type="submit">{{ text.email.remove }}</button>
        </form>
      </div>

      <div class="side-card">
        <h3 class="h2">{{ text.data.title }}</h3>
        <p>{{ text.data.about }}</p>
        <div class="acts">
          <a class="btn sm" href="/api/account/export" download
            ><svg class="i" viewBox="0 0 16 16" aria-hidden="true"><path d="M8 2.5V10.5M4.5 7L8 10.5L11.5 7M3 13.5H13" /></svg
            >{{ text.data.export }}</a
          >
          <form method="post" action="/api/auth/logout">
            <input type="hidden" name="to" :value="pathIn(lang, '/')" />
            <button class="btn sm" type="submit">{{ text.signOut }}</button>
          </form>
        </div>
        <p v-if="inTheWay.length" class="warn">{{ a.deleteFirst(inTheWay.join(', ')) }}</p>
        <details v-else class="delete">
          <summary class="btn sm danger">{{ text.data.delete }}</summary>
          <form method="post" action="/api/account/delete">
            <label for="confirm">{{ text.data.confirm(data.login) }}</label>
            <input id="confirm" class="input" name="confirm" required autocomplete="off" :placeholder="text.data.confirmLabel" />
            <input type="hidden" name="lang" :value="lang" />
            <button class="btn sm danger" type="submit">{{ text.data.deleteNow }}</button>
          </form>
        </details>
        <p class="links">
          <a :href="links.privacy">{{ text.data.privacy }}</a> · <NuxtLink :to="terms">{{ text.data.terms }}</NuxtLink>
        </p>
      </div>
      <AdminTokens v-if="me.account?.admin" />
    </aside>
  </div>

  <div v-else class="wrap guest">
    <h1 class="page-title">{{ text.guest.title }}</h1>
    <p v-if="notice" class="notice" :class="notice[0]" role="status">{{ notice[1] }}</p>
    <p class="lead">{{ text.guest.lead }}</p>
    <p v-if="off" class="notice warn">{{ text.guest.off }}</p>
    <a v-else class="btn primary" :href="signIn"
      ><svg class="i" viewBox="0 0 16 16" aria-hidden="true">
        <circle cx="4.5" cy="3.5" r="1.6" />
        <circle cx="4.5" cy="12.5" r="1.6" />
        <circle cx="11.5" cy="6" r="1.6" />
        <path d="M4.5 5.1V10.9M11.5 7.6C11.5 9.6 8 9.4 4.5 10.9" /></svg
      >{{ text.signIn }}</a
    >
    <p class="sub small">
      {{ text.guest.cookie }} <a :href="links.privacy">{{ base.footer.privacy }}</a> ·
      <NuxtLink :to="terms">{{ text.data.terms }}</NuxtLink>
    </p>
  </div>
</template>

<style scoped>
.acct-layout {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 300px;
  gap: 56px;
  padding-top: 36px;
}
.acct-layout > * {
  min-width: 0;
}
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
.mono {
  font-family: var(--mono);
  font-size: 0.86em;
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
table.t {
  width: 100%;
  margin-top: 28px;
  border-collapse: collapse;
  table-layout: fixed;
  font-size: 14px;
}
/* The preview's columns, kept when the table is still empty. */
table.mine th:nth-child(2) {
  width: 24%;
}
table.mine th:nth-child(3) {
  width: 15%;
}
table.mine th:nth-child(4) {
  width: 13%;
}
table.mine th:nth-child(5) {
  width: 10%;
}
table.t th.act {
  width: 52px;
}
table.t td.act {
  padding-right: 0;
  text-align: right;
}
table.t td.r {
  text-align: right;
}
table.t tr:has(+ .why-row) td {
  border-bottom: 0;
}
table.t tr.why-row td {
  padding-top: 0;
}
table.t tr.sel td {
  background: var(--surface);
}
.num {
  font-variant-numeric: tabular-nums;
}
.dim {
  color: var(--dim);
}
.nowrap {
  white-space: nowrap;
}
.sr {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}
.pl {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
}
.pl > span {
  min-width: 0;
}
.pl b {
  font-weight: 600;
}
.pl small {
  display: block;
  overflow: hidden;
  color: var(--dim);
  font-size: 12.5px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.btn.icon {
  width: 30px;
  padding: 0;
  justify-content: center;
}
.banner {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  gap: 14px;
  margin: 24px 0 0;
  padding: 14px 16px;
  border: 1px solid var(--line-strong);
  border-radius: 12px;
  background: var(--surface);
  font-size: 14px;
}
.banner + .banner {
  margin-top: 12px;
}
.banner .sub {
  font-size: 13px;
}
.banner .acts {
  flex-wrap: nowrap;
}
.banner button[disabled] {
  cursor: not-allowed;
  opacity: 0.5;
}
.why {
  max-width: 60ch;
  margin: 0 0 0 40px;
  padding: 12px 14px;
  border: 1px solid color-mix(in oklch, var(--danger) 35%, transparent);
  border-radius: 10px;
  background: var(--danger-bg);
  color: var(--soft);
  font-size: 13.5px;
}
.why b {
  color: var(--danger);
  font-weight: 600;
}
.why .fix {
  margin-top: 6px;
  color: var(--muted);
}
.why .arrow {
  color: var(--text);
  text-decoration: underline;
  text-underline-offset: 3px;
}
.manage {
  margin-top: 40px;
  scroll-margin-top: 80px;
}
.manage-head {
  display: flex;
  align-items: center;
  gap: 14px;
  margin-bottom: 10px;
}
.manage-head small {
  color: var(--dim);
}
.manage > .sub {
  margin: 4px 0 0;
  font-size: 13.5px;
}
.manage .notice {
  margin-top: 14px;
}
.notice.deleted {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}
table.versions {
  margin-top: 20px;
}
table.versions td {
  padding-block: 12px;
}
table.versions th:nth-child(1) {
  width: 18%;
}
table.versions th:nth-child(2) {
  width: 30%;
}
table.versions th:nth-child(6) {
  width: 92px;
}
.tag.moved {
  margin-left: 6px;
}
.yank {
  position: relative;
  display: inline-block;
}
.yank summary,
.delete summary {
  display: inline-flex;
  list-style: none;
}
.yank summary::-webkit-details-marker {
  display: none;
}
.yank form {
  position: absolute;
  z-index: 2;
  top: 36px;
  right: 0;
  display: grid;
  gap: 10px;
  width: min(320px, 80vw);
  padding: 14px;
  border: 1px solid var(--line-strong);
  border-radius: 10px;
  background: var(--bg);
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.25);
  text-align: left;
}
.yank form p {
  margin: 0;
  color: var(--muted);
  font-size: 13px;
}
.yank .btn {
  justify-self: start;
}
.rows {
  display: grid;
  margin-top: 28px;
  border-top: 1px solid var(--line);
}
.row {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px 24px;
  padding: 16px 0;
  border-bottom: 1px solid var(--line);
}
.row > div {
  flex: 1 1 280px;
  min-width: 0;
}
.row b {
  font-size: 14px;
  font-weight: 600;
}
.row p {
  max-width: 62ch;
  margin: 4px 0 0;
  color: var(--muted);
  font-size: 13.5px;
}
.row form.inline {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.row form.inline .input {
  width: 200px;
  height: 30px;
}
.row .delete form {
  min-width: 260px;
}
.stats .small {
  margin-left: 4px;
  font-size: 13px;
}
.stat-n {
  font-size: 26px;
  font-weight: 620;
  letter-spacing: -0.03em;
}
.bars {
  display: block;
  width: 100%;
  height: 54px;
}
.bars rect {
  fill: color-mix(in srgb, var(--text) 22%, transparent);
}
.bars rect.last {
  fill: var(--brand);
}
.axis {
  display: flex;
  justify-content: space-between;
  color: var(--dim);
  font: 11px var(--mono);
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
table.t th:first-child,
table.t td:first-child {
  padding-left: 4px;
}
table.t th.r {
  text-align: right;
}
table.t td {
  padding: 14px 12px;
  border-bottom: 1px solid var(--line);
  vertical-align: top;
}
table.t tr.empty td {
  padding: 18px 12px 18px 4px;
}
.empty div {
  display: grid;
  gap: 4px;
  max-width: 60ch;
}
.empty span {
  color: var(--muted);
  font-size: 13.5px;
}
.note {
  margin-top: 14px;
  font-size: 13px;
}
.app {
  margin-top: 40px;
}
.app-head {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
}
.installs {
  display: grid;
  gap: 10px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.installs > li {
  padding: 14px 16px;
  border: 1px solid var(--line);
  border-radius: 12px;
}
.who {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  font-size: 14px;
}
.who small {
  color: var(--dim);
  font-size: 12.5px;
}
.grow {
  flex: 1;
}
.repos {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 16px;
  margin: 8px 0 0;
  padding: 0;
  color: var(--soft);
  list-style: none;
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
.tag.review {
  border-style: dashed;
}
.tag.warn {
  border-color: color-mix(in oklch, var(--warn) 45%, transparent);
  background: var(--warn-bg);
  color: var(--warn);
}
.review-said {
  display: block;
  max-width: 46ch;
  margin-top: 4px;
  color: var(--dim);
  font-size: 12.5px;
  white-space: pre-line;
}
.tag.ok {
  border-color: color-mix(in oklch, var(--ok) 40%, transparent);
  color: var(--ok);
}
.tag.bad {
  border-color: color-mix(in oklch, var(--danger) 45%, transparent);
  background: var(--danger-bg);
  color: var(--danger);
}
.side-card {
  display: grid;
  align-content: start;
  gap: 14px;
  padding: 20px;
  border: 1px solid var(--line);
  border-radius: 12px;
  font-size: 14px;
}
.side-card + .side-card {
  margin-top: 16px;
}
.side-card p {
  margin: 0;
  color: var(--muted);
  font-size: 13.5px;
}
.side-card p.warn {
  color: var(--warn);
}
.side-card p.pending {
  color: var(--soft);
}
.field {
  display: grid;
  gap: 6px;
}
.field label,
.delete label {
  color: var(--dim);
  font-size: 12.5px;
  font-weight: 600;
}
.input {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 36px;
  padding: 0 11px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: var(--surface);
  font-size: 14px;
}
input.input {
  width: 100%;
}
input.input:focus-visible {
  border-color: var(--brand);
  outline: none;
}
.addr {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.input .ok {
  margin-left: auto;
  color: var(--ok);
  font-size: 12.5px;
}
.btn.sm {
  height: 30px;
  padding: 0 11px;
  border-radius: 7px;
  font-size: 13px;
  font-weight: 600;
}
.field .btn,
.delete .btn {
  justify-self: start;
}
.btn.ghost {
  border-color: transparent;
  background: none;
  color: var(--muted);
}
.btn.danger {
  color: var(--danger);
}
.acts {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.delete summary {
  display: inline-flex;
  list-style: none;
}
.delete summary::-webkit-details-marker {
  display: none;
}
.delete form {
  display: grid;
  gap: 8px;
  margin-top: 12px;
}
.links {
  font-size: 13px;
}
.links a:hover {
  color: var(--text);
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
.guest .small {
  font-size: 13px;
}
.guest .small a {
  text-decoration: underline;
  text-underline-offset: 3px;
}
@media (max-width: 1180px) {
  .acct-layout {
    grid-template-columns: minmax(0, 1fr) 260px;
    gap: 36px;
  }
}
@media (max-width: 720px) {
  .acct-layout {
    grid-template-columns: 1fr;
  }
  table.t thead {
    display: none;
  }
  .hide-sm {
    display: none;
  }
  .banner {
    grid-template-columns: auto minmax(0, 1fr);
  }
  .banner .acts {
    grid-column: 1 / -1;
  }
  .banner .sub {
    overflow-wrap: anywhere;
  }
  .why {
    margin-left: 0;
  }
  /* The preview's rows on a phone: the plugin and its action on a line, its state below. */
  .mine tr {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    border-bottom: 1px solid var(--line);
  }
  .mine tr:has(+ .why-row) {
    border-bottom: 0;
  }
  .mine tr.sel {
    background: var(--surface);
  }
  .mine td {
    border: 0 !important;
  }
  .mine td:nth-child(2) {
    grid-row: 2;
    padding: 0 12px 14px 44px;
  }
  .mine td:last-child {
    grid-column: 2;
    grid-row: 1;
  }
  .mine tr.why-row,
  .mine tr.empty {
    display: block;
  }
  .row form.inline .input {
    width: 100%;
  }
}
</style>
