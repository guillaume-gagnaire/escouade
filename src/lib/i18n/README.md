# Les textes de la fenêtre en deux langues

Tout texte qu’une personne lit dans la fenêtre (texte, `title`, `aria-label`, `placeholder`, prop `label` / `desc` / `hint`…) vient d’un catalogue, en français et en anglais. Le français est la source : ses clés sont celles que `t` accepte ; l’anglais est vérifié contre lui à la compilation (`npm run check`). Les mots fixes sont dans `GLOSSARY.md`.

## Où sont les textes

```
src/lib/i18n/
  fr/<zone>.ts    export default { … } as const satisfies Tree;
  en/<zone>.ts    export default defineZone('<zone>', { … });
  pending/<lot>.txt   les fichiers que chaque lot doit encore extraire
```

Une zone appartient à une seule tâche à la fois : `editor` (L3), `settings` (L4), `integrations` et `boardSettings` (L5), `board` et `stats` (L6), `nav` et `git` (L7), `conv` et `composer` (L8), `runs` et `shell` (L9), `errors` (L10), `accounts` (K), `mcp` (M), `branches` (G). `common` et `format` sont remplis (L1) et ne bougent plus : ce qui te manque va dans ta zone, même un mot déjà dans `common` avec un autre sens.

## Ajouter un texte

1. Dans `fr/<zone>.ts`, puis au même chemin dans `en/<zone>.ts` :

   ```ts
   tabs: {
     closeOthers: 'Fermer les autres',
     unsaved: 'Le fichier {name} n’est pas enregistré.',
     closeCount: { one: 'Fermer {count} onglet', other: 'Fermer {count} onglets' },
   },
   ```

2. Clés en camelCase, `zone.composant.sens` (`editor.tabs.closeOthers`) ; une phrase entière par clé, jamais deux morceaux recollés.
3. Paramètres `{nom}` (lettres, chiffres, `_`). Les accolades ne servent qu’à ça (`catalogs.test` refuse une `{` seule) ; `{k}` est interdit (la clé de `Rich`) ; `one` et `other` sont réservés aux pluriels.
4. Pluriel : une feuille `{ one, other }`, choisie par `Intl.PluralRules` sur `count`. En français, 0 et 1 sont au singulier : écris `{count}` dans le `one` français ; l’anglais peut l’omettre (« one file »). Les deux langues ont les mêmes paramètres (toutes formes confondues, `count` à part).
5. L’anglais doit avoir exactement les mêmes clés et les mêmes `{paramètres}` : sinon `npm run check` échoue à la ligne fautive du fichier anglais (`placeholdersMustBe`, `notInFrenchCatalog`, `notAPluralForm`, `Property '…' is missing`).

## Lire un texte

```svelte
<script lang="ts">
  import { t } from '../lib/i18n';
  import { fInt } from '../lib/format';
  const count = $derived(t('editor.tabs.closeCount', { count: n }));
</script>

<button title={t('common.close')}>×</button>
<p>{t('editor.tabs.unsaved', { name: file })}</p>
```

- `t(key)` sans paramètre, `t(key, { … })` avec tous ceux du texte (exigés et vérifiés : ni en moins, ni en trop) ; `count: number` pour un pluriel. `tIn('en', key, …)` donne une langue précise (tests, textes envoyés à Claude).
- `t` lit la langue de l’interface : dans un gabarit ou un `$derived`, le texte change avec elle. **Jamais de texte calculé au chargement d’un module** : une table de libellés devient une fonction ou un getter.

  ```ts
  // Avant : figé en français au chargement.
  export const APPROVE_LABEL: Record<BoardAction, string> = { merge: 'Valider et merger', … };
  // Après : lu à chaque rendu.
  export const approveLabel = (action: BoardAction) => t(`boardSettings.approve.${action}`);
  ```

- Les valeurs sont écrites par `String(v)` : un nombre n’est pas groupé. Pour « 1 234 fichiers », passe le compte formaté à part : feuille `{ one: '{n} fichier', other: '{n} fichiers' }` et `t(key, { count: n, n: fInt(n) })`. Un montant, une durée, une date : formate-les avec `format.ts` et passe le résultat en paramètre.

## Du balisage au milieu d’une phrase : `Rich`

