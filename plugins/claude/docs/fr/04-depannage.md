# Dépannage

Les messages ci-dessous sont ceux qu’Escouade et le plugin affichent, mot pour mot ; `{…}` est ce qui change d’une fois à l’autre.

## L’agent ne démarre pas

**« Claude Code introuvable. Installe-le ou indique son chemin dans les réglages. »**
Escouade n’a pas trouvé `claude`. Installe Claude Code (voir l’onglet Installer), ou donne le chemin de son exécutable dans Réglages › Comptes et harness, ou dans le champ « Exécutable » du compte.

**« Aucun compte Claude Code : ajoute-en un dans Réglages › Comptes et harness. »**
Aucun compte n’existe pour ce harness. Ajoute le compte de la machine ou un nouveau compte.

**« Le plugin Claude Code (dev.escouade.claude-code) est coupé ou n’est pas chargé : rallume-le dans la vue Plugins pour lancer un agent de Claude Code. »**
Le plugin est désactivé, ou n’a pas pu se charger. Rallume-le dans la vue Plugins ; sa page dit pourquoi il ne s’est pas chargé, et son journal aussi. L’agent garde sa session.

**« Le plugin Claude Code (dev.escouade.claude-code) n’est plus installé : la vue Plugins propose de l’installer, pour lancer un agent de Claude Code. »**
Tu l’as désinstallé. Installe-le de nouveau depuis la vue Plugins ; les conversations des agents sont gardées.

**« Le plugin de Claude Code n’applique pas la clôture des projets privés : l’agent ne démarre pas. »**
L’agent est dans un projet privé, ou un projet privé existe, et le lancement que le plugin a décrit ne nomme pas le fichier de réglages qui porte la clôture. Mets le plugin à jour ; si le message reste, signale-le.

**« Le plugin de Claude Code ne tient pas son harness en lecture seule : la discussion ne démarre pas. »**
Une discussion de l’onglet Docs n’a pas été décrite en lecture seule. Même chose : mets le plugin à jour.

**« Le plugin de Claude Code met le jeton d’Escouade dans les arguments de son harness, que tout le monde peut lire : l’agent ne démarre pas. »**
Escouade a refusé un lancement qui aurait exposé le jeton. Le plugin officiel ne le fait pas : vérifie que c’est bien lui qui est installé.

## Pendant un tour

**« Le plugin de Claude Code a redémarré. »**
Le plugin s’est arrêté et Escouade l’a relancé. `claude` a continué ; ce qui était en cours de traduction peut manquer dans la conversation jusqu’à la fin du tour.

**« Le plugin de Claude Code s’est arrêté après {n} plantages : l’agent s’arrête, sa session est gardée. Relance le plugin dans Réglages › Plugins, puis envoie un message. »**
Après trois plantages en cinq minutes, Escouade ne relance plus le plugin. Relance-le, puis envoie un message : l’agent reprend sa session. Le journal du plugin, sur sa page, dit ce qui s’est passé.

**« Le plugin de Claude Code est arrêté ({raison}) : l’agent s’arrête, sa session est gardée. »**
Le plugin a été coupé ou arrêté pendant que l’agent tournait. Rallume-le et envoie un message.

**« Le plugin de Claude Code ne répond pas : {raison} »** et **« Le plugin de Claude Code n’a pas traduit ce que son harness a dit ({raison}). »**
Le plugin n’a pas répondu à temps, ou a refusé une ligne de `claude`. Si cela revient, regarde son journal et signale-le avec la version de Claude Code.

**« Claude Code a signalé une erreur »**
Le tour a fini en erreur sans que Claude Code dise laquelle. Réessaie ; si cela revient, ouvre la vue Claude Code pour voir ce qu’il dit.

**« Nouvelle conversation Claude (contexte vidé) »**
Claude Code a ouvert une nouvelle conversation au lieu de reprendre l’ancienne (une session effacée, ou d’un autre compte). L’agent continue, sans le contexte d’avant.

**« Contexte compacté »**
Claude Code a résumé la conversation pour faire de la place (`/compact`, ou de lui-même). Rien à faire.

## Le compte

**La carte « Connexion à refaire »**
Claude Code n’a pas pu rafraîchir la connexion du compte (« OAuth session expired and could not be refreshed »). Clique sur « Se reconnecter » : l’agent reprend une fois connecté. Si la connexion sur le disque est déjà bonne, il reprend tout seul.
