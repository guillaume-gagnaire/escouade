# L’éditeur intégré

L’éditeur intégré sert à parcourir et à modifier les fichiers d’un projet ou du worktree d’un agent sans quitter Escouade, avec la coloration syntaxique, la recherche, la navigation dans le code et le suivi des changements.

![L’éditeur sur le worktree d’un agent : l’arborescence avec les fichiers modifiés, des onglets, le texte avec ses lignes modifiées en marge et la barre de recherche](/images/editor.jpg)

## Ouvrir l’éditeur

L’éditeur prend la place de la conversation dans la zone principale. Le bouton « ← Conversation » en haut à gauche y revient. Les onglets ouverts et les dossiers dépliés sont gardés, projet par projet, tant qu’Escouade tourne : en revenant à l’éditeur, tu retrouves ce que tu y avais laissé.

| Pour ouvrir… | Fais ceci |
|---|---|
| L’éditeur de l’agent affiché | « Ouvrir dans l’éditeur » dans le menu de l’agent (clic droit sur sa carte, ou « … » de l’en-tête de sa conversation). Il s’ouvre sur son worktree, ou sur le projet s’il n’en a pas. |
| L’éditeur du projet | Le bouton « Éditeur » en haut à droite de la barre de titre, ou « Parcourir » au pied de la barre latérale. |
| Un fichier modifié | Le bouton `</>` d’une ligne du [panneau des fichiers non commités](git), ou « Ouvrir dans l’éditeur » dans son menu. |
| Un fichier que l’agent a édité | Le nom du fichier sur une ligne d’appel d’outil (Edit, Write…) dans la conversation : l’éditeur s’ouvre à la première ligne que cette édition a modifiée. La liste des fichiers modifiés qui résume la fin d’un tour fait de même, sans ligne précise. |
| Un fichier connu par son nom | `Ctrl+P`, voir [plus bas](#ouvrir-un-fichier-par-son-nom). |

Sous macOS, `Ctrl` devient `⌘` dans les raccourcis de cette page, sauf pour revenir en arrière et en avant (voir [plus bas](#aller-a-la-definition)). Un fichier qui est en dehors du dossier du projet, ou de celui de l’agent, ne s’ouvre pas : un message le dit. Quand une source s’ouvre sans aucun onglet, l’éditeur affiche son premier fichier modifié, à défaut son `README.md`.

## La source : le projet ou le worktree d’un agent

Le bouton en haut à gauche de l’éditeur, à côté de « ← Conversation », dit ce que tu parcours : « ⎇ main » avec l’étiquette « branche » pour le dossier du projet, ou le nom d’un agent avec l’étiquette « worktree ». Il ouvre un menu « Parcourir » :

- le projet, avec sa branche et son chemin, et le nombre de ses fichiers modifiés (« Δ 3 ») ou « propre » ;
- chaque agent qui a un [worktree](worktrees), avec sa pastille de statut, son chemin, son modèle et le nombre de ses modifications (« 4 modif. ») ou « propre ».

Chaque source a ses propres onglets, ses dossiers dépliés et son arborescence. Les fichiers sont lus et écrits directement sur le disque : ce que tu enregistres dans le worktree d’un agent est ce qu’il verra en lisant ce fichier. La barre sous le texte rappelle la source (« worktree · refacto-auth » ou « branche · main »).

## L’arborescence

La colonne de gauche montre les fichiers de la source : ceux que git liste, suivis ou nouveaux et non ignorés. Les dossiers comme `node_modules`, que git ignore, n’y sont pas, ni les worktrees des agents quand ils sont rangés dans le dossier du projet. Elle va jusqu’à 50 000 fichiers ; au-delà, l’en-tête dit « liste tronquée ».

- L’en-tête donne le chemin de la source et un résumé (« 128 fichiers · 3 modif. »). Les boutons de la rangée « Fichiers » créent un fichier ou un dossier (« Nouveau fichier », « Nouveau dossier ») dans le dossier de la dernière ligne cliquée (à défaut, celui du fichier affiché, ou la racine), « Actualiser » relit l’arborescence et « Tout réduire » replie les dossiers. Elle se met à jour aussi toute seule peu après chaque changement dans le dossier, par exemple quand l’agent écrit un fichier.
- Un fichier modifié, ajouté ou supprimé prend la couleur de son état et sa lettre (`M`, `A`, `D`) ; un dossier qui en contient un porte un point de la même couleur. Les onglets reprennent la couleur du fichier.
- Les fichiers que git ignore mais que le projet copie dans ses worktrees (les `.env*`, par défaut) sont dans l’arborescence, en grisé, avec l’info-bulle « Ignoré par git » : voir [les worktrees](worktrees).
- La poignée entre la colonne et le texte règle la largeur de la colonne, au clavier comme à la souris ; un double-clic la remet à sa largeur d’origine. La largeur vaut pour tous les projets.
- Le clavier suit les règles des arbres : `↑` et `↓`, `Home` et `End` se déplacent, `→` déplie un dossier ou entre dedans, `←` le replie ou remonte au dossier parent, `Entrée` ouvre un fichier, `F2` renomme et `Suppr` supprime (`⌘⌫` sous macOS).

### Le menu de l’arborescence

Un clic droit sur une ligne, ou sur l’espace libre sous les lignes, ouvre un menu :

| Entrée | Ce qu’elle fait |
|---|---|
| « Nouveau fichier… » | Ouvre un champ de nom dans le dossier de la ligne (la racine sur l’espace libre). Un nom comme `src/lib/a.ts` crée aussi les dossiers qui manquent. |
| « Nouveau dossier… » | Même chose pour un dossier. Git ne liste pas un dossier vide : l’arborescence le montre quand même. |
| « Ouvrir un terminal ici » | Ouvre un [terminal](terminals) dans ce dossier de la source. |
| « Afficher dans l’Explorateur » (Windows), « Afficher dans le Finder » (macOS), « Ouvrir le dossier contenant » (Linux) | Ouvre le gestionnaire de fichiers du système sur le fichier ou le dossier, sélectionné. Sur l’espace libre, ouvre le dossier de la source : celui du projet, ou le worktree de l’agent (« Ouvrir le dossier » sous Linux). Si le worktree de l’agent n’existe plus, un message le dit. |
| « Renommer… » (`F2`) | Remplace la ligne par un champ. Un nom comme `autre/a.ts` déplace le fichier. Les onglets ouverts suivent. |
| « Supprimer » (`Suppr`, `⌘⌫` sous macOS) | Envoie le fichier ou le dossier à la corbeille après confirmation (« Il part dans la corbeille. » ; pour un dossier, « Le dossier et ses 12 fichiers partent dans la corbeille. »). |
| « Copier le chemin » et « Copier le chemin relatif » | Copient le chemin complet, ou le chemin à partir de la racine de la source. |

Le champ de nom refuse ce qui ne peut pas exister et le dit sous la ligne : un nom qui commence par une barre oblique, un nom invalide, un nom déjà pris (« a.ts » existe déjà à cet endroit.) ou un dossier que tu essaies de mettre dans lui-même. Si le fichier que tu crées est ignoré par git, un message le dit (« .env.local est ignoré par git : l’arborescence ne le montre pas. »).

Supprimer un dossier ou un fichier dont des fichiers sont ouverts avec des modifications non enregistrées demande d’abord, pour chacun, s’il faut l’enregistrer. Renommer ne perd rien : les onglets ouverts, modifications comprises, suivent le nouveau nom.

## Onglets et enregistrement

Un clic sur un fichier l’ouvre dans un onglet. Le bouton × d’un onglet le ferme, comme un clic du milieu ; un point remplace la croix quand le fichier a des modifications non enregistrées. Deux onglets de même nom portent le nom de leur dossier pour qu’on les distingue.

- En haut à droite, « ● Non enregistré · Ctrl+S » ou « Enregistré » dit où en est le fichier affiché, et le bouton « Enregistrer » l’enregistre. `Ctrl+S` fait de même.
- L’enregistrement garde les fins de ligne du fichier (LF ou CRLF) et son BOM, et l’écriture est atomique : un fichier n’est jamais écrit à moitié. Un fichier en lecture seule est refusé.
- Fermer un onglet qui a des modifications demande « Enregistrer « a.ts » ? » avec « Enregistrer », « Ne pas enregistrer » et « Annuler » ; fermer la fenêtre d’Escouade avec des fichiers non enregistrés demande « Quitter Escouade ? » (« Quitter quand même »).
- Un fichier binaire n’est pas affiché (« Fichier binaire : pas d’aperçu. »), pas plus qu’un fichier de plus de 2 Mo (« Fichier trop volumineux pour l’éditeur (2,4 Mo). »).
- La barre sous le texte donne la ligne et la colonne du curseur (« Ln 9, Col 22 »), le langage, l’encodage (UTF-8), les fins de ligne et l’indentation du fichier (« Espaces : 2 » ou « Tabulations »), que l’éditeur déduit du fichier.

## Le texte

La coloration syntaxique suit l’extension du fichier. Un fichier d’un autre type s’affiche en « Texte ».

| Langage | Extensions |
|---|---|
| TypeScript, JavaScript | `.ts`, `.mts`, `.cts`, `.tsx`, `.js`, `.mjs`, `.cjs`, `.jsx` |
| Web | `.html`, `.css`, `.scss`, `.svelte`, `.vue` |
| Données et configuration | `.json`, `.yaml`, `.yml`, `.toml`, `.xml`, `.svg`, `.sql` |
| Documents | `.md` |
| Langages compilés | `.rs`, `.go`, `.java`, `.c`, `.h`, `.cpp`, `.hpp`, `.cc`, `.cs`, `.kt`, `.swift` |
| Scripts | `.py`, `.rb`, `.php`, `.sh`, `.bash`, `.ps1`, `Dockerfile` |

Les fichiers `.prisma` et `.tf` sont reconnus, mais sans coloration.

L’éditeur a ce qu’on attend d’un éditeur de code : numéros de ligne, ligne courante marquée, parenthèses et crochets repérés et fermés tout seuls, indentation qui suit ce que tu tapes, `Tab` pour indenter et `Ctrl+Z` pour annuler.

- `Alt+clic` (`⌥+clic` sous macOS) ajoute un curseur.
- `Ctrl+F` ouvre la barre de recherche dans le fichier, en haut du texte : « suivant », « précédent », « tout », et les options « respecter la casse », « mot entier » et « expression régulière », avec le champ « Remplacer », « remplacer » et « tout remplacer ». `Échap` la ferme.

### Les lignes modifiées

Dans la marge, à côté des numéros, une barre colorée marque les lignes modifiées depuis la version de référence du fichier, et un repère marque l’endroit d’où des lignes ont disparu. La version de référence est celle du dernier commit. Pour le worktree d’un agent, c’est celle où sa branche a quitté sa branche de base : les commits de l’agent comptent alors comme des changements.

Au-dessus du texte, le chemin du fichier est suivi d’un résumé : « 5 lignes modifiées vs main », « Nouveau fichier · absent de main » ou « Identique à main ». Dans le dossier du projet, la référence s’écrit « HEAD ».

Le bouton « Voir les changements » (visible quand le fichier a une version de référence) affiche ces changements dans le texte : au-dessus de chaque bloc modifié, les lignes de la version de référence, avec un bouton « Annuler ce bloc » qui les remet dans le texte. Ça modifie le texte sans l’enregistrer : `Ctrl+Z` défait, et rien n’est écrit tant que tu n’as pas enregistré. Un second clic sur le bouton referme la comparaison.

## Quand un fichier change sur le disque

Tant que l’éditeur est ouvert, il relit les fichiers ouverts à chaque changement du dossier, par exemple quand l’agent travaille.

- Un fichier que tu n’as pas modifié prend la nouvelle version tout seul, sans toucher à la position du curseur.
- Un fichier que tu as modifié ne change pas sous tes doigts : un bandeau apparaît, « Ce fichier a changé sur le disque. », avec trois boutons.

| Bouton | Effet |
|---|---|
| « Recharger » | Abandonne tes modifications et prend la version du disque. |
| « Comparer » | Affiche dans le texte ce que le disque a de différent : au-dessus de chaque bloc, les lignes du disque, avec « Prendre ce bloc ». Le bandeau dit « Comparaison avec la version du disque. » |
| « Garder ma version » | Enregistre ta version par-dessus celle du disque. |

Si le fichier change encore pendant que tu le compares, le bandeau le dit (« Le fichier a encore changé sur le disque : la comparaison montre sa nouvelle version. ») et « Garder ma version » attend une seconde : un clic visé sur la version précédente n’écrase pas la nouvelle. Un enregistrement est refusé tant que le fichier a changé sur le disque depuis que tu l’as lu : tu n’écrases jamais le travail de l’agent sans l’avoir vu.

Si le fichier est supprimé, le bandeau dit « Ce fichier a été supprimé. », avec « Fermer » et « Le recréer en enregistrant ».

## Aller à la définition

Avec `Ctrl` enfoncé (`⌘` sous macOS), ce qui mène ailleurs est souligné sous la souris, avec une main, et un clic le suit. `F12` suit ce qui est sous le curseur du texte. Il n’y a pas de serveur de langage : Escouade se sert de l’arborescence de la source, de la syntaxe du fichier et d’une recherche dans les fichiers.

| Sous le curseur | Où ça mène |
|---|---|
| Un import de script (`from '…'`, `import '…'`, `import('…')`, `require('…')`) dans un fichier JavaScript, TypeScript, Svelte ou Vue | Le fichier importé. Les extensions, les dossiers `index` et le source TypeScript d’un `.js` sont trouvés, et les alias de `tsconfig.json` ou `jsconfig.json` (`paths`, `baseUrl`) et le `$lib` de SvelteKit sont suivis. Un paquet (`node_modules`) ne mène nulle part. |
| `@import`, `@use`, `@forward` ou `url(…)` dans une feuille de style, une page HTML, Svelte ou Vue | Le fichier (les partiels Sass `_x.scss` sont trouvés). |
| `mod x;` dans un fichier Rust | `x.rs` ou `x/mod.rs`. |
| `from a.b import c` ou `import a.b` dans un fichier Python | Le fichier du module, imports relatifs compris. |
| Un lien relatif, `src` ou `href` dans un fichier Markdown ou HTML | Le fichier. |
| Un chemin écrit n’importe où, qui existe dans la source (`src/app.ts`, `./a.ts`), éventuellement suivi de `:42` ou `:42:7` | Le fichier, à cette ligne et cette colonne. |
| Un nom (variable, fonction, classe, type…) | Sa définition : dans le fichier, par sa syntaxe, au plus près du curseur ; dans le fichier d’où il est importé ; sinon dans les fichiers du même langage, où une recherche trouve ce qui le déclare. Rien dans un commentaire, une chaîne ou un mot-clé, ni dans un fichier Markdown ou texte. |

Une seule destination est ouverte tout de suite. S’il y en a plusieurs, une liste « Définitions de « nom » » s’ouvre sous le mot : `↑` et `↓` choisissent, `Entrée` ouvre, `Échap` ferme. Sans résultat, un message dit « Aucune définition trouvée pour « nom ». » ; si le fichier visé n’est pas dans la source, « Fichier introuvable : … ».

`Maj+F12` sur un nom cherche où il est utilisé : la recherche dans les fichiers s’ouvre sur ce nom, mot entier, casse respectée. Le texte est cherché tel quel, commentaires compris.

### Revenir en arrière

Chaque saut (un clic, `F12`, un résultat de recherche, un fichier ouvert par `Ctrl+P`) garde l’endroit d’où tu es parti. `Alt+←` y revient et `Alt+→` repart en avant, comme dans un navigateur ; les boutons « précédent » et « suivant » de la souris font de même. Sous macOS, ce sont `⌃-` et `⌃⇧-`, avec la touche Contrôle et non `⌘`. L’historique garde 50 endroits par projet et par source.

## Ouvrir un fichier par son nom

`Ctrl+P` ouvre la palette « Ouvrir un fichier » depuis n’importe où dans un projet, éditeur ouvert ou non. Elle cherche dans la source de l’éditeur s’il est ouvert, sinon dans le worktree de l’agent affiché, sinon dans le projet ; le nom de la source est à droite du champ.

- Le champ dit « Nom du fichier, ou nom:42 pour une ligne ». Tape une partie du nom, ou seulement quelques lettres dans l’ordre : le début du nom compte le plus, puis un morceau de nom, puis le chemin.
- `nom:42` ouvre le fichier à la ligne 42, et `nom:42:7` à la ligne 42, colonne 7, comme les erreurs d’une trace (`src/app.ts:42:7`).
- Avec un champ vide, la liste montre d’abord les derniers fichiers ouverts dans cette source (30 au plus), sans celui qui est affiché : `Entrée` revient donc au fichier précédent.
- La palette montre 50 résultats au plus (« Les 50 premiers résultats : précise ta recherche. »). Un fichier ignoré par git porte l’étiquette « ignoré par git ». `↑` et `↓` choisissent, `Entrée` ouvre, `Échap` ferme.

## Rechercher dans les fichiers

`Ctrl+Maj+F` ouvre la recherche dans tous les fichiers de la source, dans la colonne de gauche ; la loupe au-dessus de l’arborescence fait de même, et le bouton voisin revient aux fichiers. Si un texte d’une seule ligne est sélectionné dans le code, c’est lui qui est cherché. `Ctrl+F` reste la recherche dans le fichier affiché.

- Le champ « Rechercher » lance la recherche 250 ms après la dernière frappe ; `Entrée` la lance tout de suite. Trois boutons règlent la recherche : « Respecter la casse » (`Aa`), « Mot entier » (`ab`) et « Expression régulière » (`.*`). Une expression invalide est signalée (« Expression régulière invalide. »).
- Escouade cherche avec `git grep` dans les fichiers que git liste (suivis, ou nouveaux et non ignorés), sans les fichiers binaires, et sans les worktrees des agents quand on cherche dans le projet. Le résumé dit « 12 résultats dans 3 fichiers ».
- Les résultats sont groupés par fichier, avec le nombre de lignes trouvées, et la partie qui correspond est surlignée. Un clic sur une ligne ouvre le fichier à cette ligne et cette colonne. `↑` et `↓`, `Home` et `End` parcourent la liste, `→` et `←` déplient et replient un fichier.
- Les résultats s’arrêtent aux 2 000 premières lignes (« Résultats limités aux 2 000 premiers. ») ou après 10 secondes (« Recherche arrêtée après 10 s : résultats partiels. »).
- La recherche est gardée quand tu fermes l’éditeur, par projet ; en changeant de source, elle repart sur la nouvelle.

## Les extensions

Le bouton « Extensions », à droite de la barre de l’éditeur, ouvre le panneau des extensions d’[Open VSX](https://open-vsx.org), le registre ouvert de la fondation Eclipse (celui de Microsoft est réservé à ses produits). Cherche une extension, ouvre sa fiche, puis « Installer » ; « Rechercher des mises à jour » propose les nouvelles versions de ce qui est installé, sans rien installer seul ; « Couper » et « Retirer » font ce qu’ils disent.

- **Escouade fait tout** : la recherche, le téléchargement, la vérification de la somme SHA-256 et de la signature qu’Open VSX pose sur chaque version, puis le déballage, où une archive piégée (un chemin qui sort de son dossier, un lien, une bombe de décompression) est refusée entière. Une version qu’Open VSX n’a pas signée ne s’installe pas. La page de l’éditeur ne parle jamais à Open VSX elle-même.
- **Ce qui marche aujourd’hui** : les extensions déclaratives, sans code : thèmes de couleurs, grammaires (la coloration d’un langage), extraits, configurations de langage, thèmes d’icônes. Une extension avec du code s’installe mais reste « En attente de l’hôte », et une extension qui demande Node le dit (« Demande l’hôte Node, pas encore dans Escouade »).
- **La confiance** : la première extension d’un éditeur demande « Faites-vous confiance à l’éditeur … ? », comme VS Code. La fiche dit si Open VSX a vérifié son espace de noms (« Vérifié ») : la signature prouve que le fichier est celui qu’Open VSX a publié, pas que l’extension est sûre.
- **Le thème** : l’éditeur garde le thème d’Escouade. « Utiliser « … » » prend celui d’une extension, et « Revenir au thème d’Escouade » le quitte.
- **Une fiche vient d’Open VSX** : son icône est téléchargée par Escouade, son README est affiché comme un message, une image du web ne se chargeant qu’au clic sur « Afficher l’image ».
- **Un projet privé** n’envoie rien à Open VSX : depuis l’éditeur d’un projet privé, pas de recherche ni de mise à jour ; les extensions installées marchent.

### Les extensions permises

Une liste, comme `extensions.allowed` de VS Code, dit ce qui peut s’installer : `"*"`, un éditeur (`dracula-theme`) ou une extension (`dracula-theme.theme-dracula`), chacun `true`, `false`, `stable` (pas de préversion) ou une liste de versions. La clé la plus précise dit ; une liste qui nomme quelque chose ne permet que ce qu’elle nomme. Elle se règle à quatre endroits, et une extension doit être permise par chacun : Réglages › Général › Avancé › « Extensions permises de l’éditeur », le [`.escouade.yml`](escouade-yml#editor) du projet (`editor.extensions_allowed`), l’équipe, et les [stratégies du poste](managed-policies) (`extensions.allowed`). Une extension refusée n’est même pas téléchargée, et sa fiche dit qui la refuse.
