# Dépannage

Les messages ci-dessous sont ceux du plugin et d'Escouade, tels qu'ils s'affichent : sous le formulaire de connexion, sur la ligne du compte dans Réglages › Services connectés, sur la carte d'un agent ou dans l'onglet de sa PR. Le plugin se nomme « Gitea » dans ses messages, pour Forgejo aussi. Quand l'instance donne une raison, elle suit le message.

## À la connexion

**« Indique l'adresse de l'instance. »**
Gitea / Forgejo n'a pas d'instance par défaut : donne son adresse (`https://codeberg.org`).

**« Type d'hébergeur introuvable : choisis-le. »**
L'app n'a pas reconnu l'instance à son adresse. Choisis Gitea / Forgejo dans « Hébergeur ».

**« Gitea : donne un jeton. »**
Le champ « Jeton » est vide. Il n'y a pas de CLI dont Escouade prendrait le jeton : crée-en un avec « Créer un jeton ».

**« L'adresse doit être en https (http seulement sur cette machine). »**
L'adresse de l'instance commence par `http://`. Donne son adresse en `https://`.

**« Gitea refuse ces identifiants (401) »**
Le jeton est faux ou a été supprimé. Crée-en un autre avec les droits que le formulaire rappelle.

**« Gitea injoignable : … »**
L'appel n'est pas arrivé : une adresse mal tapée, pas de réseau, un proxy ou un pare-feu, ou plus de 15 secondes sans réponse. Vérifie l'adresse, ta connexion et Réglages › Avancé › Réseau.

## Sur la ligne du compte

**« Jeton expiré ou révoqué · @ada »**, **« Injoignable · @ada »**
La dernière vérification a été refusée, ou n'a pas joint l'instance. Remplace le jeton, ou « Vérifier » pour réessayer.

**« Trousseau du système illisible : relance Escouade ou reconnecte le compte. »**
Au démarrage, le trousseau n'a pas rendu le jeton. Aucun appel ne part tant que c'est le cas.

**« Le plugin Gitea / Forgejo est coupé (Réglages › Plugins) : ce compte ne sert plus rien, ses PR ne sont plus suivies »**
Tu as coupé le plugin. Rallume-le dans la vue Plugins.

**« Le plugin Gitea / Forgejo s'est arrêté (…) : ses PR ne sont plus suivies jusqu'à sa relance »**
Le plugin s'est arrêté seul, la raison entre parenthèses. Sa page de la vue Plugins dit son état ; son onglet « Journal » le relance.

## Sur une PR

**« Hébergeur arrêté : PR plus suivies »**
Sur la carte d'un agent : le plugin est coupé ou arrêté. La carte garde la PR telle qu'elle a été lue en dernier ; la première lecture qui marche efface le message.

**« Gitea : l'auto-merge n'existe que sur une branche protégée, et main ne l'est pas »**
L'instance ne programme l'auto-merge que sur une branche protégée. Protège la branche cible dans les réglages du dépôt, ou merge quand c'est vert depuis Escouade.

**« Cet hébergeur ne sait pas faire : job_log »**, **« … : rerun_failed »**, **« … : rerun_failed_of_branch »**, **« … : reply »**, **« … : resolve »**
La version de l'instance n'a pas l'appel : le journal d'un job (Gitea avant 1.25, Forgejo avant 12), la relance des jobs (Gitea avant 1.26, tout Forgejo), la réponse dans un fil ou sa résolution (Gitea avant 1.26, tout Forgejo). Voir le tableau de **Utiliser**.

**« Gitea : PR sans commit »**, **« Gitea : branche main sans commit »**
L'instance n'a pas donné de dernier commit pour la PR ou la branche, dont la CI ne peut donc pas se lire. Réessaie après un push.

**« Gitea refuse l'accès (403) »**
Le jeton n'a pas le droit demandé (un des trois droits manque), ou ton compte n'a pas ce droit sur le dépôt (merger, relire).

**« Gitea limite les requêtes (429) : réessaie dans un moment »**
L'instance limite les appels. Attends un peu.

**« Gitea : introuvable (404) »**, **« Gitea : erreur 500 »** (ou une autre erreur du service), **« Gitea : réponse illisible »**, **« Gitea : trop de redirections »**
Le dépôt ou l'objet n'existe plus ou n'est pas visible, l'instance a répondu par une erreur, autre chose que du JSON (souvent un proxy ou une page de connexion), ou plus de dix redirections. Vérifie l'adresse de l'instance et Réglages › Avancé › Réseau.

**« Gitea / Forgejo : aucun jeton pour git.acme.dev. Reconnecte le compte dans Réglages › Services connectés. »**
Escouade n'a pas pu prêter de jeton au plugin. Reconnecte le compte.

**« Le plugin dev.escouade.gitea ne répond pas : … »**
Le plugin est coupé, arrêté ou trop lent. Rien n'est perdu : les PR se relisent quand il répond.
