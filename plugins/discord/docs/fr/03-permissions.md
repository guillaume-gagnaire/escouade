# Permissions de Discord

Ce que le plugin peut faire, ce qui part vers Discord, et ce qu’il ne peut pas faire.

## Ce qu’il est

Un plugin **déclaratif** : pas de programme, rien de lui ne s’exécute. Son manifeste déclare deux webhooks (une adresse, les faits qui les déclenchent, le texte du message), et c’est Escouade qui les envoie.

## Ce qu’il demande

| Permission | Ce que dit la fenêtre d’installation |
| --- | --- |
| `events` : `ticket.column`, `agent.status` | « Reçoit ces événements » |
| `network` : `discord.com` | « Se connecte à discord.com », appliqué par Escouade : ses webhooks ne partent que vers cet hôte |
| `secrets` : `webhook` | « Garde un secret : webhook » |

Chacune se retire dans l’onglet **Permissions** de sa page. Sans `events:…`, ses webhooks ne reçoivent plus ces faits ; sans `discord.com`, aucun envoi ne part.

## Les faits qu’il reçoit

Seulement `ticket.column` (un ticket change de colonne) et `agent.status` (un agent change de statut), et seulement ceux des projets où il est actif. Un projet privé ne lui envoie rien tant que tu ne l’y as pas activé.

Escouade ne poste que deux cas : un ticket qui arrive dans « À tester », un agent qui attend ta réponse. Les autres faits sont reçus et ignorés.

## Ce qui part vers Discord

Pour chaque message, une requête `POST` vers l’adresse du webhook, depuis ta machine, avec ce texte et rien d’autre :

- un ticket : sa clé, son titre et le nom du projet ;
- un agent : son nom et le nom du projet.

Escouade vérifie avant chaque envoi que l’adresse est en `http` ou `https` et que son hôte est exactement `discord.com`. Une adresse vers un autre hôte n’est pas envoyée, et le refus va au journal.

## Ce qui n’y part jamais

Ni la conversation de l’agent, ni la description du ticket, ni ses critères, ni ton code, ni un chemin de ta machine, ni le nom de la branche. Le texte posté est fixé par le manifeste : il ne contient que les champs ci-dessus.

## Le secret

L’adresse du webhook est le seul secret. Escouade la garde dans le trousseau du système, sous un nom propre au plugin, et ne l’utilise que comme adresse de ses webhooks : elle n’entre jamais dans le texte d’un message. Elle est masquée dans le journal.

## Ce qu’il ne peut pas faire

- Rien lancer sur ta machine : il n’a pas de programme.
- Lire ou écrire un fichier.
- Joindre un autre hôte que `discord.com`.
- Décider des autorisations de tes agents.
- Lire les messages du salon : un webhook ne fait qu’écrire.
