# Les dossiers d'agents — conception

Date : 2026-11-06. Sans ticket (contribution du fork Metalzoid, branche `escouade/gestion-dossiers-agents`). Fait foi
pour les noms et les formes.

## But

Dans un projet, la barre latérale liste les agents à plat, du plus ancien au plus récent (ceux qui attendent une
réponse en tête, `sidebarOrder` de `src/lib/sidebar.ts`). Sur un projet comme Nexus, une grosse feature tient un agent
orchestrateur et une dizaine d'agents qu'il a créés lui-même par le serveur MCP (`create_agent`) ; entre eux
s'intercalent les agents d'autres features. On ne voit plus qui travaille sur quoi.

Désormais, les agents d'un projet se rangent dans des **dossiers d'agents** : nommés, repliables, avec au plus un
**agent principal** (l'orchestrateur). Un dossier se crée à la main, ou **tout seul** quand un agent en crée d'autres
par le MCP. On glisse un agent dans un dossier ou hors de lui, et on réordonne librement dossiers et agents. Tout ce
que l'utilisateur fait d'un dossier, un agent le fait aussi par le MCP.

Réussi quand : sur un projet à deux features menées par des orchestrateurs, chacune tient dans son dossier, replié
ou non, sans qu'un agent qui attend une réponse ne passe inaperçu, et sans qu'un clic sur une carte ne la déplace.

## Décisions de l'utilisateur (2026-11-06, ne pas les rouvrir)

1. **Des dossiers d'agents par projet**, créés à la main ou automatiquement. Un seul niveau : pas de dossier dans un
   dossier.
2. **Création automatique** : quand un agent crée un agent par le MCP (`create_agent` ; `start_ticket` quand c'est un
   agent qui lance le ticket). L'agent créé va dans le dossier de son créateur ; si le créateur n'en a pas, un dossier
   est créé, dont **le créateur devient l'agent principal**. Les agents seulement relancés (`send_message`…) ne
   bougent pas.
3. **Nom du dossier** : celui que l'orchestrateur lui donne par le MCP, à défaut un nom court tiré de sa feature (comme
   les agents). Il se renomme par double-clic, comme un agent.
4. **Présentation « boîte »** (maquette A) : un en-tête — chevron, nom, nombre d'agents, pastilles du nombre d'agents
   en cours (vert) et en attente d'une réponse (ambre) — puis les cartes complètes des agents, décalées sous un filet.
   L'agent principal est en tête du dossier, **marqué d'une étoile** `★` à gauche de son nom, dans la couleur du projet,
   infobulle « Agent principal ». Les cartes gardent tout ce qu'elles ont aujourd'hui (réponse depuis la liste,
   sous-agents, PR…).
5. **Glisser-déposer** : un agent lâché sur un dossier y entre ; tiré hors de lui, il en sort. Dossiers et agents hors
   dossier se réordonnent dans la même liste, les agents aussi dans un dossier. **Un clic n'est jamais un
   déplacement** : il faut un minimum de mouvement, bouton maintenu, pour que le glisser commence.
   Réalisé par le geste au pointeur de l'écran partagé (`paneDrag`), relâché dans la liste ou sur la zone de travail
   (décision de l'utilisateur du 2026-11-09) : plus de glisser-déposer HTML5 dans la barre latérale.
6. **Un dossier replié ne cache jamais un agent qui attend une réponse.**
7. **MCP : parité** avec ce que fait l'utilisateur (sauf replier, qui est de l'affichage). Outils ci-dessous.
8. **Pas de rangement rétroactif** : les agents existants restent hors dossier ; les dossiers servent à partir de la
   version qui les apporte.
9. **Qui attend une réponse remonte, à son niveau** : un agent hors dossier remonte en tête de la liste du projet,
   comme aujourd'hui ; un agent d'un dossier remonte en haut de son dossier, juste sous l'agent principal. Les
   dossiers eux-mêmes ne bougent pas.

## Comportement

### Dans la barre latérale

- La liste d'un projet est une suite d'**éléments** : des dossiers et des agents hors dossier, dans l'ordre que
  l'utilisateur (ou un agent) a choisi — l'**ordre rangé**. Dans un dossier : l'agent principal d'abord, puis ses
  agents dans leur ordre.
