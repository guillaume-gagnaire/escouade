<script setup lang="ts">
import { ACCOUNT_TEXTS } from '~/data/account';
import { pathIn } from '~/data/language';

// The author's account, as the preview draws it (design/store/, #account): « Mes plugins · login » with their plugins
// (STORE-3 and STORE-5 fill the table) and the App's installations, and on the right the e-mail notices and « Mes
// données » (export, sign-out, deletion). Every action is a plain form: it works without script, under the strict
// Content-Security-Policy, and the server answers with the page and a notice.
interface Installation {
  id: number;
  owner: string;
  ownerType: string;
  all: boolean;
  suspended: boolean;
  repos: string[];
}
interface AccountData {
  login: string;
  name: string | null;
  githubId: number;
  email: string | null;
  pendingEmail: string | null;
  installations: Installation[];
  mail: boolean;
}

const { lang, text: base, links } = useLang();
const route = useRoute();
const text = computed(() => ACCOUNT_TEXTS[lang.value]);
const here = computed(() => pathIn(lang.value, '/account'));
const terms = computed(() => pathIn(lang.value, '/terms'));
const { data, error } = await useFetch<AccountData>('/api/account', { key: 'account' });
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

      <table class="t mine">
        <thead>
          <tr>
            <th v-for="(h, n) in text.columns" :key="h" :class="{ r: n > 2 }">{{ h }}</th>
          </tr>
        </thead>
        <tbody>
          <tr class="empty">
            <td colspan="5">
              <div>
                <b>{{ text.plugins.none }}</b>
                <span>{{ text.plugins.soon }}</span>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
      <p class="sub note">{{ text.installsNote }}</p>

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
        <details class="delete">
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
table.t th:nth-child(2) {
  width: 28%;
}
table.t th:nth-child(3) {
  width: 16%;
}
table.t th:nth-child(4) {
  width: 13%;
}
table.t th:nth-child(5) {
  width: 11%;
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
  padding: 18px 12px 18px 4px;
  border-bottom: 1px solid var(--line);
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
  font-size: 12px;
  font-weight: 550;
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
}
</style>
