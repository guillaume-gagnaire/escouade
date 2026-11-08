# Dépannage

Les messages ci-dessous sont ceux du plugin et d'Escouade, tels qu'ils s'affichent : sous le formulaire de connexion, sur la ligne du compte dans Réglages › Services connectés, sur la carte d'un agent ou dans l'onglet de sa PR, ou au clic sur le ⚠ d'une carte dont la synchronisation a échoué. Quand GitHub donne une raison, elle suit le message.

## À la connexion

**« GitHub : donne un jeton, ou connecte la CLI gh à github.com (gh auth login) »**
Le champ du jeton est vide et `gh` n'a pas donné de jeton : elle n'est pas installée, ou pas connectée à cette instance. Lance `gh auth login` (avec `--hostname` pour un Enterprise), ou colle un jeton.

**« L'adresse doit être en https (http seulement sur cette machine). »**
L'adresse de l'instance commence par `http://`. Donne son adresse en `https://`.

**« GitHub refuse ces identifiants (401) »**
Le jeton est faux, expiré ou révoqué. Crée-en un autre avec « Créer un jeton ».

**« GitHub injoignable : … »**
L'appel n'est pas arrivé : une adresse mal tapée, pas de réseau, un proxy ou un pare-feu, ou plus de 15 secondes sans réponse. Vérifie l'adresse, ta connexion et Réglages › Avancé › Réseau.

## Sur la ligne du compte

**« Droits manquants : repo »**
Un jeton classic sans le droit `repo` : il ne peut ni lire les dépôts privés ni ouvrir de PR. Crée un jeton avec `repo`.

**« Jeton expiré ou révoqué · @ada »**
GitHub refuse le jeton à la dernière vérification. Remplace-le, ou reconnecte `gh` (`gh auth login`).

**« Injoignable · @ada »**
La dernière vérification n'a pas joint l'instance. « Vérifier » la refait.

**« Trousseau du système illisible : relance Escouade ou reconnecte le compte. »**
Au démarrage, le trousseau n'a pas rendu le jeton. Aucun appel ne part tant que c'est le cas.

**« Le plugin GitHub est coupé (Réglages › Plugins) : ce compte ne sert plus rien, ses PR ne sont plus suivies »**
Tu as coupé le plugin. Rallume-le dans la vue Plugins.

**« Le plugin GitHub s'est arrêté (…) : ses PR ne sont plus suivies jusqu'à sa relance »**
Le plugin s'est arrêté seul, la raison entre parenthèses. Sa page de la vue Plugins dit son état ; son onglet « Journal » le relance.

## Sur une PR

**« Hébergeur arrêté : PR plus suivies »**
Sur la carte d'un agent : le plugin est coupé ou arrêté. La carte garde la PR telle qu'elle a été lue en dernier ; la première lecture qui marche efface le message.

**« GitHub : l'auto-merge n'a pas été activé »**
GitHub n'a pas pris l'auto-merge : le dépôt ne l'autorise pas (« Allow auto-merge » éteint), ou la PR n'est pas en état. Active-le dans les réglages du dépôt, ou merge à la main.

**« Cet hébergeur ne sait pas faire : semi-linear merge »**
GitHub n'a pas de merge semi-linéaire. Choisis merge, squash ou rebase.

**« GitHub : PR #42 introuvable »**
La PR n'existe pas dans ce dépôt, ou ton jeton ne la voit pas.

**« GitHub : PR sans commit »**
GitHub n'a pas donné de dernier commit pour la PR, dont la CI ne peut donc pas se lire. Réessaie après un push.

**« GitHub (GraphQL) : … »**
L'API GraphQL de GitHub a refusé la requête ; sa raison suit. Souvent un jeton qui ne voit pas le dépôt.

**« GitHub refuse l'accès (403) »**
Le jeton n'a pas le droit demandé sur ce dépôt (un jeton fine-grained sans ce dépôt ou sans ce droit, une organisation qui exige l'autorisation SSO du jeton), ou GitHub limite les requêtes. Vérifie les droits du jeton.

**« GitHub limite les requêtes (429) : réessaie dans un moment »**
La limite de l'API est atteinte. Le plugin ralentit de lui-même ; attends la fin de la fenêtre.

**« GitHub : introuvable (404) »**, **« GitHub : erreur 500 »** (ou une autre erreur du service), **« GitHub : réponse illisible »**, **« GitHub : trop de redirections »**
Le dépôt ou l'objet n'existe plus ou n'est pas visible, GitHub a répondu par une erreur, autre chose que du JSON (souvent un proxy), ou plus de dix redirections. Réessaie ; si ça dure, regarde Réglages › Avancé › Réseau.

**« GitHub : aucun jeton pour github.com. Reconnecte le compte dans Réglages › Services connectés. »**
Escouade n'a pas pu prêter de jeton au plugin. Reconnecte le compte.

## Pour GitHub Issues

**« Le plugin dev.escouade.github ne répond pas : … »**
La synchronisation d'une issue n'a pas eu de réponse : le plugin est coupé, arrêté ou trop lent. Rien n'est perdu : elle repart quand il répond.

**« Réponse illisible du plugin dev.escouade.github : … »**
Le plugin a répondu quelque chose qu'Escouade ne comprend pas. Relance-le depuis son onglet « Journal » ; si ça revient, signale-le.

Les autres messages sont ceux d'en haut (« GitHub refuse ces identifiants (401) »…). Une synchronisation qui échoue est réessayée seule, après 1, 5, 15 puis 60 minutes, pendant 24 heures ; « Resynchroniser » la renvoie tout de suite.

## Le journal d'un job ne s'affiche pas

Sur un GitHub Enterprise Server qui range ses journaux d'Actions sur un autre hôte que lui-même, le plugin ne peut pas les lire : cet hôte n'est pas dans ses permissions.
