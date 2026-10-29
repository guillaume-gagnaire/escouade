---
title: "Navigation de code"
subject: "Éditeur"
sources: ["docs/SPEC.md"]
tickets: []
decisions: []
compared: null
order: 26
---

## Ce que c'est

Dans l'éditeur (voir *Éditeur*), aller à ce qu'un mot ou un chemin désigne, sans serveur de langage : le fichier d'un import, d'un lien ou d'un chemin, ou la définition d'un nom. Chaque saut est noté dans un historique qu'on remonte et redescend.

## Comportement

### Suivre ce qui est sous la souris

- `Ctrl+clic` (Cmd+clic) suit ce qui est sous la souris ; `F12` suit ce qui est sous le curseur.
- Touche enfoncée, la zone est soulignée et le curseur devient une main. Relâcher la touche, quitter la zone ou la fenêtre retire le soulignement. Jamais avec AltGr ni Maj.
- Deux niveaux sont essayés dans l'ordre : les liens, puis les définitions.

### Niveau A : les liens

- JS, TS, JSX, TSX, Svelte et Vue : imports relatifs ou aliasés (`import … from`, `export … from`, `import()`, `require()`).
  - Extensions `.ts .tsx .d.ts .js .jsx .mjs .cjs .svelte .vue .json`, puis `index.*` ; un `./x.js` trouve `x.ts`.
  - Alias : `compilerOptions.paths` et `baseUrl` du `tsconfig.json` de la racine, sinon de son `jsconfig.json` (JSON avec commentaires, lus quand la source s'affiche), et `$lib` → `src/lib`.
- CSS et SCSS : `@import`, `@use`, `@forward` (partiels `_x.scss`) et `url()` relatifs.
- Rust : `mod x;` (`x.rs` ou `x/mod.rs`, à côté ou dans le dossier du module parent).
- Python : `from .x import`, `from ..x.y import`, `import a.b` (depuis la racine de la source ; `x.py` ou `x/__init__.py`).
- Markdown : liens relatifs, références comprises. HTML : `src` et `href`.
- Partout : un chemin de l'arborescence, suivi ou non de `:ligne` ou `:ligne:colonne` (`src/lib/x.ts:42:7`), aussi en fin de phrase ou écrit avec des `\`.
- Seuls les fichiers de l'arborescence sont des cibles. Un import relatif vers un fichier absent donne « Fichier introuvable : <chemin> ». Un paquet (`react`, `@scope/x`) n'est pas navigable, ni un alias à motif qui ne trouve rien.

### Niveau B : les définitions

- Le mot sous la souris quand c'est un identifiant : hors commentaires, chaînes, nombres et mots-clés (mais dans les `${…}`), et hors fichiers de prose (`md`, `txt`…).
- D'abord dans le fichier, par son arbre syntaxique (JS/TS, Svelte, Vue, Rust, Python, Go) : la définition la plus proche avant le clic, dans une portée qui le contient.
- Puis, si c'est un import, dans le fichier importé (JS/TS, Python `from … import`, Rust `use crate::`, `self::`, `super::` et chemins Rust 2018), lu dans son onglet s'il est ouvert. Un import de paquet mène à la ligne de l'import ; un fichier qui ne définit pas le nom (import par défaut, `* as`) s'ouvre en haut.
- Sinon, une recherche dans les fichiers du même langage, par un motif de définition, vérifiée ligne par ligne et rangée : même fichier, même dossier, le reste.

### Le résultat

- Une cible : le fichier s'ouvre dans la même source, le curseur à la ligne et à la colonne, la ligne centrée.
- Plusieurs : la liste « Définitions de « X » » sous le mot (au-dessus s'il n'y a pas la place), 50 au plus, chacune avec « chemin:ligne » et la ligne. ↑ ↓ (en boucle), Entrée ou clic ; Échap ou un clic ailleurs la ferment.
- Aucune : « Aucune définition trouvée pour « X ». ».
- Une erreur : « Navigation impossible : <erreur> ».
- Seule la réponse du dernier clic compte. Une réponse arrivée après un changement de fichier, une modification du texte ou la fermeture de l'éditeur est oubliée.

### Historique

- Un historique par projet et par source, de 50 places, en mémoire. Chaque saut (navigation, résultat de recherche, `Ctrl+P`) note d'où il part.
- `Alt+←` / `Alt+→` (sous macOS `⌃-` / `⌃⇧-`, avec la touche Contrôle et non Cmd) et les boutons « précédent » / « suivant » de la souris, au-dessus de l'éditeur, reculent et avancent. Ces touches ne sont prises que dans l'éditeur, jamais dans un terminal ni dans le composer.
- Un nouveau saut vide la pile « suivant ». Une place dont le fichier a disparu est sautée.

## Limites

- `extends` du tsconfig n'est pas suivi ; un `tsconfig.json` modifié ne compte qu'à l'affichage suivant de la source.
- Pas de réexport (un `index.ts` qui rassemble), pas de membre d'un autre fichier (`obj.methode`), pas de `import x` Python vers sa définition.
- Les mots d'un fichier HTML, CSS, JSON ou YAML sont soulignés, mais mènent le plus souvent à « Aucune définition trouvée… ».
