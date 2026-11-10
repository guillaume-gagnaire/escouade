import type { Tree } from '../types';

// The temperature of an agent's cache (ESC-398): on its card, in its conversation's header, in the status bar.
export default {
  /** The figure: « 72 °C », « — °C » when its harness says nothing of its cache. */
  degrees: '{n} °C',
  unknown: '— °C',
  /** Above the figure in the header. */
  label: 'Cache',
  /** What a screen reader says, and the tooltip of the card: the figure, then what it means. */
  aria: {
    warm: 'Cache : {n} °C, expire dans {left}',
    expired: 'Cache : 0 °C, expiré : le prochain message relira tout au plein tarif',
    broken: 'Cache : 0 °C, cassé ({why}) : le prochain message relira tout au plein tarif',
    unknown: 'Cache : — °C, {engine} ne dit rien de la durée de vie de son cache',
    /** No turn of it told anything cached yet (or its harness never does). */
    none: 'Cache : — °C, {engine} n’a encore rien dit de son cache',
  },
  /** Why the cache broke. */
  why: {
    model: 'modèle changé',
    compaction: 'contexte compacté',
    prompt: 'consigne système modifiée',
    account: 'compte changé',
    conversation: 'nouvelle conversation',
  },
  /** A remaining time: « 3 min 40 », « 40 s ». */
  left: { minutes: '{m} min {s}', seconds: '{s} s', hours: '{h} h {m}' },
  /** The detail shown over the header. */
  detail: {
    title: 'Cache : {n} °C',
    titleUnknown: 'Cache : — °C',
    /** `{ago}`: « il y a 3 min » ; `{left}`: « 1 min 20 » ; `{engine}`: Codex ; `{life}`: « 5 min ». */
    warm: 'Dernier tour {ago} · expire dans {left} (le cache de {engine} vit {life} sans activité).',
    expired: 'Dernier tour {ago} · expiré (le cache de {engine} vit {life} sans activité).',
    broken: 'Dernier tour {ago} · cassé : {why}.',
    unknownLife: 'Dernier tour {ago} · {engine} ne dit pas combien de temps vit son cache.',
    life: { minutes: '{n} min', hours: { one: '{count} heure', other: '{count} heures' } },
    read: 'Lus depuis le cache (chauds)',
    written: 'Mis en cache (froids)',
    cached: 'Lus ou mis en cache',
    fresh: 'Nouveaux, plein tarif',
    saved: 'Économisé sur ce tour',
    /** `{tokens}`: « 142 k » ; `{cold}` and `{warm}`: amounts. */
    next: 'Si le prochain message part à froid : ≈ {tokens} tokens relus, ≈ {cold} au lieu de ≈ {warm}.',
    nextTokens: 'Si le prochain message part à froid : ≈ {tokens} tokens relus au plein tarif.',
  },
  /** In the status bar: what the cache saved today, all agents together. */
  today: 'Cache aujourd’hui : ≈ {amount} économisés',
  todayHint: 'Ce que le cache des agents a fait économiser aujourd’hui, d’après les tarifs publics',
} as const satisfies Tree;
