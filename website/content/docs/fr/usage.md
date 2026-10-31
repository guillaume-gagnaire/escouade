# Quotas et coût

La barre de statut, en bas de la fenêtre, montre combien d’agents travaillent, où en sont les quotas de ton abonnement Claude et ce que tu as dépensé aujourd’hui. Cette page explique ce que chaque chiffre veut dire, d’où il vient et ce qu’Escouade fait quand un quota approche de sa limite.

## La barre de statut

La barre occupe le bas de la fenêtre. De gauche à droite :

| Élément | Ce qu’il montre |
|---|---|
| « 3 actifs » | Les agents qui travaillent, tous projets confondus. Les agents archivés ne comptent pas. |
| « 1 en attente » | Les agents qui attendent ta réponse. Un clic, ou `Ctrl+J`, met le focus sur la carte du prochain agent qui attend ou qui est à voir. Dès qu’il y en a, le compteur passe en ambre et son point pulse. |
| « ✓ 2 terminés » | Les agents dont la tâche est terminée. |
| « 4 Claude · 1,2 Go · 6 % CPU » | Les processus Claude en cours, avec la mémoire et le processeur qu’ils prennent. Absent quand aucun ne tourne. L’infobulle liste chaque agent, le plus gourmand en mémoire d’abord. |
| Quotas | Les deux fenêtres de quota du compte en cours : voir la section suivante. |
| « Aujourd’hui 4,12 $ » | Ce que les agents ont coûté depuis minuit : voir la section « Le coût ». |
| « ⎇ main ↑1 ↓3 » | La branche du projet affiché, quand c’est un dépôt git. Un clic ouvre le sélecteur de branches ([les branches](branches)). |
| À droite | L’état d’une [mise à jour](updates), « ♪ On » ou « ♪ Off » pour le son des [notifications](notifications), et ⚙ pour les réglages (`Ctrl+,`). |

> [!NOTE]
> Sous macOS, `Ctrl` devient `⌘` dans les raccourcis (sauf `Ctrl+Tab`).

## Les quotas

### Lire les barres

Escouade suit les deux fenêtres de quota de ton abonnement Claude : la fenêtre de 5 heures et la fenêtre hebdomadaire. Chacune s’écrit en compact, un libellé, une barre et un décompte :

`5h … reset 3h01 | 7j … reset 4j 12h`

- « 5h » est la fenêtre de 5 heures, « 7j » la fenêtre hebdomadaire (7 jours).
- La barre se remplit avec la part du quota utilisée. Elle passe à l’ambre au-delà de 80 %.
- « reset 3h01 » est le temps qu’il reste avant la remise à zéro de la fenêtre : 3 heures et 1 minute. À partir d’un jour, le décompte s’écrit « 4j 12h ». Le mot « reset » est le même dans les deux langues.
- Le pourcentage n’est pas écrit : il apparaît au survol du libellé, de la barre ou du décompte, avec la date de remise à zéro. Par exemple : « 42 % · remise à zéro le 10/10 à 18:00 ».
- Avec un seul compte, `Tab` donne le focus à chaque barre, que les lecteurs d’écran annoncent « Quota sur 5 heures » ou « Quota sur 7 jours » avec la même valeur.
- Tant qu’Escouade n’a rien lu, la barre est vide, un « — » remplace le décompte et l’infobulle dit « Quota indisponible ».

### Le quota d’un modèle

Sur certains forfaits, Claude compte un modèle à part : la semaine de Fable, par exemple, a son propre plafond et sa propre remise à zéro. Escouade lui donne une barre de plus, à côté des autres et sous le nom que donne Anthropic : « Fable · semaine ». Elle se lit comme les autres (infobulle, lecteurs d’écran : « Quota sur 7 jours de Fable »), dans la barre de statut, le panneau des quotas et Réglages › Comptes.

Ce quota ne retient que les agents et les tickets de ce modèle. Quand la semaine de Fable passe le seuil, un ticket Fable part sur un autre compte ou attend, et les tickets Opus ou Sonnet continuent sur le même compte. Le pilote auto n’est pas mis en pause pour autant. Un agent Fable arrêté par sa limite attend sa remise à zéro sans bloquer les autres modèles. Escouade reconnaît le modèle par son nom (« fable », « claude-fable-5-1 »…), quel que soit le nom qu’on lui donne.

Les barres suivent ce qu’annonce le serveur : un forfait qui n’a pas de fenêtre de 5 heures (certains forfaits Max) n’a pas de barre « 5h », et une fenêtre qui disparaît du forfait disparaît de la barre à la lecture suivante.

### Avec plusieurs comptes

Dès qu’il y a plus d’un compte dans [la liste des comptes](accounts), la barre écrit le nom du compte en cours devant les deux quotas : c’est le compte sur lequel partirait un nouvel agent maintenant, sans compter les comptes réservés à des projets.

Le groupe devient alors un bouton, « Quotas par compte (compte en cours : Pro) ». Un clic (ou `Entrée` au clavier) ouvre le panneau « Quotas des comptes Claude » au-dessus de la barre. `Échap` ou un clic à côté le ferme. Il montre, pour chaque compte, le compte en cours d’abord, puis les autres dans l’ordre des réglages :

- son nom, avec l’étiquette « en cours » pour le compte en cours et « au-delà du seuil de pause » quand une de ses fenêtres a passé « Pause au-delà du quota » ;
- ce que ses tours ont coûté aujourd’hui, par exemple « Aujourd’hui : 1,20 $ » (avec « ≈ » tant qu’un tour tourne) ;
- ses deux fenêtres, avec cette fois le pourcentage écrit à côté de la barre ;
- en grisé, la raison quand le compte n’est pas pleinement lu : « Inactif », « Pas connecté », « Connexion expirée : relance Claude Code pour ce compte. ».

