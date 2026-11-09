# Dépannage

Les messages ci-dessous sont ceux qu’Escouade et le plugin affichent, mot pour mot ; `{…}` est ce qui change d’une fois à l’autre.

## L’agent ne démarre pas

**« OpenCode introuvable. Installe-le ou indique son chemin dans les réglages. »**
Escouade n’a pas trouvé `opencode`. Installe-le (`npm install -g opencode-ai`), ou donne le chemin de son exécutable dans Réglages › Comptes et harness.

**« Non prise en charge », dans Réglages › Comptes et harness**
C’est OpenCode 2 (`@opencode/cli`). Installe OpenCode 1 : `npm install -g opencode-ai`.

**« Aucun compte OpenCode : ajoute-en un dans Réglages › Comptes et harness. »**
Ajoute le compte de la machine.

**« Aucun modèle d’OpenCode à utiliser : connecte un fournisseur à OpenCode (opencode auth login) ou déclare-en un dans sa config, puis choisis son modèle. »**
OpenCode n’a aucun fournisseur connecté, ou le modèle choisi n’en est pas. `opencode auth login` (ou « Se connecter… » dans Réglages › Comptes et harness), puis choisis un modèle dans le menu.

**« Le flux d’événements d’OpenCode ne répond pas »**
Le serveur d’OpenCode n’a pas ouvert son flux d’événements à temps. Envoie un message pour réessayer ; si cela revient, arrête les agents OpenCode pour que le serveur reparte.

**« Le plugin OpenCode (dev.escouade.opencode) est coupé ou n’est pas chargé : rallume-le dans la vue Plugins pour lancer un agent de OpenCode. »**
Le plugin est désactivé, ou n’a pas pu se charger. Rallume-le ; sa page et son journal disent pourquoi. L’agent garde sa session.

**« Le plugin OpenCode (dev.escouade.opencode) n’est plus installé : la vue Plugins propose de l’installer, pour lancer un agent de OpenCode. »**
Installe-le de nouveau depuis la vue Plugins ; les conversations des agents sont gardées.

**« Le plugin de OpenCode n’applique pas la clôture des projets privés : l’agent ne démarre pas. »**, **« … ne tient pas son harness en lecture seule : la discussion ne démarre pas. »**, **« … met le jeton d’Escouade dans les arguments de son harness, que tout le monde peut lire : l’agent ne démarre pas. »**
Escouade a refusé le lancement que le plugin a décrit. Le plugin officiel ne fait pas cela : mets-le à jour, et signale-le si le message reste.

## Pendant un tour

**« La session OpenCode n’a pas pu être reprise : nouvelle session (contexte vidé) »**
OpenCode n’a pas retrouvé la session de l’agent. L’agent continue, sans le contexte d’avant.

**« Compaction du contexte… »** puis **« Contexte compacté »**
OpenCode résume la conversation pour faire de la place (`/compact`, ou de lui-même). Rien à faire.

**Une carte qui dit « Hors du dossier de l’agent : {chemins} »**
La demande touche des chemins hors du dossier de l’agent.

**Une carte qui dit « « Toujours autoriser » retiendra : {motifs} »**
Ce qu’OpenCode gardera pour le reste de la session si tu choisis « Toujours autoriser » : regarde le motif, il peut être plus large que la demande (`git add *`).

**Un `git commit` ou une modification de `.git` demande en Édits auto**
C’est voulu : seules les formes simples de `git add` et `git commit` passent, et une modification de `.git`, `.opencode`, `opencode.json`, `.husky` ou `.githooks` a toujours sa carte (voir l’onglet Permissions).

## Le plugin

**« Le plugin de OpenCode a redémarré. »**
Escouade l’a relancé. Le serveur et le tour ont continué, et la session de l’agent est de nouveau écoutée.

**« Le plugin de OpenCode s’est arrêté après {n} plantages : l’agent s’arrête, sa session est gardée. Relance le plugin dans Réglages › Plugins, puis envoie un message. »**
Après trois plantages en cinq minutes, Escouade ne le relance plus. Relance-le, puis envoie un message. Son journal dit ce qui s’est passé.

**« Le plugin de OpenCode est arrêté ({raison}) : l’agent s’arrête, sa session est gardée. »**, **« Le plugin de OpenCode ne répond pas : {raison} »**
Rallume le plugin et envoie un message. Si cela revient, signale-le avec la version d’OpenCode.