```svelte
<!-- composer.sendHint : 'Appuie sur {key} pour envoyer.' -->
<Rich k="composer.sendHint">{#snippet key()}<kbd>{keyLabel('Ctrl+Enter')}</kbd>{/snippet}</Rich>
<!-- git.changedIn : { one: '{count} fichier modifié dans {branch}', other: '{count} fichiers modifiés dans {branch}' } -->
<Rich k="git.changedIn" count={n}>{#snippet branch()}<code>{name}</code>{/snippet}</Rich>
<!-- un paramètre texte se passe en prop : -->
<Rich k="git.renamed" from={a}>{#snippet to()}<b>{b}</b>{/snippet}</Rich>
```

`k` est la clé ; chaque `{nom}` du texte est une prop **exigée** : un extrait (`{#snippet nom()}` dans `<Rich>`) ou un texte (écrit comme texte, jamais comme HTML). Un extrait oublié ne compile pas. Le `count` d’un pluriel est un nombre. Pour un lien, son texte est un paramètre de plus si la phrase le demande (`{#snippet link()}<a …>{t('zone.linkText')}</a>{/snippet}`). Sans balisage, `t` avec ses paramètres suffit.

## Nombres, dates, tailles, raccourcis

Toujours `src/lib/format.ts`, jamais `toLocaleString('fr-FR')`, `> 1 ? 's' : ''` ni `plural()` (déprécié, le garde-fou signale chaque appel) :

| Fonction | Français | Anglais |
|---|---|---|
| `fTok(n)` | 1,2 k | 1.2k |
| `fUsd(x)` | 0,10 $ | $0.10 |
| `fPct(p)` | 42 % | 42% |
| `fInt(n)` / `fNum(n, décimales)` | 1 235 / 1,3 | 1,235 / 1.3 |
| `fDur(ms)` | 5m 12s, 1h 05m | idem |
| `fCountdown(ts, now)` | 4j 12h, 1h48 | 4d 12h, 1h48 |
| `fAgo(s, now)` | il y a 5 min, hier | 5 min ago, yesterday |
| `fSince(ts, now)` | 3 min, 2 h, 3 j | 3 min, 2 h, 3 days |
| `fDate(ts)` / `fTime(ts)` | 2 octobre 2026 / 09:30 | October 2, 2026 / 9:30 AM |
| `fDateTime(ts)` | 2 octobre 2026 à 09:30 | October 2, 2026 at 9:30 AM |
| `fWhen(ts, now)` | à 15:00, le vendredi 2 octobre à 09:30 | at 3:00 PM, on Friday, October 2 at 9:30 AM |
| `fBytes(n)` / `fBytes(n, 1)` | 312 Mo / 1,3 Mo | 312 MB / 1.3 MB |
| `fList(items)` | a, b et c | a, b, and c |

Un raccourci : `keyLabel('Ctrl+Shift+F')` (« Ctrl+Maj+F », « Ctrl+Shift+F », « ⇧⌘F » sous macOS) ; écris les touches en anglais (`Shift`, `Enter`, `Esc`).

## Tests

- Les tests existants restent en français et leurs attentes ne changent pas : `src/test/setup.ts` remet le français avant chaque test.
- Pour chaque composant principal, un test qui le rend en anglais : `setLang('en')` au début du test (avant le rendu, ou suivi de `flushSync()` après), quelques libellés et un format.

## Le garde-fou et ton fichier `pending`

`hardcoded.test.ts` échoue sur un texte écrit en dur dans un fichier de `src/` qui n’est dans aucune liste de `pending/`. Il signale : un texte avec une lettre dans le balisage ; un attribut lu (`title`, `aria-*`, `placeholder`, `alt`, `label`, `desc`, `note`, `hint`, `caption`) avec une lettre ; une prop de composant de deux mots ; un attribut accentué ; une chaîne qu’une expression du balisage affiche (`{ok ? 'Oui' : 'Non'}`) ; une chaîne accentuée d’un script ; chaque appel à `plural()`.

1. Extrais tes fichiers un par un (tests existants verts après chacun).
2. Supprime `pending/<ton lot>.txt`, puis `npx vitest run src/lib/i18n` : le scan doit passer sur tes fichiers. Ce qu’il voit encore est un texte à extraire.
3. Une exception (`hardcoded-allow.ts`) seulement pour un texte identique dans toutes les langues (nom propre, symbole, unité comme « CPU ») : texte exact, fichier (ou `'*'`), raison (`why`), **dans le bloc de ton lot**. Une exception qui ne sert plus fait échouer le test.
4. Ajoute tes mots à ta section de `GLOSSARY.md`.
