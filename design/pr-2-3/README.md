# ESC-449 — les PR dans Escouade (2.3), trois directions

Demande de l'utilisateur (2026-11-14) : « je veux pouvoir voir la PR dans Escouade, laisser un agent relecteur me faire ses commentaires comme le ferait un humain, à côté du code, et avoir un meilleur contrôle sur la PR » (`docs/TODO-2.3.md`, chapitre « Les PR dans Escouade »).

Rien du produit ne change dans ce ticket : ce sont des maquettes statiques pour choisir une direction.

- `index.html` : la page de comparaison (l'existant, les trois directions côte à côte, ce que chacune favorise et coûte, la recommandation). `?theme=light` pour la lire en clair.
- `maquette.html` : les trois directions. `?d=a|b|c`, `&theme=light`, puis `#fenetre` (la PR et la relecture de l'agent), `#controle` (le contrôle de la PR) ou `#etats` (relecture en cours, CI rouge, conflit, prête à fusionner). Jetons de `src/app.css`, polices de l'app, icônes de `src/lib/icons.ts` (copiées dans `icons.js`) ; données fictives : la PR #214 « Connexion par lien magique » du ticket ATL-42, ouverte par l'agent « Lien magique » et relue par l'agent « Relecteur ».
- `captures/` : `node design/pr-2-3/render.mjs` (18 PNG : 3 directions × 3 vues × sombre et clair `-clair`).
- `existant/` : la vraie app (build de debug), sur le faux GitHub et le faux claude des e2e :
  `ESCOUADE_E2E_ROOT='C:\escouade-449' ESCOUADE_E2E_SHOTS=$PWD/design/pr-2-3/existant npx playwright test tests/e2e/prs-view.spec.ts tests/e2e/prs-review.spec.ts tests/e2e/prs-tab.spec.ts tests/e2e/kanban-pr.spec.ts tests/e2e/prs-overview.spec.ts`
  (il faut `npx tauri build --debug --no-bundle` et `sh scripts/plugins-wasm.sh` d'abord ; lancer les spec une par une si le dossier `webview` de la racine fixe reste verrouillé entre deux tests).

## L'existant, relu

| Où | Ce qu'on y fait | Capture |
|---|---|---|
| Vue « Pull requests » du projet (ESC-65) | Les PR ouvertes en lignes denses, filtres, « Faire relire », « Lancer un agent ». Cliquer une PR ouvre le navigateur. | `existant/prs-view.png`, `prs-view-review.png` |
| Relecture par un agent (ESC-66) | Un brouillon : un texte et des commentaires, chacun à garder, corriger ou écarter, puis le verdict et « Publier sur GitHub ». | `existant/prs-review.png`, `prs-review-decided.png` |
| Onglet « PR et CI » du panneau latéral (ESC-305) | L'en-tête de la PR de l'agent, ses contrôles et leur journal, « Relancer les échecs », ses relectures et commentaires, ses déploiements. | `existant/pr-tab-failed.png`, `pr-tab-project.png` |
| Colonne « En revue » du Kanban (ESC-59 à 63) | La PR d'un ticket : état, CI, approbations, conflit ; « L'agent corrige », « L'agent corrige les retours », « L'agent met à jour la branche », « Merger », « Merger quand c'est vert ». | `existant/kanban-pr-*.png` |
| Vue d'ensemble (ESC-68) | Les PR qui attendent l'utilisateur, sur tous les projets. | `existant/esc-68-overview.png` |

Où ça frotte :

1. **La PR ne se voit jamais en entier.** Ni sa description, ni son diff, ni ses commits : chaque geste « ouvrir la PR » ouvre le navigateur. Ce qu'on sait d'elle est réparti entre quatre endroits (la vue, l'onglet, la carte, la relecture), chacun avec un morceau.
2. **Les relectures se lisent sans y répondre.** L'onglet « PR et CI » le dit : « En lecture : Escouade ne publie rien en ton nom. » Un commentaire y est une ligne `fichier:ligne` et son texte : ni le code qu'il vise, ni ses réponses, ni « répondre » ou « résoudre ». Les plugins des hébergeurs savent pourtant commenter, répondre et résoudre.
3. **La relecture de l'agent est un tri, pas une conversation.** Garder, corriger ou écarter chaque commentaire, puis tout publier : pas de fil, pas de réponse de l'agent, pas de suggestion de code à appliquer. On ne lit que l'extrait de diff de chaque commentaire, jamais le fichier. Pour qu'un commentaire soit corrigé, il faut le publier, puis attendre qu'il revienne comme « retour » à l'agent du ticket (ESC-61) ; une PR sans ticket n'a pas ce chemin.
4. **Le contrôle est partiel et dispersé.** Merger n'existe que sur la carte d'un ticket ; approuver ou demander des changements, seulement en publiant la relecture d'un agent. Ni brouillon ↔ prête, ni relecteurs ou étiquettes après l'ouverture, ni choix de la méthode de merge au moment de merger. La mise à jour de la branche et la résolution d'un conflit passent toujours par un agent, et la relance de la CI relance tous les échecs.

## Les trois directions

| | Direction | Forces | Limites |
|---|---|---|---|
| **A** | **L'onglet de la PR** (`captures/a-*.png`). La PR comme chez l'hébergeur, dans la place de travail (comme le Kanban), avec les onglets Conversation, Commits, Contrôles et Fichiers. Les remarques de l'agent sont des fils dans le diff, sous leur ligne. La colonne de droite porte les relecteurs, les étiquettes, brouillon ou prête et la branche, avec une boîte de merge qui dit ce qui manque. | Familière (GitHub, GitLab) ; tout au même endroit, contrôle toujours visible ; prolonge l'existant (la vue « Pull requests » ouvre la PR, la relecture d'ESC-66 devient l'onglet Fichiers) ; ne dépend pas du chantier de l'éditeur. | Refaire une page d'hébergeur (diff, fils, timeline, merge) ; un diff à nous, sans serveur de langage ; l'agent qui corrige travaille ailleurs ; une vue de plus. |
| **B** | **La relecture dans l'éditeur** (`captures/b-*.png`). Ouvrir une PR met l'éditeur en mode relecture : les fichiers modifiés, le diff, et une marge où chaque remarque se pose en face de sa ligne. Une barre porte l'état, la CI, les approbations, « Approuver » et « Merger » ; un tiroir porte la description, les commits, les contrôles, la discussion et les relecteurs. | Le code d'abord, le fichier entier à un clic, rien ne cache une ligne ; avec la vue VS Code décidée pour la 2.3, on aurait les serveurs de langage pendant la relecture et l'API des commentaires de VS Code, faite pour cela. | Dépend de l'éditeur de la 2.3 (sinon, tout faire deux fois) ; la PR se voit mal en entier (tiroir) ; la marge de 340 px est serrée sous 1280 px ; la plus chère. |
| **C** | **La PR à côté de l'agent** (`captures/c-*.png`). La PR en colonne à côté de la conversation de l'agent (du ticket, ou le relecteur). En tête, une carte « Prête à merger ? » en quatre cases (contrôles, relecture, approbations, branche), chacune avec ce qui bloque et le geste. Les remarques défilent avec leurs lignes. « Demander à l'agent de corriger » dépose la remarque dans la conversation d'à côté. | L'identité d'Escouade (on voit les agents relire et corriger) ; la carte dit d'un coup ce qui manque pour merger ; le passage remarque → correction se fait sans quitter l'écran. | Colonne étroite pour un diff entier ou une longue relecture humaine ; une PR sans agent laisse la colonne de gauche vide ; les remarques perdent l'ordre des fichiers. |

Chaque direction montre les quatre états (`*-etats.png`) : relecture en cours (l'agent relit, ses remarques arrivent), CI rouge (journal, relancer, faire corriger), conflit (mettre à jour la branche, faire résoudre), prête à fusionner (le merge et sa méthode).

## Recommandation : A, avec deux emprunts

**A** répond aux trois demandes sans attendre l'éditeur. Elle prolonge ce qui existe, et se lit comme GitHub, ce que l'utilisateur et ses collègues savent déjà faire.

- **De C**, deux choses. D'abord la boîte « prête à merger ? », qui dit ce qui bloque avec le geste qui débloque (déjà dans A, en bas à droite). Ensuite le geste « Demander à l'agent de corriger », qui ouvre l'agent dans une colonne à côté de la PR (vue côte à côte) au lieu de l'envoyer ailleurs.
- **De B**, « Ouvrir dans l'éditeur » sur chaque fil. Quand la vue VS Code sera là, une PR pourra s'y relire avec la marge, sans refaire A.

B serait la meilleure relecture de code, mais elle attend l'éditeur et cache la PR. C est la plus « Escouade », mais elle relit mal un gros diff et une PR sans agent.

### Questions à trancher avec la direction

- Les remarques de l'agent restent-elles **locales** (fils marqués « agent, pas publié », publiés un à un ou tous) ou partent-elles **chez l'hébergeur** sous le compte de l'utilisateur dès qu'il les garde ? Le découpage suppose le premier : rien ne part sans son clic, comme aujourd'hui.
- « Demander à l'agent de corriger » sur une PR **sans ticket ni agent** : lancer un agent sur la PR (ESC-65 sait le faire) ou refuser ? Le découpage suppose de le lancer.

## Découpage proposé (direction A)

| # | Ticket | Contenu | Dépend de |
|---|---|---|---|
| PR-1 | Lire une PR entière | Opérations des plugins d'hébergeur : description, commits, fichiers (le diff vient de git, par les références `refs/escouade/review/…` d'ESC-66), fils avec leurs lignes, leurs réponses et leur état ; capacités déclarées. GitHub d'abord, faux GitHub des e2e étendu. | — |
| PR-2 | La vue de la PR | La place de travail : en-tête (numéro, titre, état, branches, ticket, auteur), onglets Conversation, Commits et Contrôles (journal et relance d'un job). On l'ouvre depuis la vue « Pull requests », la carte « En revue », l'onglet « PR et CI » et la pastille de l'agent ; « Ouvrir sur GitHub » reste un bouton. En lecture seule. | PR-1 |
| PR-3 | L'onglet Fichiers | Diff unifié coloré (CodeMirror), arbre des fichiers modifiés, « Vu » gardé par PR et par commit, gros fichiers et fichiers générés repliés, « Ouvrir dans l'éditeur ». | PR-2 |
| PR-4 | Les fils | Commenter une ligne ou la PR, répondre, résoudre, rouvrir, chez l'hébergeur ; fils résolus repliés ; « Suivante » d'un fil ouvert à l'autre. | PR-3 |
| PR-5 | L'agent relecteur en fils | La relecture d'ESC-66 devient des fils de l'agent sous leur ligne : sévérité (sécurité, spec, détail), suggestions de code, résumé dans la Conversation. Le tri garder/écarter devient résoudre/publier. Toujours sans outil, toujours rien de publié sans clic. | PR-4 |
| PR-6 | Parler à l'agent et faire corriger | L'agent relecteur répond dans le fil (question ponctuelle, sans outil). « Demander à l'agent de corriger » passe la remarque à l'agent du ticket ou de la PR, ou en lance un, et l'ouvre à côté ; le fil dit « corrige… » puis le commit. « Appliquer la suggestion » la commite dans le worktree. | PR-5, vue côte à côte |
| PR-7 | Ta relecture | « Ta relecture » : commenter, approuver, demander des changements, avec un texte ; les changements demandés arrivent à l'agent du ticket comme retours (ESC-61). | PR-4 |
| PR-8 | Le contrôle de la PR | Brouillon ↔ prête, relecteurs, étiquettes (nouvelles opérations des plugins) ; mise à jour de la branche (API de l'hébergeur, sinon l'agent) ; relance d'un contrôle ; boîte de merge : ce qui manque, méthode, suppression de la branche et du worktree, auto-merge, ticket passé en Terminé (reprend ESC-62). | PR-2 |
| PR-9 | États, spec et hébergeurs | Les quatre états (relecture en cours, CI rouge, conflit, prête) ; spec (pages *Pull requests*, *Relecture d'une PR par un agent*, *Vue « Pull requests »*) et doc du site ; GitLab, Gitea / Forgejo, Bitbucket et Azure DevOps selon leurs capacités. | PR-1 à PR-8 |

PR-2 à PR-4 donnent déjà « voir la PR dans Escouade ». PR-5 et PR-6 donnent « l'agent qui commente comme un humain, à côté du code ». PR-7 et PR-8 donnent « un meilleur contrôle ».
