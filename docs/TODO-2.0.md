# Escouade 2.0 — à faire

Le chantier principal : le nouveau design de l'app, dans la langue visuelle du site. Viennent ensuite deux chantiers reportés de la 1.8 (voir « Reportés de la 1.8 ») et les projets privés (ESC-139 à ESC-142).

Tickets du Kanban d'Escouade, créés le 2026-10-14 : ESC-71 à ESC-85 (les 11 lots, les deux nouveautés, la revue finale, puis la vidéo et le site). Le premier attend la fin de la 1.8, nouveau récit du site (ESC-86) compris.

## Reportés de la 1.8 (2026-10-19)

La 1.8.0 est sortie avec Codex, OpenCode et Copilot CLI à côté de Claude Code. Deux chantiers de son plan n'y sont pas entrés ; leur plan détaillé reste dans `docs/TODO-1.8.md` :
- **Hébergeurs de code** (GitHub, GitLab, Bitbucket, Gitea) : les pull requests des tickets suivies jusqu'au merge, avec leur CI et leurs déploiements. Plan : `docs/TODO-1.8.md`, chapitre 2 ; tickets ESC-53 à ESC-70, après le nouveau design.
- **Cursor CLI**, cinquième harness, par ACP : plan dans `docs/TODO-1.8.md`, chapitre 1 ; tickets ESC-24 (essais réels), ESC-42 et ESC-43 (l'adaptateur, ses comptes et son coût) et ESC-49 (sa vue terminal).

## Projets privés (demandé le 2026-10-20)

« Pouvoir cloisonner totalement chaque projet : un projet isolé ne doit pas être connu d'un agent dans un autre projet, et il faut cloisonner aussi dans le MCP. Rendre ça configurable. » Tickets ESC-139 (réglage et MCP), puis ESC-140 (fichiers), ESC-141 (comptes) et ESC-142 (ce qu'Escouade transmet), qui attendent le premier.

Aujourd'hui, un agent d'Escouade n'agit que dans son projet, mais il lit tous les autres par le serveur MCP (`mcp/act.rs`, `own_project_only` : « it may read the others » ; `list_projects`, `list_agents`, `list_tickets`, `get_agent_summary`, `list_background_work` ouverts à tous les projets), et ses outils de fichiers peuvent lire les dossiers des autres projets, le dossier de données d'Escouade (`~/.escouade` : tickets, images, conversations) et les sessions du harness (`~/.claude/projects/…`).

- [x] **Le réglage** (ESC-139) : « Projet privé » dans les réglages d'un projet (éteint par défaut), et un défaut pour les nouveaux projets dans Réglages › Projets.
- [x] **MCP, dans les deux sens** (ESC-139) : un projet privé n'existe pas pour les agents des autres projets (absent de `list_projects`, ses agents, tickets, sous-agents, tâches de fond et flux de `wait_ticket` invisibles, « introuvable » plutôt que « refusé ») ; ses propres agents ne voient que lui. Chaque outil couvert par un test.
- [x] **Fichiers** (ESC-140) : les agents des autres projets ne lisent ni son dossier, ni ses worktrees, ni ses données dans le dossier d'Escouade, ni ses sessions de harness ; les siens ne lisent pas ceux des autres. Par les règles de chaque harness (Claude Code : `permissions.deny` des chemins ; Codex : son bac à sable ; OpenCode et Copilot CLI : ce qu'ils offrent), en disant honnêtement dans l'interface ce qui est un garde-fou et ce qui est une vraie barrière. Fait : `fence.rs` ; Claude Code `permissions.deny` (garde-fou), Codex profils de permissions (barrière, sauf Bypass), OpenCode règles de session (garde-fou), Copilot CLI refus d'office par Escouade (garde-fou, rien en Bypass ni en vue terminal) ; « Ses fichiers » dans Réglages › Projets ; essais réels `real_fence`.
- [x] **Comptes** (ESC-141) : proposer un compte réservé au projet (« Principal réservable » existe) pour que sessions, mémoire et historique du harness ne soient pas partagés ; un compte encore partagé est signalé (réglages du projet, Réglages › Comptes) avec de quoi y remédier, et aucune bascule automatique sur un compte partagé sans « Comptes partagés à la limite ».
- [x] **Ce qu'Escouade transmet** (ESC-142) : rien d'un autre projet dans ce qu'il écrit aux agents (consignes, messages, passations, textes rédigés), ni d'un projet privé vers les autres (dupliquer une conversation, copier les réglages d'un projet dans l'assistant « Nouveau projet », dépendances entre tickets).
- [ ] **À décider** (pris par défaut dans ESC-139, à confirmer) : « Claude (hors Escouade) », le jeton global de l'utilisateur, voit-il quand même tout (c'est l'utilisateur) ou faut-il un jeton par projet ? Un projet privé apparaît-il dans les statistiques et la vue d'ensemble globales (pour l'utilisateur, oui a priori) ?

## D'où ça vient (2026-10-12)

L'utilisateur juge l'interface actuelle « très AI slop » et veut un design « plus sleek, sexy ». Pour lui, « sexy » veut dire fini et soigné, pas à thème : il a déjà écarté les directions à thème pour le site.

Ses réponses, à ne pas reposer :
- l'app parle la même langue visuelle que le site (`website/app/assets/main.css`) : fond presque noir #08090A, Inter, filets blancs, teal #35C1BD du logo ;
- le teint de toute l'interface par la couleur du projet actif reste (« c'est cool ») ;
- d'abord un aperçu dans un navigateur, sans implémenter ; sur cet aperçu : « la direction me plaît bien », puis, une fois tous les écrans ajoutés, « implémente le redesign comme ça ».

L'implémentation a ensuite été repoussée à la 2.0, avec ce fichier et les artefacts ci-dessous pour tout contexte.

## Les artefacts

- **L'aperçu** : `design/app-redesign/index.html`, à ouvrir dans Chrome depuis le dépôt (il charge Inter depuis `website/node_modules` et JetBrains Mono depuis `node_modules`). Données fictives. Il est interactif :
  - les onglets de projet changent le teint ;
  - Agents et Kanban ;
  - réponse à `tests-e2e` depuis la liste (clic, ou 1 et 2), Ctrl+J ;
  - plan qui se plie ;
  - Lancement › Front, Terminaux › PowerShell ;
  - Kanban › DEM-4 › Tester, puis Lancer ;
  - Éditeur, Vue d'ensemble, Stats, réglages (Ctrl+,) ;
  - depuis le lot 0 : la flèche de « Nouvel agent », le mode du champ, les jauges de la barre d'état, « … » de l'en-tête (ou clic droit sur une carte), le micro (ou Ctrl+Espace), la conversation, « + » d'une colonne du Kanban, Réglages › Comptes › Ajouter un compte (Codex), Réglages › Projets › Ouvrir l'assistant.
