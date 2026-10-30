# Les statistiques

Les statistiques montrent ce que tes agents ont consommé : tokens, coût et prompts par jour, par semaine ou par mois, répartis par projet, par modèle, par agent, par ticket, par compte et par harness.

## Ouvrir les statistiques

Clique sur « Stats », en bas de la barre des projets. La page remplace le projet affiché ; un second clic sur « Stats », ou un clic sur la tuile d’un projet, te ramène à lui. Elle n’a pas de raccourci.

![La page Statistiques : tokens, coût global, coût moyen par prompt et prompts sur 14 jours, un graphique de tokens par jour empilés en entrée, cache et sortie, puis les répartitions par projet et par modèle.](/images/stats.jpg)

L’en-tête rappelle ce qui est compté : « Statistiques », puis par exemple « Agents lancés depuis l’app · 4 projets, 12 agents ». Seuls les agents lancés depuis Escouade comptent : ce que tu fais dans Claude Code en dehors de l’app n’y figure pas. La page se rafraîchit d’elle-même quand un tour se termine.

## Choisir la période

Le sélecteur en haut à droite propose « Jour », « Semaine » et « Mois ». Ton choix est gardé d’un lancement à l’autre.

| Choix | Étapes du graphique | Comparée à |
|---|---|---|
| Jour | Les 14 derniers jours, aujourd’hui compris (« 27/09 ») | Les 14 jours d’avant |
| Semaine | Les 12 dernières semaines, du lundi au dimanche (« S39 », le numéro de semaine ISO) | Les 12 semaines d’avant |
| Mois | Les 12 derniers mois (« oct. », « sept. »…) | Les 12 mois d’avant |

La dernière étape est en cours : aujourd’hui, la semaine ou le mois courant.

## Les indicateurs

Quatre cartes résument la période :

| Carte | Valeur | Ligne du dessous |
|---|---|---|
| « Tokens » | Les tokens de la période : entrée, cache et sortie additionnés. | « 14 derniers jours · +12 % » : l’évolution par rapport à la période précédente. Sans rien à comparer, seule la durée est écrite. |
| « Coût global » | Le coût de la période. | Le coût de tous les temps depuis le premier tour enregistré, par exemple « 412,80 $ depuis le 14 août 2025 » (ou « 412,80 $ au total »). |
| « Coût moyen / prompt » | Le coût de la période divisé par son nombre de prompts. | « ≈ 206,6 k tokens / prompt ». Sans prompt, « — » et « aucun prompt sur la période ». |
| « Prompts » | Le nombre de prompts de la période. | « 10 par jour en moyenne » (par semaine, par mois). |

Un prompt est un message remis à un agent.

## Le graphique

Le graphique « Tokens par jour » (« par semaine », « par mois ») a une barre par étape. Chaque barre empile trois séries, rappelées en légende :

| Série | Ce qu’elle compte |
|---|---|
| « Entrée » | Les tokens envoyés au modèle, hors cache. |
| « Cache » | Les tokens lus dans le cache de prompts ou écrits dedans. |
| « Sortie » | Les tokens que le modèle a produits. |

Survole une barre pour lire le détail de l’étape : les trois séries, le total, le coût et le nombre de prompts. Le bouton « Tableau » remplace le graphique par un tableau des mêmes chiffres (« Période », « Entrée », « Cache », « Sortie », « Total », « Coût », « Prompts ») ; « Graphique » le ramène.

## Les répartitions

Sous le graphique, la période est découpée de plusieurs façons.

### Par projet et par modèle

Deux listes côte à côte, classées par tokens. Chaque ligne a un nom (la couleur du projet devant son nom), une barre proportionnelle à la ligne la plus lourde, les tokens et le coût. Tant que son historique n’a pas été rouvert, un projet fermé depuis apparaît comme « Projet fermé ».

