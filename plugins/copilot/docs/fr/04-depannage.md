# Dépannage

Les messages ci-dessous sont ceux qu’Escouade et le plugin affichent, mot pour mot ; `{…}` est ce qui change d’une fois à l’autre.

## L’agent ne démarre pas

**« Copilot CLI introuvable. Installe-le ou indique son chemin dans les réglages. »**
Escouade n’a pas trouvé `copilot`. Installe-le (`npm install -g @github/copilot`), ou donne le chemin de son exécutable dans Réglages › Comptes et harness.

**« Aucun compte Copilot CLI : ajoute-en un dans Réglages › Comptes et harness. »**
Ajoute le compte de la machine ou un nouveau compte.

**« Copilot CLI n’est connecté à aucun compte GitHub sur ce compte : connecte-le dans Réglages › Comptes et harness (« Se connecter… », copilot login). »**
Le compte n’a pas de connexion à lui. Un compte autre que celui de la machine ne compte pas la connexion du GitHub CLI (`gh`) : connecte-le par « Se connecter… ».

**« Copilot CLI parle le protocole {version}, Escouade le protocole {version} : mets Copilot CLI à jour (npm install -g @github/copilot@latest). »**
Le Copilot de la machine est trop ancien, ou trop récent, pour le protocole que parle le plugin. Mets-le à jour ; si c’est Copilot qui est plus récent, mets le plugin à jour.

**« Copilot CLI n’a pas ouvert la session de l’agent : {raison} »**
Copilot a refusé la session, avec sa raison. Lis-la : un modèle que le compte n’a pas, une connexion expirée…

**« Le plugin Copilot CLI (dev.escouade.copilot-cli) est coupé ou n’est pas chargé : rallume-le dans la vue Plugins pour lancer un agent de Copilot CLI. »**
Le plugin est désactivé, ou n’a pas pu se charger. Rallume-le ; sa page et son journal disent pourquoi. L’agent garde sa session.

**« Le plugin Copilot CLI (dev.escouade.copilot-cli) n’est plus installé : la vue Plugins propose de l’installer, pour lancer un agent de Copilot CLI. »**
Installe-le de nouveau depuis la vue Plugins ; les conversations des agents sont gardées.

**« Le plugin de Copilot CLI n’applique pas la clôture des projets privés : l’agent ne démarre pas. »**, **« … ne tient pas son harness en lecture seule : la discussion ne démarre pas. »**, **« … met le jeton d’Escouade dans les arguments de son harness, que tout le monde peut lire : l’agent ne démarre pas. »**
Escouade a refusé le lancement que le plugin a décrit. Le plugin officiel ne fait pas cela : mets-le à jour, et signale-le si le message reste.

## Pendant un tour

**« Copilot utilise le compte du GitHub CLI (gh), faute de connexion à lui : connecte-toi avec copilot login pour choisir son compte. »**
Le compte de la machine n’a pas de connexion à Copilot, et Copilot a pris celle du GitHub CLI. L’agent tourne, mais sur ce compte-là. Connecte Copilot par `copilot login` pour choisir.

**« La session Copilot n’a pas pu être reprise : nouvelle session (contexte vidé) »**
Copilot n’a pas retrouvé la session de l’agent. L’agent continue, sans le contexte d’avant.

**« Contexte compacté »**
Copilot a résumé la conversation pour faire de la place. Rien à faire.

**« Le plafond de crédits d’IA de la session est atteint ({utilisés} sur {plafond}). Que faire de l’appel en attente ? »**
La session a un plafond de crédits et l’a atteint. Choisis d’en ajouter autant, de lever le plafond ou d’arrêter là.

**Une carte qui dit « Hors du dossier de l’agent », « Chemins : {chemins} », « Adresses : {adresses} » ou « Écrit dans un fichier par une redirection »**
Ce que la demande touche : des chemins hors du dossier, des adresses web, un fichier écrit par `>`.

**Une carte qui dit « git écrit dans le dépôt du projet. »**
La commande écrit dans `.git` et n’est pas une forme simple de `git add` ou `git commit` que le plugin accepte seul : regarde-la avant d’autoriser.

**Une carte qui dit « Serveur MCP {serveur}, outil {outil} : ne fait que lire » ou « … : peut modifier »**
Un outil d’un serveur MCP, et ce que son serveur dit qu’il fait.

**Copilot dit « Refusé par Escouade : ce chemin est celui d’un autre projet, et l’un des deux est privé. »**
Le plugin a refusé une demande sur un chemin clôturé (voir l’onglet Permissions). Rien à faire : c’est la clôture.

## Le plugin

**« Le plugin de Copilot CLI a redémarré. »**
Escouade l’a relancé. Copilot a continué ; ce qui était en cours de traduction peut manquer jusqu’à la fin du tour.

**« Le plugin de Copilot CLI s’est arrêté après {n} plantages : l’agent s’arrête, sa session est gardée. Relance le plugin dans Réglages › Plugins, puis envoie un message. »**
Après trois plantages en cinq minutes, Escouade ne le relance plus. Relance-le, puis envoie un message. Son journal dit ce qui s’est passé.

**« Le plugin de Copilot CLI est arrêté ({raison}) : l’agent s’arrête, sa session est gardée. »**, **« Le plugin de Copilot CLI ne répond pas : {raison} »**, **« Le plugin de Copilot CLI n’a pas traduit ce que son harness a dit ({raison}). »**
Rallume le plugin et envoie un message. Si cela revient, signale-le avec la version de Copilot CLI.
