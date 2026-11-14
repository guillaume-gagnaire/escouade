# Escouade 2.3 — à faire

La 2.2.0 réunit la 2.2 et ce que main avait déjà pour la suite (ESC-442) :
- les hébergeurs de code et le flux des PR des tickets ;
- les plugins tous en WASM et leur store ;
- la refonte des réglages ;
- le découpage du cœur en crates ;
- le quota de Claude Code lu par son plugin ;
- les correctifs de l'audit de sécurité.

La 2.3 annonce trois chantiers, choisis par l'utilisateur le 2026-11-14 pour la feuille de route du site.

## L'Overseer et les tâches programmées

Conçus le 2026-10-28 : `docs/OVERSEER.md`, maquettes dans `design/overseer/`.

- [ ] L'Overseer : un agent qui suit tous les projets à la place de l'utilisateur, à la voix ou au clavier. Il lance, relit, fusionne et rend compte : ce que fait aujourd'hui une session « chef d'escouade » par le serveur MCP, mais dans l'app.
- [ ] Les tâches programmées : des tickets d'agent lancés comme un cron, projet par projet, que l'Overseer voit et pilote.
- [ ] À découper en tickets.

## Escouade à distance : le téléphone et le mode serveur

Étudiés dans `docs/REMOTE.md`, avec quatre décisions de l'utilisateur à prendre.

- [ ] La fondation commune : le cœur et l'interface sur deux machines, au lieu de l'IPC de Tauri dans un seul process.
- [ ] Le mode serveur : Escouade sur un serveur, qui tourne jour et nuit, piloté par le client du poste.
- [ ] Le téléphone : voir les agents qui attendent, répondre, envoyer un message, regarder le Kanban, ouvrir le serveur de dev d'un agent.
- [ ] L'app mobile (iOS, Android), avec WireGuard intégré et des notifications push par un relais chiffré de bout en bout. Sa technique se choisit par l'étude ESC-445 (`docs/MOBILE-STUDY.md`, 2026-11-14) : elle recommande Capacitor, sur l'interface Svelte d'Escouade, et écarte Tauri 2 pour le téléphone. Décision de l'utilisateur à prendre.
- [ ] Plusieurs serveurs pilotés par un même Escouade, par exemple le travail et le perso : des espaces cloisonnés, comme dans Slack, avec les notifications de tous. Les règles d'une entreprise ne valent que pour son espace, sauf sur un poste qu'elle gère. Voir `docs/REMOTE.md`, « Plusieurs serveurs pilotés par un seul Escouade ».
- [ ] Les agents sur le serveur en auto-hébergé : ressources par agent, connexion des harness sans navigateur, la machine comme appareil de l'équipe, l'isolement quand plusieurs personnes partagent un serveur.

## Un serveur pour l'équipe

Demandé le 2026-11-14, à concevoir. Le Kanban partagé, la spec, les décisions, les documents et la mémoire partagée d'un projet tenus sur un serveur de l'équipe, au lieu de rester sur chaque poste. Le même serveur porte aussi un store de plugins privés : les plugins internes de l'équipe, signés par elle, pour elle seule. Aujourd'hui, ils vivent dans le dépôt (`.escouade/`, un fichier par ticket partagé depuis ESC-190) ou dans le dossier de données de chaque poste. Ce chantier peut partir de la fondation du mode serveur.

- [ ] La conception, avec l'utilisateur : ce qui part sur le serveur et ce qui reste local, les comptes et les droits, le hors-ligne, la migration depuis `.escouade/`, le Kanban partagé en direct (deux agents ne prennent jamais le même ticket).
- [ ] La liste des projets de l'équipe, chacun clonable d'un clic, avec ses réglages tenus au centre : un `.escouade.yml` d'équipe, par couches (l'équipe, puis le dépôt, puis le poste). Un réglage imposé par l'équipe ne se change pas sur le poste, comme une flotte gérée par l'entreprise (à la façon d'Intune ou des stratégies de groupe) : l'utilisateur le voit, grisé, avec qui l'impose. Exemples : la stratégie de branches, le mode de permission (jamais Bypass), les harness et les modèles permis, la clôture des projets privés, le proxy et son certificat. Ce que l'équipe impose est signé par ses administrateurs et montré aux membres. Rien d'exécutable (hooks, commandes) n'y entre sans être relu : une configuration centrale piratée ne doit pas pouvoir lancer un programme sur tous les postes.
- [ ] Le coupe-circuit de l'administrateur : retirer d'un coup, pour toute l'équipe, un plugin, un harness ou un modèle, ou arrêter tous les agents.
- [ ] Le tableau des coûts et des quotas de l'équipe, par membre, par projet et par ticket, avec budgets et alertes ; calculé sur les postes, seuls les totaux remontent.
- [ ] Passer un agent à un collègue : sa conversation, sa branche et son ticket.
- [ ] L'Overseer d'équipe, sur une instance en mode serveur, qui suit tous les projets de l'équipe (à concevoir avec l'Overseer).
- [ ] À étudier : le journal d'audit exporté vers le SIEM de l'entreprise (syslog, webhook).
- [ ] Le store de plugins privés : la clé de signature de l'équipe, bornée à son espace de noms (jamais `dev.escouade.*`) ; les contrôles du store public ; ce que l'administrateur autorise ses membres à installer.

