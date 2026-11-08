# Dépannage

Les messages ci-dessous sont ceux du plugin et d'Escouade, tels qu'ils s'affichent : sous le formulaire de connexion, sur la ligne du compte dans Réglages › Services connectés, sur la carte d'un agent ou dans l'onglet de sa PR, ou au clic sur le ⚠ d'une carte dont la synchronisation a échoué. Quand Azure donne une raison, elle suit le message.

## À la connexion

**« Indique l'adresse de l'instance. »**
Donne l'adresse de ton organisation (`dev.azure.com/acme`) ou de ta collection.

**« Azure DevOps : donne un jeton, ou connecte la CLI az à dev.azure.com (az login) »**
Le champ du jeton est vide et `az` n'a pas donné de jeton : elle n'est pas installée, ou pas connectée. Lance `az login`, ou colle un jeton personnel.

**« Azure DevOps : donne un jeton. »**
Le champ du jeton est vide pour un Azure DevOps Server, qui ne connaît pas les jetons de `az`. Colle un jeton personnel.

**« L'adresse doit être en https (http seulement sur cette machine). »**
L'adresse commence par `http://`. Donne son adresse en `https://`.

**« Azure DevOps refuse ces identifiants (401) »**
Le jeton est faux, expiré ou révoqué, ou il n'a pas accès à cette organisation. Azure répond aussi à un jeton refusé par une page de connexion : le plugin la lit comme ce refus. Crée un jeton avec « Créer un jeton ».

**« Azure DevOps injoignable : … »**
L'appel n'est pas arrivé : une adresse mal tapée, pas de réseau, un proxy ou un pare-feu, ou plus de 15 secondes sans réponse. Vérifie l'adresse, ta connexion et Réglages › Avancé › Réseau.

## Sur la ligne du compte

**« Jeton expiré ou révoqué · ada@acme.dev »**, **« Injoignable · ada@acme.dev »**
La dernière vérification a été refusée, ou n'a pas joint l'organisation. Remplace le jeton (ou `az login`), ou « Vérifier » pour réessayer.

**« Trousseau du système illisible : relance Escouade ou reconnecte le compte. »**
Au démarrage, le trousseau n'a pas rendu le jeton. Aucun appel ne part tant que c'est le cas.

**« Le plugin Azure DevOps est coupé (Réglages › Plugins) : ce compte ne sert plus rien, ses PR ne sont plus suivies »**
Tu as coupé le plugin. Rallume-le dans la vue Plugins.

**« Le plugin Azure DevOps s'est arrêté (…) : ses PR ne sont plus suivies jusqu'à sa relance »**
Le plugin s'est arrêté seul, la raison entre parenthèses. Sa page de la vue Plugins dit son état ; son onglet « Journal » le relance.

## Sur une PR

**« Hébergeur arrêté : PR plus suivies »**
Sur la carte d'un agent : le plugin est coupé ou arrêté. La carte garde la PR telle qu'elle a été lue en dernier ; la première lecture qui marche efface le message.

**« Azure DevOps : la branche cible ne permet pas ce merge (…) ; elle permet : … »**
La stratégie de branche de la cible n'autorise pas la stratégie demandée ; le message dit celles qu'elle autorise. Choisis-en une. Escouade ne réessaie pas : rien ne changerait.

**« Azure DevOps : l'auto-complete n'a pas été activé »**
Azure n'a pas enregistré l'auto-complete. Regarde la PR dans Azure DevOps (brouillon, droits sur la branche).

**« Azure DevOps : ce build n'a pas de journal »**
Le build n'a pas (encore) de journal à lire : il n'a pas démarré, ou il a été annulé avant.

**« Azure DevOps : commentaire inconnu (…) »**
La réponse ou la résolution vise un commentaire qu'Azure ne connaît plus.

**« Azure DevOps refuse l'accès (403) »**
Le jeton n'a pas le droit demandé (Code en écriture pour ouvrir ou merger, Build en exécution pour relancer), ou ton compte n'a pas ce droit sur le dépôt.

**« Azure DevOps limite les requêtes (429) : réessaie dans un moment »**
Azure a refusé un appel pour sa limite, deux fois de suite. Le plugin ralentit de lui-même ; attends un peu.

**« Azure DevOps : introuvable (404) »**, **« Azure DevOps : erreur 500 »** (ou une autre erreur du service), **« Azure DevOps : réponse illisible »**, **« Azure DevOps : trop de redirections »**
Le dépôt ou l'objet n'existe plus ou n'est pas visible, Azure a répondu par une erreur, autre chose que du JSON, ou plus de dix redirections. Réessaie ; si ça dure, regarde Réglages › Avancé › Réseau.

**« Azure DevOps : aucun jeton pour dev.azure.com. Reconnecte le compte dans Réglages › Services connectés. »**
Escouade n'a pas pu prêter de jeton au plugin. Reconnecte le compte.

## Pour Azure Boards

**« Azure DevOps refuse ces identifiants (401). Le jeton d'Azure DevOps doit avoir le droit Work Items (lecture et écriture) : crée-en un sur https://dev.azure.com/acme/_usersSettings/tokens, puis remplace-le dans Réglages › Services connectés › Hébergeurs de code. »**
Le jeton sert aux PR mais n'a pas le droit Work Items. Un refus 403 dit la même chose. Crée un jeton avec Work Items en lecture et écriture sur la page que nomme le message, puis remplace l'ancien.

**« Aucun compte Azure DevOps connecté pour l'organisation acme »**, **« Aucun compte Azure DevOps connecté »**
Le projet lié est dans une organisation dont le compte a été déconnecté. Reconnecte l'organisation dans « Hébergeurs de code ».

**« Azure Boards : aucun jeton pour … Reconnecte le compte dans Réglages › Services connectés. »**
Escouade n'a pas pu prêter de jeton au plugin. Reconnecte le compte.

**« Le plugin dev.escouade.azure-devops ne répond pas : … »**
La synchronisation d'un élément n'a pas eu de réponse : le plugin est coupé, arrêté ou trop lent. Rien n'est perdu : elle repart quand il répond.

**« Réponse illisible du plugin dev.escouade.azure-devops : … »**
Le plugin a répondu quelque chose qu'Escouade ne comprend pas. Relance-le depuis son onglet « Journal » ; si ça revient, signale-le.

Une synchronisation qui échoue est réessayée seule, après 1, 5, 15 puis 60 minutes, pendant 24 heures ; « Resynchroniser » la renvoie tout de suite.
