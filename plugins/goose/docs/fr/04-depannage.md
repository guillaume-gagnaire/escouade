# Dépannage

Les messages ci-dessous sont ceux qu’Escouade et le plugin ACP affichent, mot pour mot ; `{…}` est ce qui change d’une fois à l’autre.

## L’agent ne démarre pas

**« Goose lit et écrit les fichiers lui-même, sans les faire passer par Escouade (son manifeste le dit) : la clôture des projets privés ne tiendrait pas, l’agent ne démarre pas dans ce projet privé. »**
Un de tes projets est privé : chaque agent a alors une clôture, et Goose ne peut pas la tenir. Le message parle de « ce projet privé », mais il vaut pour tout projet tant qu’un projet est privé. Rends tes projets non privés, ou utilise un autre harness.

**« Goose introuvable. Installe-le ou indique son chemin dans les réglages. »**
Escouade n’a pas trouvé `goose` dans le `PATH`. Installe Goose (voir l’onglet Installer) et relance Escouade si le `PATH` a changé.

**« Aucun compte Goose : ajoute-en un dans Réglages › Comptes et harness. »**
Ajoute le compte de la machine ou un nouveau compte.

**« Goose n’a pas ouvert la session de l’agent : {raison}. S’il demande une connexion, connecte-le dans un terminal. »**
Goose a refusé la session. Le plus souvent, son fournisseur n’est pas configuré : `goose configure`.

**« Goose parle ACP {version}, Escouade ACP {version} : mets Goose à jour. »**
`goose update`.

**L’agent est marqué « harness absent »**
Le plugin Goose ou le plugin ACP est coupé ou désinstallé. Rallume-les dans la vue Plugins : l’agent revient avec sa conversation.

## Pendant un tour

**« La session Goose n’a pas pu être reprise : nouvelle session (contexte vidé) »**
Goose n’a pas retrouvé la session de l’agent. L’agent continue, sans le contexte d’avant.

**« Goose : {erreur} »**
Goose a fini le tour en erreur, ou refusé un réglage (le mode, le modèle), avec ses propres mots. Lis-les.

**Une carte qui dit « Hors du dossier de l’agent » ou « Serveur MCP {serveur}, outil {outil} »**
La demande touche un chemin hors du dossier de l’agent, ou c’est un outil d’un serveur MCP.

## Le plugin ACP

Les messages du plugin qui fait tourner Goose (« Le plugin de Goose a redémarré. », « … s’est arrêté après {n} plantages… ») sont dans la documentation du plugin ACP.