- L'**ordre affiché** est l'ordre rangé, avec la remontée de décision 9 : hors dossier, les agents qui attendent une
  réponse d'abord (entre eux, dans leur ordre rangé), puis les éléments dans l'ordre rangé ; dans un dossier, l'agent
  principal, puis ceux qui attendent, puis les autres. Une fois répondu, l'agent reprend sa place rangée (`sidebarOrder`
  de `src/lib/sidebar.ts` devient cette fonction). « L'ordre de la barre », partout où la spec ou le MCP le nomment,
  est l'ordre affiché.
- Glisser ou déplacer au clavier change l'ordre rangé : l'élément est posé relativement à ses voisins non remontés
  à l'endroit du dépôt. Un agent qui attend, déplacé, reste affiché en tête de son niveau jusqu'à sa réponse, puis
  prend la place où on l'a posé.
- **Nouvel agent hors dossier** : en bas de la liste (comme aujourd'hui, le plus récent en dernier). **Nouvel agent
  dans un dossier** : en bas du dossier. **Dossier créé automatiquement** : à la place qu'avait son agent principal.
  Dossier créé à la main : en bas de la liste, ou à la place de l'agent depuis lequel on l'a créé.
- **Replier** : un clic sur le chevron (ou sur l'en-tête hors du nom). L'état replié est gardé par dossier, d'un
  lancement à l'autre (comme les autres choix d'affichage, `UiState`). Replié, le dossier montre encore, sous son
  en-tête, la carte de chacun de ses agents **qui attend une réponse** (décision 6) et celle de **l'agent ouvert** ;
  elles rentrent quand ce n'est plus le cas.
- **En-tête** : nom (double-clic : renommer), nombre d'agents non archivés, pastilles « en cours » et « en attente »
  (absentes à zéro), lues aussi par les lecteurs d'écran (« 2 en cours, 1 en attente »).
- **Dossier vide** : il reste, avec « Glisse un agent ici », jusqu'à ce qu'on le supprime.
- **Agents archivés** : la section « Archivés » ne change pas (plate). Un agent archivé quitte la liste de son
  dossier mais garde son dossier : restauré, il y revient (en bas), si le dossier existe encore ; sinon hors dossier,
  en bas. Il redevient agent principal s'il l'était et que le dossier n'en a pas d'autre.

### Créer, renommer, supprimer, désigner l'agent principal

- **Créer à la main** : dans le menu à côté de « Nouvel agent », « Nouveau dossier » ; au clic droit d'un agent,
  « Nouveau dossier avec cet agent » (l'agent y entre, sans devenir principal). Le dossier naît en mode renommage,
  sous le nom « Dossier », « Dossier 2 »…
- **Menu d'un dossier** (clic droit sur son en-tête) : Renommer, Monter, Descendre, Supprimer.
- **Supprimer un dossier** : ne supprime ni n'archive aucun agent ; ils reviennent hors dossier, à la place du
  dossier, dans leur ordre. Pas de confirmation (rien ne se perd).
- **Agent principal** : au clic droit d'un agent d'un dossier, « Faire l'agent principal » / « Retirer l'étoile ».
  Au plus un par dossier : en désigner un retire l'étoile au précédent, qui reste dans le dossier.
- **Menu d'un agent**, en plus : « Déplacer vers » ▸ chaque dossier du projet, « Nouveau dossier… », « Hors dossier »
  (le chemin sans souris de décision 5), et Monter / Descendre, comme les tuiles de projet.

### Glisser-déposer et clavier

- Même mécanique que les tuiles de la barre des projets (`ProjectRail.svelte`, `dropProject` de
  `src/lib/project-rail.ts`) : un trait marque où l'élément tombera ; un agent survolant l'en-tête d'un dossier
  surligne le dossier (il y entrera en bas). Un dossier se glisse entier ; un dossier ne se lâche pas dans un dossier.
- Le glisser ne commence qu'au-delà d'un seuil de mouvement (≈ 5 px). En dessous : un clic, qui ouvre l'agent.
- Clavier : `Alt+↑` / `Alt+↓` sur une carte ou un en-tête le déplace d'un cran, en entrant dans un dossier ou en
  en sortant aux bords ; le focus suit l'élément.

### Ailleurs dans l'app

- `Ctrl+Tab` / `Ctrl+Maj+Tab` (agent suivant / précédent) et les listes « dans l'ordre de la barre latérale » (menu
  `+` des terminaux, « Dans le worktree de… ») suivent l'ordre de la barre, dossiers compris, repliés ou non.
- La vue d'ensemble, le Kanban, les noms des sessions du remote control (« projet · agent ») ne changent pas.
- **Dupliquer la conversation** : la copie va dans le dossier de l'original, juste après lui, jamais principale.
- **Projet fermé** : ses dossiers sont gardés avec son historique et reviennent à sa réouverture.
- **Agent supprimé** : il quitte son dossier ; s'il était principal, le dossier n'a plus d'agent principal.

### Création automatique (MCP)

- Quand `create_agent` (ou `start_ticket`, appelé par un agent) crée un agent **dans le projet de l'appelant** :
  - avec `folder`, il va dans ce dossier du projet (par son id ou son nom, sans tenir compte de la casse) ; un nom
    qu'aucun dossier ne porte crée le dossier sous ce nom, l'appelant y entrant comme agent principal s'il n'est dans
    aucun dossier ;
  - sans `folder` : dans le dossier de l'appelant ; si l'appelant n'en a pas, un dossier est créé à sa place, nommé
    d'après lui (ci-dessous), l'appelant en agent principal, le nouvel agent dedans.
- Dans un **autre projet** que celui de l'appelant, ou pour un **client extérieur** (`Caller::External`), ou pour un
  agent que crée le pilote auto du Kanban : pas de dossier automatique ; `folder` reste possible (dossier existant ou
  nouveau, sans agent principal).
- `folder: ""` (vide) : l'agent créé reste hors dossier, sans dossier automatique.
- **Tickets** : `start_ticket` ne crée pas l'agent tout de suite (il met le ticket en file, le passage du Kanban le
  crée ensuite). Le dossier visé est donc résolu à l'appel (mêmes règles, `folder` compris) et **retenu sur le
  ticket** ; l'agent du ticket y entre à son lancement, et les agents que le ticket relance plus tard
  (`resume_ticket`, `reject_ticket`, renvoi en « À faire » puis relance) aussi, tant que le dossier existe. Un ticket
  lancé par l'utilisateur ou le pilote auto ne retient aucun dossier.
