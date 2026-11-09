# Utiliser Discord

Une fois l’adresse du webhook gardée, il n’y a plus rien à faire : Escouade poste dans le salon quand l’un de ces deux faits arrive, dans un projet où le plugin est actif.

## Ce qu’il poste

| Quand | Le message |
| --- | --- |
| Un ticket arrive dans la colonne « À tester » | `🧪 **APP-7** to test: Le panier (Boutique)` |
| Un agent attend ta réponse (une question, une autorisation) | `✋ **Mila** waits for you (Boutique)` |

La clé et le titre du ticket, le nom de l’agent et celui du projet sont ceux d’Escouade au moment du fait. Le reste du texte est en anglais, quelle que soit la langue de l’interface : il est écrit dans le manifeste du plugin.

Les autres déplacements (vers « En cours », « Terminé »…) et les autres statuts d’un agent (au travail, fini, en erreur) ne postent rien.

## Les mentions

Les mentions sont coupées (`allowed_mentions` vide) : un `@everyone` ou un `@here` dans le titre d’un ticket, venu par exemple d’une issue publique, s’affiche tel quel sans notifier le salon.

## Ses limites

- **Un seul salon** pour tous les projets : le plugin n’a qu’une adresse de webhook. Coupe dans l’onglet **Projets** ceux que tu ne veux pas y voir.
- **Pas de nouvel essai** : un envoi qui échoue (Discord ne répond pas en 10 secondes, refuse, ou la machine est hors ligne) est perdu. Il est écrit dans son journal.
- **Pas de rattrapage** : Escouade fermé, rien n’est envoyé, et ce qui s’est passé entre-temps ne l’est pas après.
- **Une file de 256 faits** par plugin : si Discord est très lent et que les faits s’accumulent, les plus anciens sont abandonnés, et le journal le dit.
- **Pas de lien** vers le ticket ni vers l’agent : le message dit lequel, à toi de l’ouvrir dans Escouade.
- Le texte des messages ne se règle pas : il est celui du manifeste.

## Le journal

Chaque envoi est écrit dans le journal du plugin, avec la réponse de Discord : `webhook to discord.com: 204 No Content (ticket.column #42)`. L’adresse du webhook n’y apparaît pas.

Le plugin n’ayant pas de programme, l’onglet **Journal** de sa page ne montre pas ces lignes (« Un plugin sans programme n’a pas de journal »). Elles sont dans le fichier `~/.escouade/plugin-logs/dev.escouade.discord.log`.