Les chiffres d’un projet fermé sont gardés avec les autres. Ajoute de nouveau le projet dans son dossier (voir [Projets fermés](settings-projects#projets-fermes)) : il retrouve le même identifiant, donc son nom, ses agents et tous ses chiffres. Avec « Partir d’un projet vierge », le projet neuf repart de zéro et l’ancien reste « Projet fermé ».

### Par compte

Avec plusieurs comptes, le tableau « Par compte » met les comptes côte à côte, du plus cher au moins cher, avec les colonnes « Compte », « Tokens », « Coût », « Tours » et « Agents ». Un tour est une réponse de Claude ; « Agents » compte les agents distincts qui ont tourné sur le compte pendant la période. Un compte supprimé depuis reste listé sous le nom « Compte supprimé (pro) » (son identifiant entre parenthèses).

### Par harness

Avec des comptes ou des tours de plusieurs harness, le tableau « Par harness » met les harness côte à côte, avec les colonnes « Harness », « Tokens », « Coût », « Tours » et « Agents ». Le harness d’un tour est celui qui l’a fait, même si l’agent en a changé depuis. Le coût de chaque harness est écrit comme il le donne : exact, estimé, ou absent (voir « Coût estimé » plus bas).

### Par agent

Le tableau « Par agent » liste les agents de la période, du plus cher au moins cher, avec leur projet, leurs tokens et leur coût. Il compte aussi les agents archivés ; un agent supprimé garde sa ligne, sous « Agent supprimé ».

### Par ticket

Le tableau « Par ticket » additionne ce que les agents de chaque ticket ont coûté : « Ticket », « Titre », « Boucles » et « Coût sur la période », du plus cher au moins cher. Sans ticket, « Aucun ticket sur la période. »

Les listes d’agents et de tickets montrent 20 lignes. « Tout voir » déplie le reste, « Réduire » le replie, et « 20 sur 57 » dit où tu en es.

### Index du code

Quand un projet a allumé l’[index du code](code-index), le tableau « Index du code » dit, par projet et sur la période, combien de réponses l’index a données et ce qu’elles pèsent en tokens. Sa dernière colonne, « Lecture de fichiers évitée (estimation haute) », compte les fichiers d’où viennent les réponses comme si l’agent les avait lus entiers : ce n’est pas une économie mesurée, et l’infobulle le dit. Il vaut pour tous les comptes et tous les harness, quel que soit le filtre.

## Filtrer par harness et par compte

Avec plusieurs harness (des comptes de plusieurs, ou des tours enregistrés sur plusieurs), un menu « Harness » apparaît en haut : « Tous les harness », puis chacun. Choisir un harness restreint à lui les indicateurs, le graphique, les listes et les prompts ; le tableau « Par harness » reste complet, avec sa ligne mise en avant. Le menu « Compte » se combine avec lui : il ne propose plus que les comptes de ce harness, et le tableau « Par compte » ne liste qu’eux.

Avec plusieurs comptes (ou des tours enregistrés sur plusieurs), un menu « Compte » apparaît à gauche du sélecteur de période. Il propose « Tous les comptes », chaque compte, et les comptes supprimés que les statistiques connaissent encore.

Choisir un compte restreint à lui les indicateurs, le graphique et les listes par projet, par modèle, par agent et par ticket, ainsi que la comparaison avec la période précédente. Le tableau « Par compte » reste complet, avec la ligne du compte choisi mise en avant. Ce choix n’est pas gardé : la page s’ouvre toujours sur « Tous les comptes ».

Pour les comptes eux-mêmes, voir [Plusieurs comptes Claude](accounts).

## Coût estimé

Chaque harness dit comment il donne le coût d’un tour. Claude Code le donne exactement, Copilot CLI aussi : ses crédits d’IA (un crédit, 0,01 $), sinon les prix par token de ses modèles. Codex ne donne que ses tokens : le coût de ses tours est estimé d’après le tarif public des modèles d’OpenAI, dans la conversation (« ≈ 0,02 $ estimé ») comme dans les statistiques. Avec un abonnement ChatGPT, c’est un coût théorique, comme celui de Claude avec un abonnement Max. Un harness qui ne donne que ses tokens a un coût calculé par Escouade d’après le tarif public de ses modèles : il est marqué « ≈ » et « estimé », et une infobulle le dit. Un harness qui ne donne aucun coût a « — » à la place. Dès qu’un harness estimé compte dans ce qui est affiché, les totaux sont marqués « estimé » eux aussi.

## D’où viennent les chiffres

Les chiffres sont dans `stats.db`, une base SQLite du dossier de données d’Escouade (`~/.escouade/`, voir [les données locales](data)).

- À la fin de chaque tour, Escouade y écrit une ligne par modèle utilisé : la date, l’agent, le projet, le modèle, les tokens d’entrée, de cache et de sortie, le coût et le compte. Le coût est le chiffre exact de Claude Code : l’estimation « ≈ » de la barre de statut n’y entre pas (voir [Quotas et coût](usage)).
- Chaque message remis à un agent est noté comme un prompt, avec sa date, l’agent, le projet et le compte.
- Le contenu des conversations n’y est pas.
- Le compte d’un tour est celui qu’avait l’agent à ce moment-là : un agent repris sur un autre compte compte ensuite sur celui-là.
- Archiver ou supprimer un agent, ou fermer un projet, n’efface pas ses lignes : elles restent dans les totaux.

Quand les statistiques ne peuvent pas être lues, la page écrit « Statistiques indisponibles : » suivi de l’erreur.
