# Dépannage de Slack

Ce que le plugin écrit dans son journal, onglet **Journal** de sa page (le fichier entier : `~/.escouade/plugin-logs/dev.escouade.slack.log`), ce que ça veut dire et quoi faire.

## Rien n’arrive, et le journal ne dit rien

Aucun fait ne lui est parvenu. Vérifie :

- que le plugin est **activé** pour l’app (son interrupteur dans **Installés**) et que son état n’est pas « Désactivé » ;
- qu’il est actif pour le **projet** (onglet **Projets** ; un projet privé est coupé d’office) ;
- que tu n’as pas retiré `ticket.column` ou `agent.status` dans l’onglet **Permissions** ;
- que le fait attendu a eu lieu : un ticket arrivé dans « À tester », un agent qui attend ta réponse. Les autres colonnes et statuts ne postent rien.

## Les messages du journal

**`posted in #escouade-boutique`**
Envoyé : Slack a accepté le message.

**`no Slack token kept: nothing sent (Plugins › Slack › Settings)`**
Aucun jeton n’est gardé. Colle le jeton du bot dans l’onglet **Réglages** de la page du plugin, section **Secrets**, puis **Garder**.

**`the token kept is not a Slack bot token (xoxb-…): nothing sent`**
Le jeton gardé ne commence pas par `xoxb-` : c’est peut-être un jeton d’utilisateur (`xoxp-`) ou celui de l’app. Copie le **Bot User OAuth Token** depuis **OAuth & Permissions**.

**`Slack refused the message in #escouade-boutique: {erreur}`**
Slack a répondu, mais a refusé. La fin est le code d’erreur de Slack :
- `not_in_channel` : le bot n’est pas dans le salon. Invite-le (`/invite @le-nom-de-ton-app`) ;
- `channel_not_found` : aucun salon de ce nom, ou un salon privé où le bot n’est pas. Crée le salon avec exactement ce nom, ou invite le bot ;
- `invalid_auth`, `token_revoked`, `account_inactive` : le jeton n’est plus bon. Réinstalle l’app dans l’espace de travail et colle le nouveau jeton ;
- `missing_scope` : le bot n’a pas `chat:write`. Ajoute-le, réinstalle l’app, et colle le nouveau jeton ;
- `is_archived` : le salon est archivé.

**`Slack could not be reached: {raison}`**
La requête n’a pas abouti : machine hors ligne, proxy, certificat, Slack n’a pas répondu en 10 secondes, ou sa réponse n’était pas du JSON (une panne de Slack). Vérifie ta connexion et **Réglages › Avancé › Réseau**. Ce message-là est perdu.

**`the queue of facts is full (256): the oldest, agent.status #17, is dropped`**
Les faits sont arrivés plus vite que le plugin ne les traitait : le plus ancien n’a pas été posté.

## Le plugin est arrêté ou désactivé

L’état du plugin, en tête de son journal et sur sa ligne dans **Installés**, dit pourquoi :

**« Ses fichiers ont changé depuis son installation (empreinte …) : installe-le à nouveau. »**
Un fichier de `~/.escouade/plugins/dev.escouade.slack` a été modifié (par exemple `channels.json`). Désinstalle-le en gardant ses réglages et ses secrets, puis réinstalle-le depuis le store.

**« Arrêté après {n} plantages »**
Son programme s’est arrêté plusieurs fois de suite. Lis les dernières lignes du journal, puis **Relancer**. Si ça recommence, signale-le sur sa page du store.