### D’où viennent les quotas

Escouade relit les quotas de chaque compte toutes les minutes, une première fois deux secondes après le démarrage. Les chiffres viennent de deux sources :

- **Un processus Claude du compte**, quand un agent du compte tourne : Escouade l’interroge, et Claude Code annonce aussi lui-même l’état des fenêtres pendant un tour. C’est la source la plus directe.
- **Le service d’usage d’Anthropic**, pour un compte actif sur lequel aucun agent ne tourne. Escouade l’interroge avec la connexion que Claude Code garde pour le compte, qu’elle lit sans la modifier, et au plus une fois toutes les 5 minutes par compte. La requête passe par le proxy des [réglages réseau](settings-network) s’il y en a un.

Quand un compte vient d’être connecté, son quota est lu tout de suite.

### Quand un quota n’est pas lu

| Ce que tu vois | Ce que ça veut dire |
|---|---|
| « Quota indisponible », barre vide, « — » | Escouade n’a encore rien lu pour ce compte, ou la lecture a échoué. |
| « Pas connecté » | Escouade ne trouve pas de connexion claude.ai pour ce compte. Connecte-le depuis [les comptes](accounts). Pour Principal : « Pas connecté par un compte claude.ai (clé d’API ?) ». |
| « Connexion expirée : relance Claude Code pour ce compte. » | La connexion est périmée. Claude Code en obtient une nouvelle quand il tourne ; « Se connecter… » dans les réglages des comptes la renouvelle. |
| « Inactif » | Le compte est désactivé. Son quota n’est plus lu que par un agent qui tourne dessus. |
| Des chiffres qui ne bougent plus | Une lecture qui échoue (réseau, serveur, proxy) laisse les dernières valeurs en place. |

Pour un compte « Pas connecté » ou « Connexion expirée », les dernières valeurs lues restent affichées jusqu’à la fin de leur fenêtre.

> [!NOTE]
> Sous macOS, Claude Code garde la connexion dans le trousseau, et macOS peut te demander d’autoriser Escouade à la lire. Si tu refuses, Escouade ne redemande pas avant 10 minutes.

## Le coût

Les montants sont en dollars américains.

### Aujourd’hui

« Aujourd’hui » est la somme des tours terminés depuis minuit, heure de ton ordinateur, sur tous les comptes, y compris ceux que tu as supprimés depuis. Elle se met à jour à la fin de chaque tour. Avec plusieurs comptes, le panneau des quotas donne le même total compte par compte.

### En direct pendant un tour

Tant que Claude travaille, les tokens et le coût montent en direct. Claude Code ne donne le coût exact d’un tour qu’à sa fin ; en attendant, Escouade l’estime d’après les tarifs publics des modèles (entrée, sortie, cache lu et cache écrit). Une estimation s’écrit avec « ≈ » devant le montant, et l’infobulle l’explique : « Estimation (tarifs publics) pendant que Claude travaille ; coût exact à la fin du tour ».

À la fin du tour, le chiffre exact de Claude Code remplace l’estimation. Pour un modèle dont Escouade ne connaît pas le tarif, les tokens montent mais le coût n’apparaît qu’à la fin du tour.

### Où voir les chiffres d’un agent

| Où | Ce qui s’affiche |
|---|---|
| Carte de l’agent, dans la barre latérale | Ses tokens (« tok »), son coût et le nombre de fichiers modifiés. |
| En-tête de la conversation | « Tokens » et « Coût », avec « Contexte » et « Durée ». |
| Fin d’un tour | La carte « Tâche terminée » (durée, tokens, coût, fichiers modifiés), ou une ligne durée · tokens · coût entre deux tours. |
| Carte d’un ticket du Kanban | Le coût de son agent. |
| [Vue d’ensemble](overview) | Le coût de chaque agent. |

Les [statistiques](stats) cumulent tout cela par jour, par semaine ou par mois.

## Quand un quota approche de sa limite

Escouade réagit de trois façons. Les deux premières dépendent du seuil « Pause au-delà du quota », que tu règles dans Réglages › Kanban, groupe « Pilote auto » : 80, 90, 95 ou 100 % d’une fenêtre, 100 % par défaut, le même pour tous les projets ([les réglages du Kanban](settings-board)). Un compte est au seuil quand l’une de ses fenêtres l’atteint et que sa remise à zéro est encore à venir.

1. **Les nouveaux agents changent de compte.** Quand le compte en cours passe le seuil et qu’un autre est disponible, les nouveaux agents et les tickets partent sur l’autre. Une notification le dit : « Le compte Principal a atteint 91 % : les nouveaux agents partent sur Pro. » ([Plusieurs comptes Claude](accounts)).
2. **Le pilote auto se met en pause.** Quand tous les comptes qu’un ticket peut utiliser sont au seuil, aucun ticket ne démarre. Le Kanban écrit alors, sous son en-tête, une ligne comme « Pilote auto en pause : quota de 5 h à 92 % (reprise à 15:00) », avec le nom des comptes concernés quand il y en a plusieurs, et le bouton « Reprendre maintenant » pour passer outre. La pause se lève seule à la remise à zéro. Quand l’agent d’un ticket bute sur la limite d’usage sans reprise prévue (reprise automatique éteinte, ou heure de remise à zéro inconnue), aucun ticket ne démarre non plus pendant 30 minutes sur son compte. Voir [le pilote auto](autopilot).
3. **Un agent atteint la limite en plein tour.** Il s’arrête et reprend tout seul après la remise à zéro, ou sur un autre compte si tu le demandes. Voir [Reprendre un agent arrêté par la limite d’usage](accounts#reprendre-un-agent-arrete-par-la-limite-d-usage).