- **Nom automatique** : le dossier créé automatiquement sans `folder` porte le nom de son agent principal (déjà un nom
  court de sa feature, donné par le modèle des textes rédigés) et le **suit** — nom encore à venir, renommage de
  l'agent — tant que le dossier n'a pas été renommé (par l'utilisateur ou par `update_folder`).

## Serveur MCP

Mêmes règles d'accès qu'aujourd'hui (`may_act_on` : son projet et les projets non privés), un agent n'est pas limité
à son propre dossier. Un agent d'Escouade peut omettre `project` pour le sien. Un dossier se nomme par son id ou son
nom (casse ignorée), un agent comme aujourd'hui.

- `list_agents` : chaque agent gagne `folder` (`{ id, name }`, `null` hors dossier) et `principal` (booléen). Les
  agents viennent dans l'ordre de la barre latérale, projet par projet dans l'ordre de l'app (au lieu du plus ancien
  d'abord : la description de l'outil le dit).
- `list_folders` (lecture) : les dossiers d'un projet dans l'ordre de la liste : `id`, `name`, `principal`
  (`{ id, name }` ou `null`), `agents` (`[{ id, name, status }]` dans leur ordre, archivés exclus).
- `create_folder` : `project`, `name`, `agents` (facultatif, dans cet ordre), `principal` (facultatif, l'un d'eux).
  Répond le dossier comme `list_folders`.
- `update_folder` : `folder`, et au choix `name`, `principal` (un agent du dossier, ou `""` pour aucun), `position`
  (rang du dossier dans la liste du projet, à partir de 0).
- `delete_folder` : `folder` ; ses agents reviennent hors dossier.
- `move_agent` : `agent`, `folder` (un dossier, ou `""` pour hors dossier), `position` (facultative : son rang dans le
  dossier, ou dans la liste du projet hors dossier ; en bas par défaut).
