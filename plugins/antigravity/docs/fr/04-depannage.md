# Dépannage

Les messages ci-dessous sont ceux qu’Escouade et le plugin affichent, mot pour mot ; `{…}` est ce qui change d’une fois à l’autre.

## L’agent ne démarre pas

**« Antigravity CLI introuvable. Installe-le ou indique son chemin dans les réglages. »**
Escouade n’a pas trouvé `agy`. Installe-le (voir l’onglet Installer), ou donne le chemin de son exécutable dans Réglages › Comptes et harness.

**« Aucun compte Antigravity CLI : ajoute-en un dans Réglages › Comptes et harness. »**
Ajoute le compte de la machine ou une clé d’API Gemini.

**« Antigravity CLI n’a pas démarré la conversation de l’agent : {raison}. S’il n’est pas connecté, lance agy dans un terminal pour te connecter à ton compte Google. »**
`agy` n’a pas ouvert la conversation. Le plus souvent, la connexion Google manque : « Se connecter… » sur la fiche du compte, ou `agy` dans un terminal.

**« Le plugin Antigravity CLI (dev.escouade.antigravity-cli) est coupé ou n’est pas chargé : rallume-le dans la vue Plugins pour lancer un agent de Antigravity CLI. »**
Le plugin est désactivé, ou n’a pas pu se charger. Rallume-le ; sa page et son journal disent pourquoi. L’agent garde sa conversation.

**« Le plugin Antigravity CLI (dev.escouade.antigravity-cli) n’est plus installé : la vue Plugins propose de l’installer, pour lancer un agent de Antigravity CLI. »**
Installe-le de nouveau depuis la vue Plugins ; les conversations des agents sont gardées.

**« Le plugin de Antigravity CLI n’applique pas la clôture des projets privés : l’agent ne démarre pas. »**, **« … ne tient pas son harness en lecture seule : la discussion ne démarre pas. »**, **« … met le jeton d’Escouade dans les arguments de son harness, que tout le monde peut lire : l’agent ne démarre pas. »**, **« … veut écrire {fichier}, hors du dossier de l’agent : l’agent ne démarre pas. »**
Escouade a refusé le lancement que le plugin a décrit. Le plugin officiel ne fait pas cela : mets-le à jour, et signale-le si le message reste.

## Pendant un tour

**« Limite d’usage d’Antigravity atteinte : {mots d’agy} »**
Le quota est épuisé : le tour finit « limité ». L’agent réessaie seul à la remise à zéro qu’Antigravity annonce, sinon une heure plus tard (une minute si c’est la capacité du modèle qui manque), ou passe sur un autre compte selon « Changer de compte à » et « Continuer sur le harness suivant… ». Son `/usage`, dans sa vue terminal, dit où en est le quota.

**« La session Antigravity n’a pas pu être reprise : nouvelle session (contexte vidé) »**
La conversation à reprendre n’existe plus. L’agent continue, sans le contexte d’avant.

**Des cartes arrivent toutes à la fin du tour**
C’est normal : `agy` ne demande rien pendant un tour. Réponds-leur, et l’agent reprend dans le même tour avec ce que tu as permis.

**Une carte répondue ne fait rien, et le tour finit**
Le plugin a redémarré pendant que la carte attendait : elle ne peut plus reprendre, et sa réponse finit le tour. Envoie un message pour que l’agent refasse ce qu’il voulait.

**Une carte qui dit « Hors du dossier de l’agent » ou « Serveur MCP {serveur}, outil {outil} »**
La demande touche un chemin hors du dossier de l’agent, ou c’est un outil d’un serveur MCP.

## Le plugin

**« Le plugin de Antigravity CLI a redémarré. »**
Escouade l’a relancé. `agy` a continué ; ce qui était en cours de traduction peut manquer jusqu’à la fin du tour.

**« Le plugin de Antigravity CLI s’est arrêté après {n} plantages : l’agent s’arrête, sa session est gardée. Relance le plugin dans Réglages › Plugins, puis envoie un message. »**
Après trois plantages en cinq minutes, Escouade ne le relance plus. Relance-le, puis envoie un message. Son journal dit ce qui s’est passé.

**« Le plugin de Antigravity CLI est arrêté ({raison}) : l’agent s’arrête, sa session est gardée. »**, **« Le plugin de Antigravity CLI ne répond pas : {raison} »**, **« Le plugin de Antigravity CLI n’a pas traduit ce que son harness a dit ({raison}). »**
Rallume le plugin et envoie un message. Si cela revient, signale-le avec la version d’Antigravity CLI.
