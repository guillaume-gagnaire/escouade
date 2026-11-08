# Dépannage

Les messages ci-dessous sont ceux qu’Escouade et le plugin affichent, mot pour mot ; `{…}` est ce qui change d’une fois à l’autre.

## L’agent ne démarre pas

**« Codex introuvable. Installe-le ou indique son chemin dans les réglages. »**
Escouade n’a pas trouvé `codex`. Installe-le (`npm install -g @openai/codex`), ou donne le chemin de son exécutable dans Réglages › Comptes et harness.

**« Aucun compte Codex : ajoute-en un dans Réglages › Comptes et harness. »**
Ajoute le compte de la machine ou un nouveau compte.

**« Le plugin Codex (dev.escouade.codex) est coupé ou n’est pas chargé : rallume-le dans la vue Plugins pour lancer un agent de Codex. »**
Le plugin est désactivé, ou n’a pas pu se charger. Rallume-le ; sa page et son journal disent pourquoi. L’agent garde sa session.

**« Le plugin Codex (dev.escouade.codex) n’est plus installé : la vue Plugins propose de l’installer, pour lancer un agent de Codex. »**
Installe-le de nouveau depuis la vue Plugins ; les conversations des agents sont gardées.

**`initialize: {raison}`** ou **`thread/start: {raison}`** (ou `thread/resume`, `thread/fork`)
Codex a refusé d’ouvrir la conversation, avec sa raison. Le plus souvent, le compte n’est pas connecté : « Se connecter… » sur sa fiche, dans Réglages › Comptes et harness. Sinon, mets Codex à jour.

**« Le plugin de Codex n’applique pas la clôture des projets privés : l’agent ne démarre pas. »**, **« Le plugin de Codex ne tient pas son harness en lecture seule : la discussion ne démarre pas. »**, **« Le plugin de Codex met le jeton d’Escouade dans les arguments de son harness, que tout le monde peut lire : l’agent ne démarre pas. »**
Escouade a refusé le lancement que le plugin a décrit. Le plugin officiel ne fait pas cela : mets-le à jour, et signale-le si le message reste.

## Pendant un tour

**« La conversation Codex n’a pas pu être reprise : nouvelle conversation (contexte vidé) »**
Codex n’a pas retrouvé la conversation de l’agent (effacée, ou d’un autre compte). L’agent continue, sans le contexte d’avant.

**« Codex a signalé une erreur »**
Le tour a fini en erreur sans que Codex dise laquelle. Réessaie ; si cela revient, ouvre la vue Codex pour voir ce qu’il dit.

**« Codex n’a pas compacté le contexte : {raison} »**
`/compact` a échoué. Réessaie plus tard ; la conversation continue telle quelle.

**« Compaction du contexte… »** puis **« Contexte compacté »**
Codex résume la conversation pour faire de la place. Rien à faire.

**« Relecteur automatique de Codex : {message} »**
En Auto, le relecteur de Codex a décidé d’une demande à ta place ; la conversation le note.

**Une carte qui dit « git écrit dans le dépôt du projet, hors du bac à sable de Codex. »**
La commande écrit dans `.git` et n’est pas une forme simple de `git add` ou `git commit` que le plugin accepte seul : regarde-la avant d’autoriser.

**Une carte qui dit « Dans {dossier} »**
La commande tourne hors du dossier de l’agent.

**Une carte « Codex demande l’accès au réseau », « … l’écriture dans {chemins} », « … la lecture de {chemins} »**
Codex veut sortir de son bac à sable. « Pour cette session » le lui laisse jusqu’à la fin de sa session.

## Le plugin

**« Le plugin de Codex a redémarré. »**
Escouade l’a relancé. Codex a continué, et ce qu’il avait demandé avant reçoit encore sa réponse.

**« Le plugin de Codex s’est arrêté après {n} plantages : l’agent s’arrête, sa session est gardée. Relance le plugin dans Réglages › Plugins, puis envoie un message. »**
Après trois plantages en cinq minutes, Escouade ne le relance plus. Relance-le, puis envoie un message. Son journal dit ce qui s’est passé.

**« Le plugin de Codex est arrêté ({raison}) : l’agent s’arrête, sa session est gardée. »**, **« Le plugin de Codex ne répond pas : {raison} »**, **« Le plugin de Codex n’a pas traduit ce que son harness a dit ({raison}). »**
Rallume le plugin et envoie un message. Si cela revient, signale-le avec la version de Codex.
