# Dépannage

Les messages ci-dessous sont ceux du plugin et d'Escouade, tels qu'ils s'affichent : sous le formulaire de connexion, sur la ligne du compte dans Réglages › Services connectés, sur la carte d'un agent ou dans l'onglet de sa MR, ou au clic sur le ⚠ d'une carte dont la synchronisation a échoué. Quand GitLab donne une raison, elle suit le message.

## À la connexion

**« Indique l'adresse de l'instance. »**
L'adresse est vide et l'hébergeur n'a pas d'instance par défaut. Pour gitlab.com, laisse GitLab choisi et l'adresse vide.

**« Type d'hébergeur introuvable : choisis-le. »**
L'app n'a pas reconnu l'instance à son adresse. Choisis GitLab dans « Hébergeur ».

**« GitLab : donne un jeton, ou connecte la CLI glab à git.acme.dev (glab auth login) »**
Le champ du jeton est vide et `glab` n'a pas donné de jeton pour cette instance. Lance `glab auth login`, ou colle un jeton.

**« L'adresse doit être en https (http seulement sur cette machine). »**
L'adresse de l'instance commence par `http://`. Donne son adresse en `https://`.

**« GitLab refuse ces identifiants (401) »**
Le jeton est faux, expiré ou révoqué. Crée-en un autre avec « Créer un jeton ».

**« GitLab injoignable : … »**
L'appel n'est pas arrivé : une adresse mal tapée, pas de réseau, un proxy ou un pare-feu, ou plus de 15 secondes sans réponse. Vérifie l'adresse, ta connexion et Réglages › Avancé › Réseau.

## Sur la ligne du compte

**« Jeton expiré ou révoqué · @ada »**, **« Injoignable · @ada »**
La dernière vérification a été refusée, ou n'a pas joint l'instance. Remplace le jeton, ou « Vérifier » pour réessayer.

**« Trousseau du système illisible : relance Escouade ou reconnecte le compte. »**
Au démarrage, le trousseau n'a pas rendu le jeton. Aucun appel ne part tant que c'est le cas.

**« Le plugin GitLab est coupé (Réglages › Plugins) : ce compte ne sert plus rien, ses PR ne sont plus suivies »**
Tu as coupé le plugin. Rallume-le dans la vue Plugins.

**« Le plugin GitLab s'est arrêté (…) : ses PR ne sont plus suivies jusqu'à sa relance »**
Le plugin s'est arrêté seul, la raison entre parenthèses. Sa page de la vue Plugins dit son état ; son onglet « Journal » le relance.

## Sur une MR

**« Hébergeur arrêté : MR plus suivies »**
Sur la carte d'un agent : le plugin est coupé ou arrêté. La carte garde la MR telle qu'elle a été lue en dernier ; la première lecture qui marche efface le message.

**« GitLab : l'auto-merge n'a pas été activé »**
GitLab n'a ni programmé le merge ni mergé la MR : le projet ne le permet pas, ou la MR n'est pas en état (conflit, brouillon, pipeline absent). Regarde la MR dans GitLab.

**« Cet hébergeur ne sait pas faire : rebase merge »**, **« Cet hébergeur ne sait pas faire : semi-linear merge »**
GitLab merge par merge commit ou squash ; le rebase avant merge est un réglage du projet. Choisis merge ou squash.

**« GitLab : MR !12 introuvable »**
La MR n'existe pas dans ce projet, ou ton jeton ne la voit pas.

**« GitLab refuse l'accès (403) »**
Ton compte n'a pas le droit demandé dans ce projet (merger, relancer un job, approuver), ou le jeton n'a pas le droit `api`.

**« GitLab limite les requêtes (429) : réessaie dans un moment »**
GitLab a refusé un appel pour sa limite, deux fois de suite. Le plugin ralentit de lui-même ; attends un peu.

**« GitLab : introuvable (404) »**, **« GitLab : erreur 500 »** (ou une autre erreur du service), **« GitLab : réponse illisible »**, **« GitLab : trop de redirections »**
Le projet ou l'objet n'existe plus ou n'est pas visible, l'instance a répondu par une erreur, autre chose que du JSON (souvent un proxy ou une page de connexion), ou plus de dix redirections. Vérifie l'adresse de l'instance et Réglages › Avancé › Réseau.

**« GitLab : aucun jeton pour git.acme.dev. Reconnecte le compte dans Réglages › Services connectés. »**
Escouade n'a pas pu prêter de jeton au plugin. Reconnecte le compte.

## Pour GitLab Issues

**« Aucun compte GitLab connecté pour git.acme.dev »**
Le projet lié est sur une instance dont le compte a été déconnecté. Reconnecte l'instance dans « Hébergeurs de code ».

**« Le plugin dev.escouade.gitlab ne répond pas : … »**
La synchronisation d'une issue n'a pas eu de réponse : le plugin est coupé, arrêté ou trop lent. Rien n'est perdu : elle repart quand il répond.

**« Réponse illisible du plugin dev.escouade.gitlab : … »**
Le plugin a répondu quelque chose qu'Escouade ne comprend pas. Relance-le depuis son onglet « Journal » ; si ça revient, signale-le.

Les autres messages sont ceux d'en haut. Une synchronisation qui échoue est réessayée seule, après 1, 5, 15 puis 60 minutes, pendant 24 heures ; « Resynchroniser » la renvoie tout de suite.