- **Chaque écran par son adresse** : `index.html#board`, `#plan-closed`, `#git`, `#history`, `#terminal`, `#run`, `#test`, `#test-run`, `#editor`, `#overview`, `#stats`, `#settings`, `#settings-<onglet>` (`app`, `claude` pour Harness, `accounts`, `voice`, `notifications`, `projects`, `board`, `integrations`, `mcp`, `terminals`, `network`, `about`).
- **Les écrans du lot 0** (ESC-116, les fonctionnalités venues après l'aperçu) : `#new-agent`, `#codex`, `#modes`, `#handover`, `#agent-menu`, `#quota`, `#account-codex`, `#terminal-view`, `#welcome`, `#dictation`, `#dictation-offer`, `#voice`, `#voice-question`, `#attachments`, `#drop`, `#ticket`, `#subagent`, `#task`, `#wizard`, `#wizard-summary`, `#telemetry`, `#update`, `#update-failed`, `#hosts`. Avec `?theme=light` (ou `dark`, `system`) avant le `#`, le thème ; avec `?lang=en`, les libellés anglais (ceux du catalogue `src/lib/i18n/en` quand il les a), par exemple `index.html?lang=en#modes`.
- **Les écrans du lot 0 bis** (ESC-150, les fonctionnalités de la 1.8.0 à la 1.8.9) : `#harness-opencode`, `#accounts-copilot`, `#accounts-quotas`, `#account-machine`, `#no-account`, `#welcome-account`, `#overage`, `#opencode`, `#modes-opencode`, `#copilot`, `#models`, `#opencode-view`, `#copilot-view`, `#switch-account`, `#waiting`, `#run-worktree`, `#run-conflict`, `#worktree-kept`, `#pr-open`, `#pr`, `#voice-live`, `#dictation-live`, `#settings-voice-online`, `#voice-online-voice`, `#code-hosts`, `#private`, `#private-accounts`, `#escouade-yml`, `#escouade-yml-pending`, `#escouade-yml-board`, `#worktree-cleanup`, `#wizard-private` ; et, mis à jour à leur adresse, `#new-agent`, `#agent-menu`, `#quota`, `#account-codex`, `#voice-question`, `#welcome` et les onglets Harness, Comptes, Application, Voix, Kanban, Projets, Intégrations, Serveur MCP et À propos. Ce qu'ils ajoutent à une surface partagée (barre d'état, panneau des quotas, liste des agents) n'apparaît qu'à leurs adresses, pour que les autres captures restent celles validées.
- **Les notes d'UX du lot 0** : `design/app-redesign/UX-NOTES.md`. Pour chaque fonctionnalité, ce qui est mal pensé aujourd'hui et ce que l'aperçu propose ; les libellés anglais vérifiés ; les choix qui engagent le produit, tous validés par l'utilisateur le 2026-10-17. Sa section « Lot 0 bis » (2026-10-23) a les adresses, les libellés anglais vérifiés et les choix 14 à 38, tous validés par l'utilisateur le 2026-10-23.
- **Les captures** : `design/app-redesign/*.png`, refaites par `node design/app-redesign/render.mjs [nom…]` (le rendu échoue si un écran lève une erreur). Elles sont prises à 1440×900, la taille par défaut de l'app ; celles en `-small` à 1028×779, la fenêtre de la CI ; celles en `-en` en anglais. Chaque capture est sombre, et chacune à 1440×900 a sa jumelle claire `<nom>-light.png` (`?theme=light`), comme quelques-unes à 1028×779 et en anglais (`agents-small-light-en.png`…). `settings-app.png` montre le sélecteur « Thème ».
- **Le contrôle de contraste** : `node design/app-redesign/contrast.mjs [--verbose]`, dans les deux thèmes, pour chaque couleur de projet (voir « Le thème clair »).
- **Le contrat de direction** : `.impeccable/surfaces/src-app-svelte.md`. La commande `impeccable surface-brief read src/App.svelte` le relit.
- **Le produit** : `PRODUCT.md`.

## Le monde visuel (ce que l'aperçu fixe)

**Couleurs.** `--p` est la couleur du projet (`oklch(0.72 0.12 <teinte>)`, les teintes de `PROJECT_COLORS`). Elle est déclarée par `@property` et glisse en 0,7 s d'un projet à l'autre. Les surfaces s'en déduisent :

