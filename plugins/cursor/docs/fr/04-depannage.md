# Dépannage

Les messages ci-dessous sont ceux qu’Escouade et le plugin affichent, mot pour mot ; `{…}` est ce qui change d’une fois à l’autre.

## L’agent ne démarre pas

**« Cursor CLI introuvable. Installe-le ou indique son chemin dans les réglages. »**
Escouade n’a pas trouvé `agent`. Installe Cursor CLI (voir l’onglet Installer), ou donne le chemin de son exécutable dans Réglages › Comptes et harness.

**« Aucun compte Cursor CLI : ajoute-en un dans Réglages › Comptes et harness. »**
Ajoute le compte de la machine ou un compte par sa clé d’API.

**« Cursor CLI n’a pas ouvert la session de l’agent : {raison}. S’il demande une connexion, connecte-le dans un terminal. »**
Cursor a refusé la session. Le plus souvent, la machine n’est pas connectée : lance `agent login` dans un terminal.

**« Cursor CLI parle ACP {version}, Escouade ACP {version} : mets Cursor CLI à jour. »**
Cursor parle une autre version d’ACP que le plugin. `agent update`.

**« Le plugin Cursor CLI (dev.escouade.cursor-cli) est coupé ou n’est pas chargé : rallume-le dans la vue Plugins pour lancer un agent de Cursor CLI. »**
Le plugin est désactivé, ou n’a pas pu se charger. Rallume-le ; sa page et son journal disent pourquoi. L’agent garde sa session.

**« Le plugin Cursor CLI (dev.escouade.cursor-cli) n’est plus installé : la vue Plugins propose de l’installer, pour lancer un agent de Cursor CLI. »**
Installe-le de nouveau depuis la vue Plugins ; les conversations des agents sont gardées.

**« Le plugin de Cursor CLI n’applique pas la clôture des projets privés : l’agent ne démarre pas. »**, **« … met le jeton d’Escouade dans les arguments de son harness, que tout le monde peut lire : l’agent ne démarre pas. »**
Escouade a refusé le lancement que le plugin a décrit. Le plugin officiel ne fait pas cela : mets-le à jour, et signale-le si le message reste.

## Le compte et ses limites

**« Cursor CLI n’est pas connecté sur cette machine (« Please sign in to continue ») : lance agent login dans un terminal. »**
Le compte de la machine n’est plus connecté. `agent login`, puis envoie un message.

**« Cursor a refusé la clé d’API de ce compte : donne-la de nouveau dans Réglages › Comptes et harness, « Changer la clé… ». »**
La clé du compte est fausse ou révoquée. Crée-en une sur cursor.com, Settings › API Keys, et donne-la par « Changer la clé… ».

**« Limite d’usage de ce compte Cursor atteinte (« Upgrade your plan to continue ») : l’usage inclus du mois est épuisé jusqu’à la remise à zéro de son offre. Son usage : {adresse} »**
L’usage du mois est épuisé, sur Auto. La carte propose, s’il y en a, un autre compte ou un autre harness où continuer.

**« Cursor refuse le modèle {modèle} sur ce compte (« Upgrade your plan to continue ») : il n’est pas dans l’offre de ce compte Cursor, ou son usage inclus du mois est épuisé. Choisis Auto dans le menu Modèle ; son usage : {adresse} »**
Le modèle choisi n’est pas dans l’offre (une offre Free n’a qu’Auto). Choisis Auto.

**« Limite d’usage de ce compte Cursor atteinte : la suite passe par l’usage à la demande, qui demande un moyen de paiement (« Add a payment method to continue »). Son usage : {adresse} »**
Ajoute un moyen de paiement sur cursor.com, ou continue sur un autre compte.

## Pendant un tour

**« La session Cursor CLI n’a pas pu être reprise : nouvelle session (contexte vidé) »**
Cursor n’a pas retrouvé la session de l’agent. L’agent continue, sans le contexte d’avant.

**« Cursor n’a rien écrit : l’écriture a été refusée, par un hook (ceux de Claude Code tournent aussi sous Cursor) ou par Cursor lui-même, qui n’en donne pas la raison. »**
Un outil d’écriture a fini sans rien changer. Regarde tes hooks de Claude Code, du compte et du projet.

**Une carte qui dit « Hors du dossier de l’agent » ou « Serveur MCP {serveur}, outil {outil} »**
La demande touche un chemin hors du dossier de l’agent, ou c’est un outil d’un serveur MCP.

**« Cursor CLI : {erreur} »**
Cursor a fini le tour en erreur, ou refusé un réglage (le mode, le modèle : `session/set_mode: …`), avec ses propres mots. Lis-les ; si cela revient, signale-le avec la version de Cursor CLI.

## Le plugin

**« Le plugin de Cursor CLI a redémarré. »**
Escouade l’a relancé. L’agent garde sa session, son tour et les demandes qui attendent.

**« Le plugin de Cursor CLI s’est arrêté après {n} plantages : l’agent s’arrête, sa session est gardée. Relance le plugin dans Réglages › Plugins, puis envoie un message. »**
Après trois plantages en cinq minutes, Escouade ne le relance plus. Relance-le, puis envoie un message. Son journal dit ce qui s’est passé.

**« Le plugin de Cursor CLI est arrêté ({raison}) : l’agent s’arrête, sa session est gardée. »**, **« Le plugin de Cursor CLI ne répond pas : {raison} »**, **« Le plugin de Cursor CLI n’a pas traduit ce que son harness a dit ({raison}). »**
Rallume le plugin et envoie un message. Si cela revient, signale-le avec la version de Cursor CLI.