- `create_agent` et `start_ticket` : nouveau paramètre facultatif `folder` (section précédente). Leur réponse dit où
  l'agent est (ou sera) rangé (`folder`).
- Les deux listes de noms d'outils de `src-tauri/src/mcp/tests.rs` et la page *Serveur MCP* de la spec les prennent.

## Contraintes

- **La spec du projet vit dans Escouade** : mettre à jour par `write_spec_page`, avec le code, au moins
  `fenetre/barre-laterale` (section « Dossiers d'agents ») et `harness/serveur-mcp` ; jamais `docs/SPEC.md`.
- Deux langues : tout texte visible et toute réponse du serveur en français et en anglais (`t()` côté fenêtre,
  `tr!` côté Rust), comme le reste.
- L'état est la vérité du cœur Rust (`PersistedState`), sauvegardé comme les projets et les agents ; la fenêtre ne
  fait qu'afficher et demander. Un état d'avant cette version se lit sans dossier (`serde(default)`).
- Une opération qui vise un agent ou un dossier disparu entre-temps est refusée avec sa raison, jamais à moitié faite.
- Pas de dépendance nouvelle (le glisser-déposer natif suffit, comme la barre des projets).
- Tests :
  - Rust (`cargo nextest`, profil `ci`) : les opérations de dossiers du cœur (ordre, archivage/restauration,
    suppression d'agent, copie, projet fermé puis rouvert, état ancien) et chaque outil MCP, dont la création
    automatique et ses exceptions ;
  - Vitest : l'ordre affiché (repliés, en attente, agent ouvert), les menus ;
  - e2e Playwright (Windows, en CI) : un fichier `tests/e2e/agent-folders.spec.ts` qui crée un dossier à la main,
    glisse un agent dedans et hors de lui, vérifie qu'un clic ne déplace pas, replie un dossier avec un agent en
    attente, et fait créer des agents par le MCP (comme `mcp.spec.ts`) pour voir le dossier automatique ;
  - sur le Mac de l'utilisateur, une vérification dans la vraie app (mémoire Escouade `captures-app-sur-mac`).
- Avant de changer un menu, chercher ses libellés dans `tests/e2e/` aussi (mémoire `grep-e2e-avant-de-changer-un-menu`).

## Hors périmètre

- Dossiers imbriqués, couleurs ou icônes de dossier, dossiers partagés entre projets ou avec l'équipe.
- Rangement rétroactif des agents existants (décision 8).
- Archiver ou supprimer un dossier avec ses agents d'un coup.
- La vue d'ensemble et le Kanban.

## Anti-buts (ce qui serait un échec même si ça marche)

- Un agent qui attend une réponse invisible parce que son dossier est replié.
- Un clic sur une carte qui la déplace.
- Un orchestrateur qui doit penser aux dossiers pour qu'ils existent : sans un mot de lui, ses agents sont rangés.

## Laissé au constructeur

- La forme du stockage (dossiers sur `Project`, ordre des éléments, appartenance sur `AgentMeta`…), tant que l'ordre
  est un et que fermer/rouvrir un projet garde ses dossiers.
- Le détail visuel au-delà de la maquette A (`.superpowers/brainstorm/…/dossiers-layout-v2.html`, non commitée) et de
  `DESIGN.md` : espacements, survol, trait de dépôt, barre étroite (sous 250 px).
- Les événements vers la fenêtre, les noms internes, les libellés anglais.

## Mes choix (à vérifier en relisant)

- Le nom automatique d'un dossier est **le nom de son agent principal**, qu'il suit jusqu'à un renommage, sans appel de
  plus au modèle.
- Les agents d'un ticket lancé par un orchestrateur, et ceux que ce ticket relance, vont dans le dossier retenu sur le
  ticket ; `start_ticket` prend `folder`.
- Un agent qui attend, déplacé, reste en tête de son niveau jusqu'à sa réponse.
- Replié, un dossier montre aussi la carte de **l'agent ouvert**, pour que `Ctrl+Tab` n'atterrisse jamais sur une
  carte cachée.
- Un dossier vide reste jusqu'à ce qu'on le supprime.
- `list_agents` passe à l'ordre de la barre latérale.
- La copie d'un agent suit l'original dans son dossier.