| Rôle | Valeur |
|---|---|
| Cadre (barre de titre, barre latérale, barre d'état) | `color-mix(in oklch, var(--p) 8%, #08090a)` |
| Panneau du travail | `color-mix(in oklch, var(--p) 4.5%, #0e0f11)` |
| Reliefs | 6 % sur #15161a et sur #1b1c20 |
| Puits (code, terminal) | 2 % sur #0a0b0d |
| Filets | blanc à 7,5 %, 12 % et 20 % |
| Survol | blanc à 4,5 % |
| Textes | #F7F8F8, #D0D3D8, #8A8F98, #7B8089 |

D'autres couleurs viennent compléter les surfaces :
- **Lumière du projet** : un dégradé radial du projet à 20 %, depuis le haut à gauche, sur 520 px.
- **Vue d'ensemble et stats** : elles prennent le teal d'Escouade, `oklch(0.74 0.115 190)`.
- **États** : en cours oklch(0.79 0.15 155), attente oklch(0.84 0.14 82) (fond à 7 %, filet à 32 %), erreur oklch(0.74 0.14 22), ajout oklch(0.8 0.13 155), git modifié oklch(0.82 0.11 75).
- **Bouton principal** : blanc (#F2F3F4, encre #08090A). Le teal reste au logo et aux vues globales. Le focus et la sélection prennent la couleur du projet.

**Graphiques.** Palettes validées par le script de dataviz (daltonismes compris) :
- tokens : Entrée #4c88d3, Cache #008b7a, Sortie #9555b0 ;
- modèles : Opus #ca723c, Sonnet #4682cc, Fable #339c6d, Haiku #9555b0.

Les couleurs de projet échouent au contrôle : ce sont celles du produit, donc chaque barre porte le nom du projet.

**Typographie.**
- Inter variable (`@fontsource-variable/inter`, réglages `cv11` et `ss03`), 13px de base, chiffres tabulaires pour les tokens, coûts et durées.
- JetBrains Mono seulement pour le code, les chemins, les branches et les commandes.

**Formes et mouvement.**
- Coins de 6 à 14px. Ombres seulement sous ce qui flotte (panneau, zone de saisie, fenêtres, notifications).
- Icônes SVG tracées en 1,5px sur une grille de 16, une seule famille. Elles remplacent les caractères Unicode qui servent d'icônes : 92 dans les gabarits (× ✓ ↑ ↓ ▾ ▶ ▸ ■ ⌕ ⟳ ⚙…), plus ceux des textes (« ▶ Tester », « ✦ Remplir automatiquement ») et des onglets de réglages (`SETTINGS_TABS`) ; les tracés sont dans l'aperçu (`<symbol id="i-…">`).
- Mouvement : courbe `cubic-bezier(0.16, 1, 0.3, 1)`. Trois moments seulement : le teint qui glisse, l'anneau de l'agent qui attend, le reflet de « Claude travaille… ». Tout s'arrête avec `prefers-reduced-motion`.

## Le thème clair (ESC-129)

Demandé par l'utilisateur le 2026-10-19. Le design ci-dessus est le thème sombre ; l'app a aussi un thème clair, fait sur l'aperçu avant d'être codé (ESC-129, captures `-light`). La direction claire est validée par l'utilisateur sur ces captures, le 2026-10-23.

**Le réglage.**
- Réglages › Application › Affichage › « Thème » : Système, Clair, Sombre, en segments.
- **Par défaut : Système** (décidé par l'utilisateur le 2026-10-23). Il suit le thème de Windows, macOS ou Linux (`prefers-color-scheme`) et ses changements, à chaud.
- Appliqué tout de suite, sans attendre « Enregistrer » : l'onglet n'est pas marqué modifié.
- Le choix est lu avant le premier rendu (pas d'éclair sombre au lancement d'une app claire). Dans l'app, à faire au lot 1 : le réglage lu côté Rust avant de créer la fenêtre, pour qu'elle naisse dans le bon thème.
- **La fenêtre native suit** : `color-scheme: light | dark` sur la racine (barres de défilement, champs natifs, menus), et le thème de la fenêtre Tauri (`Window::set_theme(Some(Theme::Light | Dark))`, `None` pour Système) pour la barre de titre native de macOS, les coins et l'ombre de Windows 11 et le fond affiché avant la WebView. Sous Linux, le thème GTK de la fenêtre.
- Dans l'aperçu : `data-theme="light"` sur `<html>`, posé par le petit script de `<head>` ; `?theme=light`, `?theme=dark` ou `?theme=system` dans l'adresse l'emporte (pour les captures) ; le choix fait dans le sélecteur est gardé (`localStorage`).

**Les jetons.** Ce sont les mêmes noms dans les deux thèmes ; le clair redéfinit leurs valeurs (`:root[data-theme='light']` dans l'aperçu). Toute couleur écrite en dur dans l'aperçu est devenue un jeton (ESC-129), sans changer un pixel du sombre : les 66 captures sombres sont identiques, octet pour octet, avant et après.

| Rôle | Jeton | Sombre | Clair |
|---|---|---|---|
| Cadre (barres, latérale) | `--frame` | `--p` 8 % sur #08090a | `--p` 7 % sur #f0f1f3 |
| Panneau du travail | `--panel` | `--p` 4,5 % sur #0e0f11 | `--p` 2,5 % sur #fdfdfd |
| Reliefs | `--raise`, `--raise-2` | `--p` 6 % sur #15161a, #1b1c20 | `--p` 4 % sur #f4f5f7, 5 % sur #eceef1 |
| Puits (code, terminal) | `--well` | `--p` 2 % sur #0a0b0d | `--p` 2,5 % sur #f6f7f9 |
| Encre des filets, survols, reliefs translucides | `--fg-rgb` | 255, 255, 255 | 15, 17, 22 |
| Filets | `--line`, `--line-2`, `--line-3` | blanc 7,5 %, 12 %, 20 % | encre 9 %, 14 %, 24 % |
| Survol | `--hover` | blanc 4,5 % | encre 4,5 % |
| Ombres et assombrissements | `--shade` (multiplie l'alpha) | 1 | 0,3 |
| Voile des fenêtres | `--scrim` | rgba(3, 4, 5, 0.62) | rgba(24, 26, 32, 0.3) |
| Textes | `--text`, `--soft`, `--muted`, `--dim` | #F7F8F8, #D0D3D8, #8A8F98, #7B8089 | #15171C, #363A42, #555A63, #5C616A |
| Réponse de l'agent | `--text-ai` | #E9EBEE | #1D2026 |
| Couleur du projet en texte (liens, focus) | `--p-ink` | `--p` 70 % avec blanc | `--p` 50 % avec #15171c |
| Lumière du projet | `--glow` | 20 % | 10 % |
| Sélection | `--sel` | `--p` à 40 % | `--p` à 26 % |
| Teal d'Escouade (icônes) | `--brand` | #35C1BD | oklch(0.52 0.09 190) |
| En cours | `--run` | oklch(0.79 0.15 155) | oklch(0.48 0.13 155) |
| Attente | `--wait` (fond 7 %/9 %, filet 32 %/40 %) | oklch(0.84 0.14 82) | oklch(0.5 0.11 70) |
| Erreur, suppression | `--del` | oklch(0.74 0.14 22) | oklch(0.5 0.18 25) |
| Ajout | `--add` | oklch(0.8 0.13 155) | oklch(0.47 0.12 155) |
| Git modifié | `--git-m` | oklch(0.82 0.11 75) | oklch(0.5 0.11 65) |
| Avertissement en texte | `--warn-ink` | oklch(0.8 0.1 60) | oklch(0.49 0.11 55) |
| Teal en texte | `--teal-ink`, `--teal-ink-2` | oklch(0.86 0.08 190), oklch(0.82 0.09 190) | oklch(0.47 0.08 190) |
| Points inactifs, cases vides | `--idle`, `--ring` | #5D626B, #4D5259 | #7A7F88 |
| Séparateurs « · » | `--sep` | #4A4E55 | #B0B4BB |
| Bouton principal | `--primary`, survol `--primary-hover`, texte `--on-primary` | #F2F3F4, #FFF, #08090A | #1B1D22, #30333A, #F7F8F8 |
| Pastille qui attend (chiffre) | `--on-wait` | #1A1405 | #FFF |
| Voix (fond, orbe) | `--vo-base`, `--orb-base` | #070809, #0B0C0E | #F3F4F6, #DFE2E7 |

Les puces d'état prennent leur fond et leur filet de leur couleur (`oklch(from var(--run) l c h / 0.1)`), donc suivent le thème. Le focus et la sélection restent à la couleur du projet. Les interrupteurs allumés gardent `--p` 75 % avec noir et leur bouton blanc.

**Code, terminal, éditeur.**

| Rôle | Jeton | Sombre | Clair |
|---|---|---|---|
| Texte du code et du terminal | `--code-fg` | #D5D8DD | #24272E |
| Commentaires, `dim` du terminal | `--code-dim` | #6C717A | #62676F |
| Numéros de ligne (diff, éditeur) | `--code-ln`, `--code-nb` | #555A62, #565B63 | #8F949C |
| Mots-clés | `--syn-k` | oklch(0.78 0.11 300) | oklch(0.5 0.16 300) |
| Fonctions | `--syn-f` | oklch(0.83 0.09 235) | oklch(0.5 0.13 250) |
| Chaînes | `--syn-s` | oklch(0.82 0.11 150) | oklch(0.5 0.12 150) |
| Types | `--syn-t` | oklch(0.84 0.08 190) | oklch(0.5 0.09 200) |
| Nombres | `--syn-n` | oklch(0.84 0.11 65) | oklch(0.53 0.13 55) |
| ANSI vert, rouge | `--add`, `--del` | ci-dessus | ci-dessus |
| ANSI jaune | `--ansi-y` | oklch(0.85 0.12 85) | oklch(0.54 0.12 80) |
| ANSI cyan, liens | `--ansi-cy`, `--ansi-cy-2` | oklch(0.82 0.09 205), oklch(0.8 0.1 210) | oklch(0.5 0.1 215) |
| Sélection d'un TUI (vidéo inverse) | `--term-sel-bg`, `--term-sel-fg` | #C8CCD2 sur #08090A | #2A2E35 sous #F7F8F8 |
| Occurrence courante (recherche de l'éditeur) | `--on-hit` | #FFF | #15171C |

Pour le lot 8, le thème clair d'xterm reprend ces valeurs : `foreground` `--code-fg`, `background` `--well`, `cursor` `--text`, `selectionBackground` `--p` à 26 %, et les 16 couleurs ANSI foncées comme ci-dessus ; proposé pour celles que l'aperçu ne montre pas : blanc #4A4E55, blanc vif #15171C, noir #F6F7F9, noir vif #8F949C, à passer au contrôle de contraste au lot 8. Le lot 9 fait de même pour CodeMirror (`src/lib/editor/theme.ts`, un `HighlightStyle` par thème, choisi par un `Compartment` qui change à chaud).

**Graphiques.** Les deux palettes gardent leurs couleurs dans les deux thèmes : revalidées par le script de dataviz sur le vrai panneau des stats (#f9fcfb en clair, #101616 en sombre), toutes les vérifications passent (bande de luminosité, chroma, daltonismes ≥ 9,4, vision normale ≥ 15,2, contraste ≥ 3:1). Grilles et axes passent par `--fg-rgb`. Les couleurs de projet restent celles du produit : en clair aussi, chaque barre et chaque pastille porte le nom du projet.

**Contraste.** `node design/app-redesign/contrast.mjs` lit chaque jeton dans le navigateur, pour chaque couleur de `PROJECT_COLORS` (et le teal des vues globales), sur chaque surface où il se pose : cadre, cadre sous la lumière du projet, cadre et panneau survolés, panneau, reliefs, puits, carte qui attend, puce du projet ; et les textes posés sur une couleur (bouton principal, pastille d'attente, sélection du terminal, occurrence courante, texte sélectionné). Seuil AA : 4,5:1 pour un texte, 3:1 pour un point ou un anneau.
- **Clair : 183 paires × 15 couleurs, toutes au-dessus du seuil.** Les plus justes : le jaune ANSI sur relief (4,54), l'attente sur la carte qui attend (4,57), `--dim` sur cette carte (4,63) ; les anneaux à 3,08 sur le cadre survolé.
- **Sombre : 28 paires sous le seuil, déjà là avant ESC-129** et laissées telles quelles (l'aperçu sombre ne change pas) : `--muted`, `--dim` et `--done` sous la lumière du projet (jusqu'à 3,06 avec la couleur grise), `--dim` sur les reliefs et les survols (3,9 à 4,2), `--code-dim` (3,4 à 3,95), `--idle` et `--ring` (1,5 à 3). Le lot 1 les remonte jusqu'à ce que `contrast.mjs` passe aussi en sombre ; l'écart visuel est faible (quelques points de luminosité sur des gris).

**Ce que chaque lot vérifie dans les deux thèmes.**
- Aucune couleur en dur dans un composant : tout passe par un jeton (`src/app.css`), qui a ses deux valeurs.
- Les captures de l'app réelle (banc e2e, 1440×900 et 1028×779) dans les deux thèmes, comparées à celles de l'aperçu (`<écran>.png` et `<écran>-light.png`).
- Le contrôle de contraste repris pour les jetons de l'app (un test Vitest qui lit les deux tables, chaque couleur de projet).
- Le changement de thème à chaud : rien ne garde l'ancien (xterm, CodeMirror, graphiques redessinés, `color-scheme`, fenêtre native).
- Lot 1 : le réglage, son défaut Système, la lecture avant le premier rendu, la fenêtre native. Lot 8 : thèmes d'xterm. Lot 9 : thèmes de CodeMirror. Lot 10 : graphiques. Lot 11 : le sélecteur dans Réglages › Application.

## Écran par écran

- **Coque.**
  - Cadre teinté. Le travail est posé sur un panneau en relief (rayon 12px, marge droite de 8px).
  - Barre de titre de 44px : logo, Vue d'ensemble, onglets de projet (pastille, nom, nombre d'agents qui attendent en ambre, Δ), recherche Ctrl K, Éditeur, Stats.
  - Barre d'état de 30px : « 1 en attente Ctrl J » cliquable, jauges de quota fines.
- **Barre latérale.**
  - Agents et Kanban en segments, puis « Nouvel agent Ctrl N ».
  - Cartes d'agent sans cadre. L'agent qui attend est en tête, ambre, avec sa question et ses réponses.
  - Sections Lancement (état de chaque commande, relancer et stopper au survol, groupe d'un test en cours) et Terminaux, puis le dossier et la branche.
- **Conversation.**
  - En-tête de 60px : nom, état en pastille, chemin, chiffres (étiquette au-dessus de la valeur), actions. Les libellés se replient en icônes quand la place manque (requêtes de conteneur).
  - Colonne de 780px. Appels d'outils en un fil, sans une boîte par appel. Le code et les diffs gardent un cadre.
  - Zone de saisie flottante : modèle, effort et mode en puces, Stop avec Échap, Envoyer en blanc. Sa barre passe à la ligne quand la place manque.
- **Plan.**
  - Aligné sur la colonne de la conversation : barre d'une case par tâche, titre, sous-agents actifs, nombre de tâches, pourcentage.
  - Chaque tâche montre son état, son sous-agent et son modèle, son étape (« 2/4 étapes ») et ses dépendances (« après 04, 05 »).
  - Poignée de redimensionnement.
- **Kanban.** Colonnes sans boîte. Cartes avec la clé, la boucle en cases, les critères cochés, l'activité de l'agent et le coût. Le ticket à tester est ambre, avec Valider et Tester.
- **Git, en disposition côte à côte.**
  - Onglets soulignés « Non commités » et « Historique ».
  - Portée « Cet agent / Tout le projet ». Fichiers avec M ou A et leurs compteurs, puis le diff.
  - Voir le diff, Commit…, Merger.
  - L'historique est un graphe des branches des agents.
- **Terminal et lancement.**
  - Log en vrai terminal, avec la palette ANSI de l'aperçu.
  - En-tête : nom, shell, état, « depuis 14:02 », commande et dossier, Effacer, Relancer, Stopper.
  - Une notification signale le plantage d'une commande, avec « Voir le log ».
- **Tester.**
  - Fenêtre qui dit d'abord qui a écrit les commandes et qu'elles tournent hors du mode de permission. Puis la préparation, les processus (dossier, variables, adresse) et l'ouverture.
  - Après « Lancer », les étapes s'enchaînent avec leur temps de démarrage.
- **Éditeur.** Arbre avec les dossiers modifiés marqués, onglets, recherche flottante, repères de changement dans la marge, ligne d'état.
- **Vue d'ensemble.** Plein cadre. Groupe « Attend ta réponse » en tête, avec Autoriser/Refuser ou Répondre sur place, puis un groupe par projet. Colonnes : activité, modèle, coût, contexte, depuis.
- **Statistiques.**
  - Une seule bande de quatre chiffres au lieu de quatre cartes.
  - Colonnes empilées, avec un écart de 2px entre les segments et le détail d'un jour au survol.
  - Barres par projet et par modèle, table par ticket.
- **Réglages.**
  - Navigation en trois groupes : général, « Par projet », « Système ». Un point de la couleur du projet marque un onglet modifié.
  - Lignes à filets : nom, explication, contrôle à droite. Interrupteurs, cases et boutons radio dans la couleur du projet.
  - Projet choisi par pastilles. Commandes de lancement sur une grille (nom, commande, sous-dossier, shell).

Puis les écrans du lot 0 (ESC-116), validés par l'utilisateur le 2026-10-17 avec les 13 choix de `design/app-redesign/UX-NOTES.md` (le détail et les raisons y sont) : c'est le design à suivre par les lots, pas une proposition. **Les harness sont les quatre que l'app connaît : Claude Code, Codex, OpenCode et Copilot CLI** (règle du lot 0 bis, ESC-150, qui remplace « Claude Code et Codex, eux seuls » du 2026-10-17) ; Cursor CLI, reporté, n'apparaît pas dans l'interface tant qu'il n'est pas fait (ESC-24, 42, 43, 49).

- **Harness.**
  - La marque d'un harness est un mot dans un filet (« Claude », « Codex »), la même sur la carte, l'en-tête (sur la ligne du chemin), le ticket, les menus et le Kanban ; jamais une couleur.
  - « Nouvel agent » en bouton divisé : `Ctrl N` sur le harness du projet ; la flèche ouvre le menu « Harness » (modèle, mode et raccourci de chacun ; Claude Code ou Codex pas installé, grisé avec où le régler ; « Harness par défaut du projet… »).
  - Un agent Codex porte « Bac à sable » dans son en-tête. Le menu des modes dit le sens de chaque mode sur son harness ; les cinq modes restent aux mêmes places.
  - Réglages › Harness : un groupe par harness (état, version, exécutable, nouveaux agents), puis « Tous les harness ». Ce qu'un harness ne fait pas est dit en une ligne, à l'endroit du réglage.
  - Premier démarrage sans harness : la fenêtre vide, les commandes d'installation à copier, « Chercher à nouveau ».
- **Passation (ESC-88).** La carte du quota épuisé, le séparateur « Suite sur Codex · … », le dossier de passation replié, les tours marqués de leur harness. « Revenir sur Claude Code » dans une bande au-dessus du champ, jamais pris seul. Le menu de l'agent s'ouvre aussi par « … » dans l'en-tête.
- **Comptes et quotas.**
  - Barre d'état : un groupe de jauges par harness qui travaille ; le total du jour prend « ≈ » dès qu'il contient de l'estimé.
  - Panneau des comptes groupé par harness : fenêtres de 5 h, 7 j, ou 30 j (offre gratuite de Codex), coût du jour « ≈ … estimé » pour Codex, seuil de pause.
  - Réglages › Comptes : un groupe par harness, puis « Quand un quota est épuisé » (autre compte, harness suivant, ordre, modèle de passation). Ajouter un compte : le harness d'abord, puis la connexion dans un terminal.
- **Vue terminal.** Deux segments « Escouade | Codex » dans l'en-tête. La question de confiance du dossier de Codex est dite au-dessus du terminal, et l'agent passe en tête de la liste.
- **Voix.**
  - Barre du champ : modèle, effort et mode à gauche ; joindre, dicter et converser en icônes à droite, avant Stop et Envoyer.
  - Dictée dans le champ (niveau, durée, touches) ; l'offre du modèle et sa progression au même endroit.
  - Conversation plein écran (direction A) : orbe, phase, sous-titres, choix d'une question tels qu'ils se disent, Pause, Arrêter. « Dit à voix haute » et « Lu à voix haute » dans la conversation.
  - Réglages › Voix : Dictée (micro, raccourci, langue, modèles) puis Conversation (voix par langue, licences, exception de Fuze, vitesse, délai).
- **Pièces jointes.** Une rangée de puces au message comme au ticket : vignette ou icône, taille, ce qui arrive au fichier, gros fichier en ambre, copie en cours. Glisser entoure le champ. Trombone et nombre sur la carte du Kanban ; champ « Harness » du ticket.
- **Sous-agents et tâches de fond.**
  - Deux compteurs sur la carte ; la liste n'est dépliée que pour l'agent ouvert.
  - Leurs vues : fil d'Ariane, faits, consigne ou sortie suivie, Arrêter, « Lui écrire, par … ».
  - À 820 px de haut et moins, Lancement et Terminaux se replient sur leur titre ; un plantage reste dit.
- **Assistant « Nouveau projet ».** Rail des étapes ; le récapitulatif met d'abord ce qui empêcherait la méthode de marcher, avec sa correction. Projets fermés dans Réglages › Projets.
- **Serveur MCP.** Un onglet « Serveur MCP » (Système) : interrupteur, déclaration, journal. « via Escouade » à côté de l'auteur dans la conversation.
- **Remote control, statistiques, mise à jour.**
  - Remote control « Claude Code seulement », icône teal quand une session est connectée.
  - Question des statistiques au premier démarrage.
  - Mise à jour prête dans la barre d'état ; son échec reste écrit dans À propos.
- **Après la 2.0.** La place des PR, de la CI et des déploiements : un onglet « PR et CI » du panneau git, et la ligne « PR → main » de la carte du ticket.

Puis les écrans du lot 0 bis (ESC-150, 2026-10-23), les fonctionnalités de la 1.8.0 à la 1.8.9, dans les deux thèmes ; leurs choix (14 à 38 de `UX-NOTES.md`) sont validés par l'utilisateur le 2026-10-23, à suivre par les lots :
- **Quatre harness.** Marques « Claude », « Codex », « OpenCode », « Copilot ». Menu « Harness » numéroté (`1` à `4` une fois ouvert par `Ctrl Maj N`), un harness sans compte à sa place avec « Ajouter un compte… », puis « Nouvel agent sans worktree » (ou dans un worktree) sous « Worktree ». Ce qu'un harness n'a pas reste à sa place, grisé avec sa raison (mode Auto d'OpenCode).
- **Réglages › Harness** : un groupe par harness avec sa version et la plage testée (« Version non testée » ambre, « Trop ancienne » rouge avec la commande à copier), « Mettre à jour Claude Code automatiquement » et son dernier résultat, les modèles favoris d'OpenCode ; plus de « Nouveaux agents » ni de « Se connecter… ».
- **Comptes.** Chaque compte a sa ligne « Nouveaux agents » (modèle, effort, mode) ; « Compte de la machine » marqué ; un harness sans compte garde son groupe et son bouton « Ajouter un compte <harness>… » ; « Ajouter un compte » propose le compte de la machine ; budget par jour d'OpenCode ; « Changer de compte à » (80, 90, 95 %, « À la limite ») et « Projection des quotas ».
- **Barre d'état et quotas.** Projection affichée quand elle passe 80 % ; « usage supplémentaire » en ambre dans le groupe du harness ; OpenCode « 0,42 $ / 5 $ » ; panneau avec toutes les projections, le budget d'OpenCode et les crédits de Copilot CLI.
- **Conversation.** Séparateur d'un changement de compte au seuil ; carte « Aucun compte <harness> » d'un agent qui ne démarre pas ; « Toujours autoriser » d'OpenCode avec son motif ; plafond de crédits et formulaire MCP de Copilot CLI ; long menu des modèles (recherche, favoris, groupes, ×N).
- **Barre latérale.** « Attend » ou « Réveil à 15:30 » avec un anneau vide et son infobulle ; agents pilotés repliés sous la carte ; PR sous la carte (numéro et glyphe d'état, bouton de correction) ; lancements dans le worktree d'un agent sous son nom, et le menu de ▶ ; archivés avec « worktree gardé ».
- **En-tête d'un agent.** La PR à côté de la branche ; « Ouvrir une PR » à la place de « Préparer le lancement » tant que la branche a des commits sans PR (le lancement passe dans « … »).
- **Vues terminal.** Vue OpenCode avec la bande verte « Escouade garde le fil » ; vue Copilot CLI avec la bande ambre de confiance du dossier.
- **Voix.** Questions une à une (« Question 1 sur 2 », choix numérotés) ; transcription en direct en plus pâle avec « <service> · en direct » ; Réglages › Voix en « Transcription » et « Conversation », chacun « Local | En ligne ».
- **Réglages.** Application : zoom en segments, tailles de texte, « Modèle des textes rédigés ». À propos : « Site web », « Discord ». Kanban : « Les agents partent de la branche cible » (devenu « Branche de départ des nouveaux worktrees » de l’onglet Projets, ESC-160 : voir le lot 11b), « Supprimer le worktree d'un ticket terminé ou supprimé ». Intégrations : « Hébergeurs de code » en tête. Projets : « Confidentialité » (projet privé, ses fichiers, ses comptes), « Worktrees restés » et « Nettoyer », « .escouade.yml » (marques, commandes à approuver, export), « Nouveaux projets privés ». Serveur MCP : « Les agents peuvent piloter Escouade », déclaré par harness.
- **Fenêtres.** « Ouvrir une PR sur GitHub » (titre et description proposés, branche cible, brouillon, fichiers non commités dits) ; « « Front » tourne déjà » ; l'assistant qui ne propose pas un projet privé et propose « Réglages de .escouade.yml » ; la bande du Kanban « Les commandes de .escouade.yml attendent ton accord ».

## Nouveautés (en TDD)

Ce qui suit ne se fait pas aujourd'hui :
- [ ] **Ce que fait l'agent, sur sa carte** de la barre latérale, par exemple `Edit src/auth/jwt.ts` et `184 k · 2,41 $`, ou bien le résultat (« 2 fichiers modifiés »). La donnée existe : c'est l'activité que montre déjà la vue d'ensemble.
- [ ] **Répondre à une question depuis la liste** : la question et ses options sur la carte de l'agent qui attend, cliquables ou au clavier (1, 2…) sans quitter la conversation ouverte. L'agent repart, puis reprend sa place dans la liste. Le comportement existant d'une autorisation lue en entier (Autoriser/Refuser depuis la vue d'ensemble) sert de modèle.

## Le chantier

**Ampleur** : 83 composants Svelte (22 100 lignes, dont 9 000 de styles), 35 fichiers de test qui citent ces caractères (flèches comprises), et les e2e Playwright. Estimation du 2026-10-12 en mode économe : 20 à 35 % d'un quota hebdomadaire Max. C'est une estimation grossière.

Organisation prévue :
- 11 lots implémentés par Sonnet, relus par paires ;
- une revue finale Opus ;
- la revue de fin d'Impeccable, puis son documenteur.

Lots, dans l'ordre :
0. **Mode clair dans l'aperçu** (ESC-129, fait le 2026-10-23) : le thème clair de chaque écran de `design/app-redesign/`, le sélecteur « Thème » (`#settings-app`), les captures `-light`, le contrôle `contrast.mjs` ; voir « Le thème clair ». Direction claire validée par l'utilisateur le 2026-10-23.
   - **Lot 0 bis** (ESC-150, 2026-10-23) : les fonctionnalités de la 1.8.0 à la 1.8.9 dans l'aperçu, en sombre et en clair (voir « Écran par écran » et la section « Lot 0 bis » de `UX-NOTES.md`, choix 14 à 38 validés le 2026-10-23). Les lignes « Lot 0 bis » ci-dessous disent ce que chaque lot en reprend.
1. **Fondations** : Inter (`@fontsource-variable/inter`), jetons de `src/app.css` dans les deux thèmes et le réglage « Thème », teint par projet et lumière dans `src/lib/theme.ts`, un composant d'icône, les contrôles communs (bouton, interrupteur, segments, champ, `kbd`), l'infobulle.
   - Nouveau : `EngineMark`, la marque du harness, partout.
   - Lot 0 bis : `EngineMark` pour quatre harness (« OpenCode », « Copilot ») ; `Star` (favori plein ou en contour) ; les marques d'origine d'un réglage (`SharedMark` : teal « Défini par .escouade.yml », neutre « Réglage local », ambre « en attente ») ; `PrivateMark` (cadenas) ; le menu à état grisé avec sa raison ; une infobulle riche (plusieurs lignes, icônes).
2. **Coque** : `App.svelte`, `TitleBar`, `StatusBar`, `QuotaMeter`, `AccountsPanel`, `Toasts`, `Welcome`.
   - Lot 0 : jauges par harness (`StatusBar`), panneau des comptes groupé par harness (`AccountsPanel`), coût estimé, mise à jour prête dans la barre d'état, `Welcome` sans harness.
   - Lot 0 bis : projection dans `QuotaMeter` (barre d'état au-delà de 80 %, toutes dans `AccountsPanel`), « usage supplémentaire », budget d'OpenCode et crédits de Copilot CLI ; `Welcome` « Aucun compte » ; `PrivateMark` sur l'onglet d'un projet privé (`TitleBar`) ; notifications du changement de compte et du worktree gardé (`Toasts`).
3. **Barre latérale** : `Sidebar`, `RunsSection`, terminaux, `StatusDot`, et les deux nouveautés.
   - Lot 0 : « Nouvel agent » et son menu « Harness », compteurs de sous-agents et de tâches de fond, liste dépliée de l'agent ouvert, icône du remote control, menu de l'agent, Lancement et Terminaux repliés en petite hauteur.
   - Lot 0 bis : menu « Harness » numéroté sur quatre harness, sans compte à sa place, et l'agent sans worktree ; « Attend » et « Réveil à … » avec l'infobulle, agents pilotés repliés ; PR sous la carte et leurs corrections (`Sidebar`) ; « Lancer… » et « Ouvrir une PR… » dans le menu de l'agent ; le menu de ▶ et les lancements dans un worktree (`RunsSection`) ; archivés avec « worktree gardé ».
   - ESC-160 (demande de Metalzoid) : « Nouvel agent depuis une autre branche… » dans le menu « Harness », sous « Worktree » avec « Nouvel agent sans worktree » ; dans le sélecteur de « Nouvel agent sur une branche… », la ligne « Créer « x » depuis une autre branche… » sous celle qui la crée tout de suite, puis le sélecteur en mode « Branche de départ (de « x ») » (`BranchPicker`, `mode="base"`).
4. **Conversation** : `Conversation`, `Composer`, `conv/*` (`TurnCard`, `ToolRow`, `PatchView`, `UserMessage`, `Thinking`, `Markdown`, `EventMessage`, `Notice`, `SetupProgress`, `CriteriaReport`, `PermissionCard`, `QuestionCard`).
   - Nouveaux : `conv/Handover`, `AttachmentChip`, `MicButton`, `ConversationButton`, `ConversationOverlay`, `SubagentView`, `TaskView`.
   - Lot 0 : en-tête (marque, bac à sable, bascule de vue, « … »), barre du champ réordonnée, « via Escouade » dans `PermissionCard` et `QuestionCard`, « Dit à voix haute » dans `UserMessage`, harness des tours dans `TurnCard`.
   - Lot 0 bis : en-tête avec la PR et « Ouvrir une PR » ; long menu des modèles du `Composer` (recherche, `Star`, groupes, ×N) et le mode grisé d'un harness ; « Toujours autoriser » et son motif (`PermissionCard`) ; formulaire MCP et plafond de crédits de Copilot CLI ; séparateur du changement de compte et carte « Aucun compte » (`TurnCard`, `EventMessage`) ; questions une à une et transcription en direct (`ConversationOverlay`, `MicButton`).
5. **Plan** : `PlanBanner`, dont les sous-agents mènent à leur vue.
6. **Kanban** : `board/*`, `ImportModal`.
   - Lot 0 : marque et trombone sur `TicketCard` ; champ « Harness » et pièces jointes en une rangée dans `TicketForm`.
   - Lot 0 bis : la bande « Les commandes de .escouade.yml attendent ton accord » du `Board`.
7. **Git** : `SidePanel`, `FilesPanel`, `FileDiff`, `DiffView`, `DiffModal`, `GitGraph`, `branches/*`, `CommitModal`.
   - Lot 0 : garder la place de l'onglet « PR et CI » (ESC-53 à 70, après la 2.0), sans le coder.
   - Lot 0 bis : « Ouvrir une PR… » dans `BranchPicker`, comme dans le menu de l'agent.
8. **Terminaux et lancements** : `TerminalView`, `RunView`, `AgentTerminal` (thèmes xterm clair et sombre), `TestLaunchModal`.
   - Lot 0 : la bande de confiance du dossier de Codex dans `AgentTerminal`.
   - Lot 0 bis : la même bande pour Copilot CLI, la bande verte « Escouade garde le fil » de la vue OpenCode (`AgentTerminal`) ; la fenêtre « « Front » tourne déjà » et le log d'un lancement dans un worktree (`RunView`).
9. **Éditeur et recherches** : `editor/*`, thèmes clair et sombre de CodeMirror (`src/lib/editor/theme.ts`), `QuickOpen`, `ConvSearch`.
10. **Vue d'ensemble et stats** : `Overview`, `Stats`, avec la marque du harness et les coûts estimés, graphiques lisibles dans les deux thèmes.
    - Lot 0 bis : les marques des quatre harness ; l'état « Attend » dans la vue d'ensemble.
11. **Fenêtres et réglages** : `Modal`, `ConfirmModal`, `UpdateModal`, `NewProjectModal`, `AccountModal`, `SettingsModal`, `settings/*`, `ContextMenu`, `Dropdown`, `Splitter`.
    - Nouveaux : `modals/TelemetryModal`, `wizard/*` (`AccountStep`, `Checks`, `CopiedStep`, `IntegrationsStep`, `KanbanStep`, `LaunchStep`, `SummaryStep`, `WorktreesStep`), `settings/ClosedProjects`, `settings/VoiceTab`, `settings/SpeechSettings`.
    - Lot 0 : Réglages › Harness en un groupe par harness ; « Quand un quota est épuisé » dans Comptes ; un onglet « Serveur MCP » (`McpSection`) ; Voix ; Projets (harness par défaut, assistant, projets fermés) ; À propos (statistiques, échec de mise à jour).
    - Lot 0 bis : `PrModal` ; Harness (version et plage testée, mise à jour de Claude Code, `ModelFavorites`) ; `AccountsTab` (« Nouveaux agents » par compte, « Aucun compte », budget, « Changer de compte à », projection) et `AccountModal` (compte de la machine) ; `VoiceTab`, `SpeechSettings`, `OnlineKey`, `OnlineModel`, `OnlineVoiceField` (« Local | En ligne », temps réel) ; `ApplicationTab` (zoom, « Modèle des textes rédigés ») ; `BoardTab` (branche cible, worktree d'un ticket fini) ; `ForgesSection` ; `ProjectTab` (« Confidentialité », ses fichiers, ses comptes, worktrees restés), `SharedSection` (.escouade.yml), `NewProjects` ; `McpSection` par harness ; À propos (site, Discord) ; `wizard/*` (point de départ : .escouade.yml, projet privé non proposé).
    - Ce lot a presque doublé : coupé en deux le 2026-10-23, 11a les fenêtres (ESC-83), puis 11b les réglages (ESC-152).
    - Pour 11b (ESC-152), ESC-160 (demande de Metalzoid) : « Branche de départ des nouveaux worktrees » dans le groupe Git de `ProjectTab`, sous « Créer le worktree au premier message » (segments « Branche actuelle du dossier | Branche cible du Kanban | ⎇ <branche> ▾ », ce que fait le choix dessous ; `WorktreeBaseField`), et la même rangée dans `WorktreesStep` (une liste des branches du dossier tant que le projet n’est pas ajouté) ; « Les agents partent de la branche cible » a quitté `BoardTab`.

**Méthode.**
- Impeccable, chemin code-led : le contrat de direction est déjà écrit et l'aperçu tient lieu de comp de référence. Relire le contrat et les captures avant chaque lot.
- Détecteur sur les fichiers changés. Revue de fin (`impeccable-finish-reviewer`) avec les captures de l'app réelle et celles de l'aperçu.
- Puis le documenteur, qui écrit le `DESIGN.md` de l'app et `.impeccable/design.json`.
- Qualité comme d'habitude : TDD pour les deux nouveautés, Vitest, `svelte-check`, build, e2e sur l'app réelle (`npx tauri build --debug --no-bundle`).
- Les captures de l'app réelle passent par le banc e2e (CDP), à 1440×900 et 1028×779, dans les deux thèmes à chaque lot.

## À décider ou à vérifier pendant le chantier

- [ ] **macOS** : l'aperçu dessine les boutons de fenêtre de Windows ; sous macOS, les feux restent à gauche, et la barre de titre doit leur laisser la place.
- [ ] **Anglais** : l'aperçu a un mode anglais (`?lang=en`) pour ce qui est serré, vérifié à 1028×779 pour le lot 0 (voir `UX-NOTES.md`). Le reste (Kanban, git, éditeur, vue d'ensemble, stats) reste à vérifier en anglais dans l'app.
- [ ] **Hauteur** : à 779px, les sections Lancement et Terminaux ne laissaient que deux ou trois agents visibles. Décidé avec le lot 0 (validé le 2026-10-17) : repliées sur leur titre à 820 px de haut et moins, un plantage toujours dit.
- [ ] **Signalements du détecteur** laissés sur l'aperçu, à rejuger dans l'app :
  - petits textes de métadonnées (11 à 11,5px) ;
  - points animés de l'activité ;
  - reflet en dégradé de « Claude travaille… » ;
  - cartes posées sur le panneau, vues comme imbriquées.
- [ ] **Ce qui montre l'app ailleurs**, à refaire une fois le design en place :
  - la vidéo, qui redessine l'app en React (`video/src/ui/*`, couleurs dans `video/src/theme.ts`) ;
  - les captures du site (`website/public/images/`, `en/` compris) ;
  - la démonstration du site (`SquadDemo`, `AgentCard`), qui garde aujourd'hui « les couleurs de l'app ».
- [ ] **Mettre à jour `PRODUCT.md`** : l'app suit désormais le site.
