# Utiliser Slack

Une fois le jeton gardé et le bot invité dans les salons, il n’y a plus rien à faire : le plugin poste dans le salon du projet quand l’un de ces deux faits arrive, dans un projet où il est actif.

## Ce qu’il poste

| Quand | Le message, dans `#escouade-<projet>` |
| --- | --- |
| Un ticket arrive dans la colonne « À tester » | 🧪 **APP-7** to test: Le panier |
| Un agent attend ta réponse (une question, une autorisation) | ✋ **Mila** waits for you |

La clé et le titre du ticket et le nom de l’agent sont ceux d’Escouade au moment du fait ; le projet est dit par le salon. Le reste du texte est en anglais, quelle que soit la langue de l’interface : il est écrit dans son code.

Les autres déplacements (vers « En cours », « Terminé »…) et les autres statuts d’un agent ne postent rien. Un fait sans projet non plus.

## Les mentions

La clé, le titre et le nom sont échappés comme Slack le demande (`&`, `<`, `>`) : un `<!channel>` ou un `<@U…>` dans le titre d’un ticket, venu par exemple d’une issue publique, s’affiche tel quel sans notifier personne.

## Quand il tourne

Son programme démarre au premier fait qu’il reçoit, dans Escouade, en WASM. Il demande le jeton à Escouade à chaque fait, et ne le garde pas.

## Ses limites

- **Un salon par projet**, par son nom : le bot doit y être invité. Le salon n’est pas créé pour toi.
- **Pas de nouvel essai** : un message que Slack refuse, ou qui ne part pas (Slack ne répond pas en 10 secondes, la machine est hors ligne), est perdu. Le journal le dit.
- **Pas de rattrapage** : Escouade fermé, rien n’est envoyé, et ce qui s’est passé entre-temps ne l’est pas après.
- **Une file de 256 faits** par plugin : si les faits s’accumulent, les plus anciens sont abandonnés, et le journal le dit.
- **Pas de lien** vers le ticket ni vers l’agent : le message dit lequel, à toi de l’ouvrir dans Escouade.
- Le texte des messages ne se règle pas.

## Le journal

L’onglet **Journal** de sa page montre chaque message posté (`posted in #escouade-boutique`) et chaque refus, relu toutes les 2 secondes. Le fichier entier est `~/.escouade/plugin-logs/dev.escouade.slack.log`.
