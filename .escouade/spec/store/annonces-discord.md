---
title: "Les annonces du store sur Discord"
subject: "store"
sources: ["docs/STORE.md","store/README.md","store/server/utils/discord.ts"]
tickets: ["ESC-295"]
decisions: []
compared: null
---

# Les annonces du store sur Discord

Détail, exemples de chaque message en fr et en : `docs/STORE.md` §4.7 (il fait foi). Mise en route : `store/README.md`, « Les annonces sur Discord ».

## Les salons
- **Le salon des admins**, privé (rôle admin), en français, webhook `STORE_DISCORD_ADMIN_WEBHOOK` : une version publiée qui demande la revue (auteur, ce qui a grandi depuis la dernière vérifiée, lien vers `/review`, commande `npm run plugin-review -- <id>@<version>` à copier), un signalement (motif et nombre d'ouverts, jamais son texte ni qui l'a fait), un nom trop proche, une étiquette déplacée, un retrait d'urgence ou un badge retiré (avec le n° de la liste de révocation).
- **Le salon public**, en français (`STORE_DISCORD_PUBLIC_WEBHOOK_FR`) et en anglais (`STORE_DISCORD_PUBLIC_WEBHOOK_EN`), un message par langue : un nouveau plugin (« Non vérifié » tant qu'il ne l'est pas), une nouvelle version, « Vérifié par Escouade » (avec le résumé public de la revue), un plugin retiré (la raison publique de la liste de révocation seule). Rien au public d'un plugin que son auteur a retiré de la liste, ni d'un retrait `--quiet`.
- Les trois variables sont facultatives : sans l'une, son salon ne reçoit rien et le démarrage le dit une fois. Seule une adresse `https://discord.com/api/webhooks/…` (ou `discordapp.com`) est prise.

## L'envoi
- Une tâche pg-boss par message (`store.discord`), écrite en arrière-plan : une soumission, un signalement ou un dépôt de la revue n'attend jamais Discord et n'échoue pas pour lui.
- Un `429` est attendu le temps de son `Retry-After` puis renvoyé ; une attente de plus d'une minute, un `5xx` ou Discord injoignable : la tâche est reprise plus tard (30 s à 10 min, six fois). Un autre `4xx` jette le message et le dit.
- Au plus 5 nouveaux plugins ou nouvelles versions par auteur, par jour (UTC) et par salon public ; au-delà, un seul message qui les résume à minuit (UTC), lié à la page de l'auteur.

## La sécurité
- Ce qui vient d'un auteur est hostile : `allowed_mentions: { parse: [] }` sur chaque message, embeds seuls ; Markdown de Discord échappé, caractères de contrôle et invisibles retirés, liens écrits cassés, longueurs bornées (puis les limites de Discord tenues).
- Les seuls liens mènent aux pages du store (plugin, auteur, `/review`). Rien du README ni du code.
- Une adresse de webhook est un secret : seulement dans Coolify ; la tâche ne porte que le nom du salon, le journal et les erreurs que son hôte.

## La forme
- Un embed sobre par événement, aux couleurs du store, le titre lié à la page du plugin (ou à `/review`), l'auteur par son login GitHub lié à sa page du store.
