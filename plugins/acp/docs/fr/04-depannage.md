# Dépannage

Les messages ci-dessous sont ceux qu’Escouade et le plugin affichent, mot pour mot ; `{harness}` est le nom que le manifeste donne au harness, et `{…}` ce qui change d’une fois à l’autre.

## L’agent ne démarre pas

**« {harness} lit et écrit les fichiers lui-même, sans les faire passer par Escouade (son manifeste le dit) : la clôture des projets privés ne tiendrait pas, l’agent ne démarre pas dans ce projet privé. »**
Le manifeste dit `"fs": false`, et l’agent a une clôture : un de tes projets est privé. Le message parle de « ce projet privé », mais il vaut pour tout projet tant qu’un projet est privé. Pour un auteur : si l’agent sait passer ses fichiers par le client, déclare `"fs": true`.

**« {harness} n’a pas de mode qui ne change rien (son manifeste n’en dit aucun) : il ne tient pas une discussion en lecture seule. »**
Une discussion de l’onglet Docs a été lancée sur un harness sans `readOnlyMode`. Choisis un autre harness pour la discussion ; pour un auteur, déclare ce mode.

**« {harness} parle ACP {version}, Escouade ACP {version} : mets {harness} à jour. »**
L’agent parle une autre version d’ACP que le plugin. Mets-le à jour.

**« {harness} n’a pas ouvert la session de l’agent : {raison}. S’il demande une connexion, connecte-le dans un terminal. »**
L’agent a refusé la session. Lance-le dans un terminal pour voir ce qu’il demande (une connexion, un fournisseur).

**« Le plugin de {harness} veut poser la variable {variable}, qu’Escouade refuse à un plugin parce qu’elle changerait ce que lance le harness ou toucherait à Escouade : l’agent ne démarre pas. »**
Le manifeste d’un plugin tiers pose dans `acp.env` une variable interdite (`PATH`, `NODE_OPTIONS`, `LD_*`, `DYLD_*`, `ESCOUADE_*`…). L’auteur doit la retirer.

**« {harness} introuvable. Installe-le ou indique son chemin dans les réglages. »**
Escouade n’a pas trouvé le programme dans le `PATH`, sous les noms de `locate.names`.

**« Aucun compte {harness} : ajoute-en un dans Réglages › Comptes et harness. »**
Ajoute un compte du harness.

**L’agent est marqué « harness absent »**
Le plugin ACP, ou le plugin qui décrit le harness, est coupé, désinstallé ou a changé. Rallume-les dans la vue Plugins : l’agent revient avec sa conversation.

## Pendant un tour

**« {harness} a lu ou modifié {chemin} sans le faire passer par Escouade : la clôture des projets privés ne tient pas pour ces accès, que seules ses permissions protègent. »**
L’agent a touché un fichier lui-même alors qu’une clôture tient. Le message vient une fois par agent. Ce n’est pas une erreur du plugin : c’est la limite de ce harness.

**« La session {harness} n’a pas pu être reprise : nouvelle session (contexte vidé) »**
L’agent n’a pas retrouvé sa session. Il continue, sans le contexte d’avant.

**« {harness} : {erreur} »**
L’agent a fini le tour en erreur, ou refusé un réglage (`session/set_mode: …`, le modèle), avec ses propres mots.

**L’agent dit « Refused by Escouade: … »**
Le client a refusé un fichier qu’il demandait : « is in a private project, out of this agent's reach » (un projet clôturé), « is not an absolute path » (ACP veut un chemin absolu), « is out of this agent's folders » (hors du dossier de l’agent et de son dépôt), « this agent is read-only » (une discussion). Rien à faire, sauf pour un auteur dont l’agent envoie des chemins relatifs.

**Une carte qui dit « Hors du dossier de l’agent » ou « Serveur MCP {serveur}, outil {outil} »**
La demande touche un chemin hors du dossier de l’agent, ou c’est un outil d’un serveur MCP.

## Le plugin

**« Le plugin de {harness} a redémarré. »**
Escouade l’a relancé. L’agent garde sa session, son tour et les demandes qui attendent.

**« Le plugin de {harness} s’est arrêté après {n} plantages : l’agent s’arrête, sa session est gardée. Relance le plugin dans Réglages › Plugins, puis envoie un message. »**
Après trois plantages en cinq minutes, Escouade ne le relance plus. Relance le plugin ACP, puis envoie un message. Son journal dit ce qui s’est passé.

**« Le plugin de {harness} est arrêté ({raison}) : l’agent s’arrête, sa session est gardée. »**, **« Le plugin de {harness} ne répond pas : {raison} »**, **« Le plugin de {harness} n’a pas traduit ce que son harness a dit ({raison}). »**
Rallume le plugin ACP et envoie un message. Si cela revient, signale-le avec le harness et sa version.
