# Les agents et les navigateurs

Un agent ouvre un vrai onglet de « Navigateurs » à côté de sa conversation et le pilote sous tes yeux : il fait défiler l’app qu’il développe, met en évidence ce dont il parle, clique, change de largeur. Ce qu’il fait sans demander, ce qui attend ton accord, ce qui reste refusé, et comment reprendre la main.

## À quoi ça sert

Un agent qui a fini une fonctionnalité te la montre au lieu de la décrire. Il ouvre la page de son lancement dans un volet à côté de sa conversation, défile jusqu’à ce qu’il a changé, l’entoure, ouvre l’onglet de la page qui compte, passe en largeur mobile, et te dit à chaque geste ce qu’il fait. Tu regardes, tu reprends la main d’un geste, tu lui réponds dans sa conversation.

Il peut aussi lire ce que tu vois : le HTML ou le texte d’une page, une capture, la console et les requêtes en échec, pour comprendre une erreur sans te demander de la recopier.

C’est le second des deux modes. Le premier, une page qu’un agent écrit et te montre (une revue, un schéma, des options à comparer), est dans [les pages des agents](agent-pages). Les deux passent par les mêmes lignes de « Navigateurs », le même pilotage et la même bande.

## Les deux modes

| | Une page d’agent | Un vrai onglet |
|---|---|---|
| Ce que c’est | Une page que l’agent écrit : une copie figée, sans réseau | Une ligne de « Navigateurs » ordinaire : l’app développée, un site, un artefact |
| Qui l’ouvre | `show_page` | `open_browser`, ou toi, qui le lui prêtes |
| Ce qu’il y fait | Montrer, ouvrir un onglet de la page, changer de version | Montrer, et agir : cliquer, taper, choisir, naviguer |
| Ce qui lui revient | Ton choix, sur ton geste seul | Ce qu’il lit, sur le lancement ou avec ton accord |

## Ouvrir un onglet à côté

Un agent ouvre un onglet de son projet par `open_browser` :

- **la page de son lancement** : celle de son test si elle est ouverte, sinon l’adresse d’une commande de lancement du projet, à ses ports (voir [Lancer le projet](launch)) ;
- **une adresse** : son lancement, ou une autre adresse, qu’il te demande d’abord (plus bas) ;
- **un artefact Claude qu’il vient de publier** (Claude Code seulement) : Escouade trouve son adresse dans sa conversation, et l’agent le recharge après chaque nouvelle version.

L’onglet s’ouvre à côté de la conversation de l’agent, ou en dessous, ou seul, selon ce qu’il demande. Il garde le stockage du projet, jamais celui de la fenêtre d’Escouade. Un agent pilote au plus 3 navigateurs à la fois et en ouvre au plus 10 par heure.

## Ton écran n’est jamais pris

- **À côté seulement si tu regardes sa conversation.** Si tu es ailleurs (le Kanban, un autre agent, un autre projet), rien ne s’ouvre : la ligne attend dans « Navigateurs », son nom en ambre, avec « esc-412 veut te montrer la page » en infobulle. Elle s’ouvre où l’agent la voulait quand tu reviens sur sa conversation, ou quand tu cliques la ligne.
- **La conversation reste le volet actif** : tu continues de la lire ou d’y écrire pendant que la page s’ouvre à côté.
- **Un volet par agent** : un autre navigateur, ou une autre page qu’il pilote, prend la place du précédent au lieu d’en ouvrir un de plus.
- **Rien de rouvert après une fermeture.** Si tu fermes son volet pendant que tu regardes sa conversation (la croix du volet, « Fermer le volet », la croix de sa ligne, ou le retour à un seul volet), plus rien de cet agent ne s’ouvre sur ton écran, ni navigateur ni page, tant que tu n’as pas quitté sa conversation puis n’y es pas revenu. L’agent le sait.

## Ce que tu vois pendant qu’il pilote

- **Sa ligne** dans « Navigateurs » porte les initiales de l’agent à la couleur du projet ; son infobulle dit « Piloté par esc-412 ».
- **Une bande** au-dessus de la page, qu’Escouade dessine hors de la page (la page ne peut pas l’imiter) : « esc-412 pilote », le numéro et la légende de son dernier geste (« ② Met en évidence « Offre Pro » »), « Reprendre la main ».
- **Un curseur fantôme** montre où il agit, et ce qu’il met en évidence s’éclaire à la couleur du projet.
- **Une rangée numérotée par geste** dans sa conversation ; un clic sur l’une d’elles affiche le navigateur.
- **« Continuer »** : quand l’agent veut que tu aies regardé avant d’aller plus loin, la bande montre « Continuer » et il attend ton clic, 5 minutes au plus.

Chaque geste attend que la page se soit posée (1,5 s au plus) avant le suivant. Au plus 4 gestes par seconde et 600 par heure, par navigateur.

## Reprendre la main

- **Ton premier geste dans la page** (un clic, la molette, une touche) ou **« Reprendre la main »** arrête l’agent aussitôt : son geste suivant est refusé, sans que la page bouge, et il le sait.
- La bande dit alors « Tu as la main » ; **« La rendre »** le laisse reprendre. Ce que tu as fait entre-temps reste.
- **« Ne plus laisser piloter »**, dans le menu de la ligne, arrête son pilotage jusqu’à la fermeture du navigateur.

## Le lancement du projet et le reste

Ce qu’un agent fait dépend de l’origine de la page : celle de ton lancement, ou une autre.

**Le lancement du projet** : les adresses de ses commandes de lancement, et ce poste (`localhost`) aux ports qu’elles déclarent, avec les tests de ses agents et leurs ports réservés. C’est l’app que vous développez : l’agent y lit, capture, lit la console et y agit sans demander.