## Les PR dans Escouade

Demandé le 2026-11-14 : le flux des PR de la 2.2 marche, mais son expérience n'est pas à la hauteur. À concevoir avec l'utilisateur, avec un aperçu de chaque direction.

- [ ] Voir la PR dans Escouade, comme sur l'hébergeur : sa description, ses fichiers et leur diff, ses commits, ses contrôles, sa discussion.
- [ ] Un agent relecteur qui commente comme un humain : ses remarques à côté du code, ligne par ligne, en fils de discussion. L'utilisateur y répond, les résout, ou demande à l'agent de corriger.
- [ ] Un meilleur contrôle de la PR depuis Escouade : approuver ou demander des changements, passer de brouillon à prête, relecteurs et étiquettes, mise à jour de la branche, conflits, relance de la CI, choix du merge.

## L'éditeur

Demandé le 2026-11-14 : l'éditeur (CodeMirror 6, avec son arborescence et ses onglets à nous) est agréable, mais il n'a pas tout. Il n'a ni serveur de langage, ni extensions. Peut-on y mettre l'éditeur de VS Code, avec son arborescence, son code et ses onglets ?

- [x] L'étude est faite (ESC-444, 2026-11-14, `docs/EDITOR-STUDY.md`, maquette sur la branche `study/esc-444`) : elle recommande la vue VS Code par monaco-vscode-api, d'abord sans Node (6 à 7 semaines d'agent), puis l'hôte Node par un serveur VSCodium téléchargé à la demande. Décision de l'utilisateur à prendre.
- [ ] (Pour mémoire, ce qu'elle visait) Une étude, avec une maquette qui tourne. La piste : `@codingame/monaco-vscode-api` (MIT), qui fait tourner les services du plan de travail de VS Code dans une page web, sans Electron : l'éditeur, les groupes d'onglets, l'explorateur, la recherche, les thèmes, les raccourcis et les extensions web. Il faut y brancher :
  - les fichiers, par le cœur Rust ;
  - les serveurs de langage, lancés par le cœur et reliés par `monaco-languageclient` ;
  - le thème d'Escouade.
- [ ] À mesurer :
  - le poids du bundle et la mémoire ;
  - le démarrage ;
  - la tenue sous WebKitGTK (Linux) et WKWebView (macOS), car VS Code pour le web vise surtout Chromium ;
  - les extensions qui marchent sans Node.

  Les extensions viendraient d'Open VSX : le marketplace de Microsoft et ses extensions fermées (Pylance, C# Dev Kit…) sont réservés à son VS Code.

  Les extensions qui demandent Node utiliseraient le Node du poste quand il y en a un, Node restant facultatif pour Escouade. Le cœur lancerait l'hôte d'extensions de VS Code (son serveur, comme OpenVSCode Server) en boucle locale, avec un jeton. La version de Node et ses modules natifs sont à vérifier, et il faut décider si on télécharge à la demande le Node exact que veut ce serveur.
- [ ] L'éditeur dans une colonne, comme les agents et les terminaux, à côté d'eux, quelle que soit la voie retenue. Avec la vue VS Code, il faut vérifier si plusieurs colonnes d'éditeur peuvent coexister.
- [ ] Les autres voies, à comparer :
  - VS Code entier (code-server ou OpenVSCode Server) dans une vue. Il a tout, mais il faut Node, un serveur local à protéger, et il ressemble à une autre app dans Escouade.
  - Garder CodeMirror et lui ajouter les serveurs de langage. C'est plus léger et cohérent avec le design, mais tout le reste est à faire nous-mêmes.

## Reporté de la 2.2

- [ ] Les fonctionnalités à activer et les réglages proposés au bon moment (`docs/FEATURES.md`). La température du cache est déjà sortie.
- [ ] Gemini CLI : ce qui reste à vérifier avec un compte (`docs/TODO-2.2.md`, chapitre « Gemini CLI »).
- [ ] Les restes du redesign (`docs/TODO-2.2.md`, chapitre « Restes du redesign »), dont la vidéo qui montre encore l'app de la 1.8 (ESC-85).
- [ ] Les essais réels de Gitea, de Forgejo et d'Azure DevOps (`docs/TODO-2.2.md`, chapitre « Hébergeurs de code »).
- [ ] `ort` à monter à la main sur les deux cibles ensemble (la PR Dependabot #36 ne montait que l'une), après vérification de la bibliothèque de Microsoft pour Linux.
- [ ] Les constats faibles encore ouverts de l'audit de sécurité (SEC-12 à SEC-23, SEC-25, SEC-26), à trier.
