# Dépannage de Discord

Le plugin n’ayant pas de programme, il n’affiche rien dans l’app. Ce qu’Escouade fait pour lui est écrit dans son journal, le fichier `~/.escouade/plugin-logs/dev.escouade.discord.log` (l’onglet **Journal** de sa page ne le montre pas pour un plugin sans programme). Ouvre-le pour savoir ce qui s’est passé.

## Rien n’arrive dans le salon, et le journal ne dit rien

Aucun envoi n’a été tenté. Vérifie :

- que le plugin est **activé** pour l’app (son interrupteur dans **Installés**) ;
- qu’il est actif pour le **projet** (onglet **Projets** ; un projet privé est coupé d’office) ;
- que tu n’as pas retiré `ticket.column`, `agent.status` ou `discord.com` dans l’onglet **Permissions** ;
- que le fait attendu a eu lieu : un ticket arrivé dans « À tester », un agent qui attend ta réponse. Les autres colonnes et statuts ne postent rien.

## Les lignes du journal

**`webhook to discord.com: 204 No Content (ticket.column #42)`**
Envoyé : Discord a accepté le message.

**`webhook to discord.com: {code} (ticket.column #42) {réponse}`**
Discord a refusé le message ; la fin de la ligne est le début de sa réponse (300 caractères au plus). Selon le code :
- `404 Not Found` : le webhook n’existe plus (supprimé dans Discord) ou l’adresse est fausse. Recrée-le, puis colle sa nouvelle adresse dans l’onglet **Réglages** de la page du plugin, section **Secrets** ;
- `401 Unauthorized` : la fin de l’URL (le jeton du webhook) est fausse. Recopie l’URL entière ;
- `429 Too Many Requests` : Discord limite le nombre de messages, celui-là est perdu. Coupe les projets les plus bavards dans l’onglet **Projets**.

**`webhook to discord.com: no answer (ticket.column #42): timed out`**
Discord n’a pas répondu en 10 secondes. Le message est perdu ; les suivants partiront si Discord répond.

**`webhook to discord.com: no answer (…): {raison}`**
La requête n’a pas abouti : machine hors ligne, proxy, certificat. Vérifie ta connexion et **Réglages › Avancé › Réseau**.

**`a webhook's address is not one (ticket.column)`**
Aucune adresse n’est gardée, ou ce qui est gardé n’est pas une adresse. Colle l’URL du webhook dans l’onglet **Réglages** de la page du plugin, section **Secrets**, puis **Garder**.

**``a webhook to {hôte} is not sent: not an http(s) address of a host of `permissions.network` (ticket.column)``**
L’adresse gardée ne pointe pas vers `discord.com`. Une adresse en `discordapp.com` (l’ancien domaine de Discord) est refusée : remplace `discordapp.com` par `discord.com` dans l’URL, ou recopie-la depuis Discord.

**`the queue of facts is full (256): the oldest, agent.status #17, is dropped`**
Les faits sont arrivés plus vite que Discord ne répondait : le plus ancien n’a pas été envoyé.

## Le plugin est désactivé

Sa ligne dans **Installés** dit pourquoi :

**« Ses fichiers ont changé depuis son installation (empreinte …) : installe-le à nouveau. »**
Un fichier de `~/.escouade/plugins/dev.escouade.discord` a été modifié. Désinstalle-le en gardant ses réglages et ses secrets, puis réinstalle-le depuis le store.