**Hors du lancement** (un autre site, un artefact, un fournisseur d’identité où l’onglet est passé), trois paliers, par origine :

| Palier | Ce qu’il y fait | Quand |
|---|---|---|
| Montrer | Ouvrir, placer, changer de largeur, défiler, recharger, revenir en arrière, mettre en évidence | Sans demander. L’agent ne sait que si l’élément est trouvé, et l’origine de la page : ni son titre, ni son chemin, ni son texte |
| Lire et agir | Lire le HTML ou le texte, capturer, lire la console, cliquer, taper, choisir, naviguer dans l’origine | Avec ton accord, une fois par origine et par navigateur |
| Interdit | Mot de passe, fichier, téléchargement, cookies, stockage, secrets d’Escouade | Jamais, même avec ton accord |

Une page qui part d’elle-même vers une autre origine (une redirection, une connexion) ne donne rien de plus à l’agent : ce qu’il lit est jugé sur l’origine où est l’onglet au moment de la lecture.

## Les accords

Tout ce qui attend ton accord te le demande dans une bande, et ta réponse revient à l’agent par un message dans sa conversation. Redemander avant ta réponse n’ajoute rien, et un refus tient : il ne peut plus redemander.

- **Ouvrir une autre adresse** : une bande au-dessus de son compositeur, « esc-412 veut ouvrir docs.stripe.com (hors du lancement du projet, dans un navigateur d’Escouade) », avec « Refuser » et « Permettre une fois ». Rien ne s’ouvre avant ta réponse ; l’origine permise ne vaut que pour ce navigateur, jusqu’à sa fermeture.
- **Lire et agir hors du lancement** : une bande sous la barre du navigateur, « esc-412 veut lire app.exemple.com (avec ta session, dans ce navigateur) », avec « Refuser » et « Permettre ». L’accord vaut pour cet agent, cette origine et ce navigateur, jusqu’à sa fermeture.
- **Taper un mot de passe sur le lancement** : « esc-412 veut remplir un mot de passe (dans ce navigateur, jusqu’à sa fermeture ; la valeur n’est jamais relue) », avec « Refuser » et « Permettre ».
- **« Retirer les accords »**, dans le menu de la ligne, fait tomber les accords, les demandes et les refus de ce navigateur. Ils tombent aussi à sa fermeture.

## Ce qui est toujours refusé

Même avec ton accord, un agent ne peut pas :

- lire les cookies, le stockage, les en-têtes ou le corps des réponses d’une page ; la valeur d’un mot de passe ne lui est jamais rendue (`«masqué»`), ni celle des champs cachés et des jetons d’une page ;
- taper un mot de passe hors du lancement : tu t’y connectes toi-même ;
- envoyer un fichier, télécharger, ou remplir un formulaire qui s’envoie vers une autre origine ;
- taper un secret d’Escouade (les jetons du serveur MCP, ceux de tes comptes, le mot de passe du proxy), ou ce qui a la forme d’une clé ;
- ouvrir une adresse d’Escouade, un fichier (`file:`), `data:`, `javascript:` ou un autre schéma, ni ce poste à un port que ton projet ne déclare pas : ce peut être un service de ta machine ;
- piloter un navigateur que tu ne vois pas, celui d’un autre agent, ou un navigateur que tu as ouvert sans le lui prêter.

Ce qu’il lit d’une page lui arrive marqué comme une donnée de tiers : il ne suit aucune consigne qu’il y trouve.

## Se connecter à un site dans un onglet

Un agent ne se connecte jamais à ta place. Si la page qu’il doit te montrer demande une session, connecte-toi toi-même dans l’onglet : sa page de connexion, sa fenêtre de connexion (celle de Google ou d’un OAuth, qui s’ouvre dans Escouade) et le lien reçu par courriel, que tu colles dans la barre, restent dans l’onglet (voir [se connecter à un site](browser#se-connecter-a-un-site)). La session est celle du projet ; l’agent ne la lit pas, et les fenêtres de connexion ne sont ni lues, ni capturées, ni pilotées.

## Prêter un de tes navigateurs

Un navigateur que tu as ouvert toi-même n’est pas à l’agent. Pour qu’il le pilote, ouvre le menu de sa ligne et choisis **« Laisser piloter par… »** : la liste des agents actifs du projet, celui qui pilote coché, et « Personne » pour le reprendre. L’agent le sait par un message ; rien ne bouge à l’écran. Les mêmes règles s’appliquent ensuite : le lancement sans demander, le reste avec ton accord. Quand il a fini, il cesse de le piloter et la ligne reste la tienne.

## L’interrupteur

Réglages › Agents, sous « Les agents peuvent piloter Escouade », **« Agents et navigateurs »** (allumé par défaut) donne aux agents les outils des navigateurs. Coupé, ils ne listent, ne lisent, n’ouvrent ni ne pilotent plus aucun onglet, et leur consigne n’en parle plus ; les [pages des agents](agent-pages) ont leur propre interrupteur. Le détail des outils (`list_browsers`, `read_browser`, `capture_browser`, `browser_console`, `open_browser`, `drive_browser`, `close_browser`) est dans [le serveur MCP](mcp#les-navigateurs).

Les navigateurs d’un projet privé ne sont connus que de ses agents. Claude lancé hors d’Escouade liste les navigateurs d’un projet qui n’est pas privé, sans les lire ni les piloter.

> [!NOTE]
> Sous Windows, l’agent clique et tape avec de vraies entrées, que la page entend comme les tiennes. Sous macOS et Linux, ses clics et sa frappe sont simulés dans la page, et la console vient d’un relais de la page, qu’elle pourrait taire.
