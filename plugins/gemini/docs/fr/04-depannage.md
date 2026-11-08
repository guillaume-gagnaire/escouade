# Dépannage

Les messages ci-dessous sont ceux qu’Escouade et le plugin affichent, mot pour mot ; `{…}` est ce qui change d’une fois à l’autre.

## L’agent ne démarre pas

**« Gemini CLI introuvable. Installe-le ou indique son chemin dans les réglages. »**
Escouade n’a pas trouvé `gemini`. Installe-le (`npm install -g @google/gemini-cli`), ou donne le chemin de son exécutable dans Réglages › Comptes et harness.

**« Aucun compte Gemini CLI : ajoute-en un dans Réglages › Comptes et harness. »**
Ajoute un compte Google ou une clé d’API.

**« Gemini CLI n’a pas ouvert la session de l’agent : {raison}. S’il n’est pas connecté, lance gemini dans un terminal pour te connecter à ton compte Google, ou donne-lui une clé d’API par GEMINI_API_KEY. »**
Gemini a refusé la session. Pour un compte Google, « Se reconnecter… » sur sa fiche refait la connexion ; pour une clé, « Changer la clé… ».

**« Gemini CLI parle ACP {version}, Escouade ACP {version} : mets Gemini CLI à jour (npm install -g @google/gemini-cli). »**
Le Gemini de la machine parle une autre version d’ACP que le plugin. Mets-le à jour.

**« Le plugin Gemini CLI (dev.escouade.gemini-cli) est coupé ou n’est pas chargé : rallume-le dans la vue Plugins pour lancer un agent de Gemini CLI. »**
Le plugin est désactivé, ou n’a pas pu se charger. Rallume-le ; sa page et son journal disent pourquoi. L’agent garde sa session.

**« Le plugin Gemini CLI (dev.escouade.gemini-cli) n’est plus installé : la vue Plugins propose de l’installer, pour lancer un agent de Gemini CLI. »**
Installe-le de nouveau depuis la vue Plugins ; les conversations des agents sont gardées.

**« Le plugin de Gemini CLI n’applique pas la clôture des projets privés : l’agent ne démarre pas. »**, **« … ne tient pas son harness en lecture seule : la discussion ne démarre pas. »**, **« … met le jeton d’Escouade dans les arguments de son harness, que tout le monde peut lire : l’agent ne démarre pas. »**
Escouade a refusé le lancement que le plugin a décrit. Le plugin officiel ne fait pas cela : mets-le à jour, et signale-le si le message reste.

## Pendant un tour

**« Limite d’usage de Gemini atteinte : {mots de Gemini} »**
Le tour finit « limité ». L’agent reprend seul quand Gemini le dit (« Please retry in 30s »), sinon une minute plus tard, une heure plus tard pour son quota du jour, ou passe sur un autre compte selon « Changer de compte à » et « Continuer sur le harness suivant… ».

**« La session Gemini CLI n’a pas pu être reprise : nouvelle session (contexte vidé) »**
Gemini n’a pas retrouvé la session de l’agent. L’agent continue, sans le contexte d’avant.

**« Gemini CLI : {erreur} »**
Gemini a fini le tour en erreur, ou refusé un réglage (le mode, le modèle : `session/set_mode: …`), avec ses propres mots. Lis-les ; si cela revient, signale-le avec la version de Gemini CLI.

**Une carte qui dit « Hors du dossier de l’agent » ou « Serveur MCP {serveur}, outil {outil} »**
La demande touche un chemin hors du dossier de l’agent, ou c’est un outil d’un serveur MCP.

**Gemini dit « Refused by Escouade: a path of a private project. »** ou **« Refused by Escouade: this conversation changes no file and runs no command. »**
Les règles qu’Escouade lui a données ont refusé l’outil : la clôture d’un projet privé, ou une discussion. Rien à faire.

**L’agent ne passe pas en Bypass alors que Gemini écrit « [MODE_UPDATE] yolo »**
C’est voulu : un texte du modèle ne monte jamais le mode. Change le mode dans le Composer.

## Le plugin

**« Le plugin de Gemini CLI a redémarré. »**
Escouade l’a relancé. L’agent garde sa session, son tour et les demandes qui attendent.

**« Le plugin de Gemini CLI s’est arrêté après {n} plantages : l’agent s’arrête, sa session est gardée. Relance le plugin dans Réglages › Plugins, puis envoie un message. »**
Après trois plantages en cinq minutes, Escouade ne le relance plus. Relance-le, puis envoie un message. Son journal dit ce qui s’est passé.

**« Le plugin de Gemini CLI est arrêté ({raison}) : l’agent s’arrête, sa session est gardée. »**, **« Le plugin de Gemini CLI ne répond pas : {raison} »**, **« Le plugin de Gemini CLI n’a pas traduit ce que son harness a dit ({raison}). »**
Rallume le plugin et envoie un message. Si cela revient, signale-le avec la version de Gemini CLI.
