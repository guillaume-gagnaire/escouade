---
title: "Les comptes partagés par plugin"
subject: "plugins"
sources: ["docs/PLUGINS.md §4.3, §4.4, §4.8","docs/TODO-2.2.md (INT-6)","src-tauri/src/plugins/accounts.rs","src-tauri/src/plugins/runtime.rs","src-tauri/src/integrations/jira.rs"]
tickets: ["ESC-212","ESC-327","ESC-360"]
decisions: [33]
compared: null
---

## Ce que c'est

Un compte (un GitLab, un Jira…) est d'un genre, que déclare le plugin qui le sert. Un compte sert tous les points de son plugin : une connexion, tous les usages. Les intégrations d'Escouade, encore dans le cœur, ont chacune leur genre, sous l'id du plugin de première main qui les servira (`dev.escouade.github`, `dev.escouade.gitlab`, `dev.escouade.azure-devops`, `dev.escouade.jira`…).

## Comportement

### Le genre d'un compte

- Il dit son instance publique, s'il a des instances à soi et comment en reconnaître une (une sonde), comment on se connecte (un jeton saisi, la CLI du service), les champs du formulaire (ceux qui vont au trousseau marqués secrets), les droits du jeton et la page qui en crée un.
- Un plugin le déclare dans `contributes.accounts` ; ses `ticketSources` et `forges` qui le nomment (`account`) sont les points que ses comptes servent.

### Une connexion, tous les usages

- github.com : GitHub Issues et ses PR. Une instance GitLab : GitLab Issues et ses MR. Une organisation Azure DevOps : Azure Boards et ses PR. Un GitHub Enterprise : ses PR seules.
- La ligne d'un hébergeur dit « Sert aussi à … » d'après les points de son compte.

### Le plugin qui sert un compte

- Quand plusieurs plugins actifs servent le même genre (un GitLab d'entreprise à côté de celui d'Escouade), « Connecter… » montre « Servi par », Escouade d'abord ; un seul : rien à choisir. Jamais de remplacement en silence.
- Le choix est gardé avec le compte (`plugin` dans `integrations.json`, absent pour Escouade). Le compte se dit « Servi par le plugin <id> ».
- Un compte servi par un autre plugin passe par lui : son jeton va au trousseau avant la vérification (le plugin le lit lui-même) et en repart si le plugin le refuse. GitLab Issues et Azure Boards suivent le plugin du compte de leur hébergeur (une instance par plugin pour l'instant).
- Un plugin qui ne sert pas ce genre est refusé : « Le plugin <id> ne sert pas les comptes <genre>. ».

### Le trousseau

- Le secret d'un compte est sous `plugin:<id>:account:<compte>` : le compte est l'hôte de son instance (`gitlab.com`), ou l'id du genre pour un genre à un seul compte (`jira`, `trello`, `linear`, `clickup`). Il tient ce que tenait l'ancienne entrée : le jeton, ou `{key, token}` pour Trello.
- Un plugin le lit par `secrets/get` de `account:<compte>`, sous son propre id seulement (il lui faut `account:*` dans `permissions.secrets`) ; jamais le compte d'un autre plugin.
- **Migration réversible** : pour les comptes servis par Escouade, les anciennes entrées (`jira`, `github`, `forge:<hôte>`) restent tenues à côté, écrites et effacées avec les nouvelles, et lues d'abord tant qu'elles sont tenues ; un jeton qu'une version d'avant a changé après un retour en arrière est repris et recopié. Une version d'avant relit ses comptes sans rien faire. `integrations.json` garde sa forme et dit `vault: 2` à sa prochaine écriture, qu'une version d'avant ignore. « Déconnecter » efface tous les noms du compte.

### La limite d'appels

- Elle est comptée par compte : la dernière réponse d'un point du plugin (une source ou un hébergeur) dit ce qui reste au compte, et tous ses points la lisent (le budget de GitHub est celui du jeton, Issues et PR ensemble).

## Limites

- Un plugin sert sa source sur une seule instance de son hébergeur tant que les sources ne sont pas sorties dans les plugins (INT-8).
- Les comptes servis par un autre plugin n'ont pas d'ancienne entrée : une version d'avant les voit « Trousseau du système illisible ».

## Le compte Mistral

- Le plugin « Mistral » déclare le genre `mistral`, dont les points sont un harness et une voix (`harnesses:vibe`, `voice:mistral`) : un compte de plugin ne sert pas que des sources et des hébergeurs. Le genre le dit dans `voice`, le harness par son `account`.
- Une exception assumée à la lecture par `secrets/get` : le plugin n'a aucun droit sur le trousseau. La clé part avec chaque appel de la voix, et à Vibe par `MISTRAL_API_KEY`.
- Le compte `mistral` et les comptes `vibe-<id>` ont leurs anciens noms (`dictation-mistral`, `vibe:<id>`), tenus et lus d'abord, comme ci-dessus. Le détail est dans *Mistral Vibe en plugin*.

## Ce que le cœur prête à un plugin, et quand il le reprend

- **Pendant un essai de connexion**, le cœur prête au plugin l'hôte saisi et le jeton, le temps de cet essai seul. Réussi ou raté, le prêt finit avec lui : un hôte mal tapé n'est plus joignable, ni un jeton refusé lisible. Un essai réussi laisse son jeton prêté au compte enregistré.
- Deux essais en même temps sur un plugin ont chacun leur prêt : aucun ne prend l'hôte de l'autre, et chacun reprend le sien seulement.
- Un essai raté sur un compte qui tourne (un jeton retapé) lui laisse son jeton. Pendant l'essai, un appel de ce compte lit le jeton essayé : `secrets/get` ne dit pas qui demande.
- **Un compte enregistré** n'a pas besoin d'hôte prêté : `{account}` le lit dans les comptes à chaque appel. Son jeton lui reste prêté (celui de sa CLI n'est nulle part ailleurs), et reprêté à chaque client.
- **Au retrait du compte**, son jeton est repris, pour tout plugin, sans toucher aux prêts d'un autre compte. Un jeton lu pendant le retrait n'est plus prêté. Un compte repris par un autre plugin perd de même le prêt du premier.
- **L'adresse d'un compte** (Jira, hébergeur de code) est lue par un parseur d'URL : c'est l'hôte réel qui décide de « https, sauf cette machine ». `http://localhost:1@exemple.dev` vise `exemple.dev` : refusée dès le formulaire. `localhost`, `127.0.0.1` et `[::1]` passent en http, à toute casse et avec un port ou un utilisateur ; `localhost.exemple.dev` non.
