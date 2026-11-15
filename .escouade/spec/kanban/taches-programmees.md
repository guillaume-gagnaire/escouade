---
title: "Tâches programmées"
subject: "kanban"
sources: ["docs/OVERSEER.md","design/overseer/programmees-barre.png","design/overseer/programmees-formulaire.png","src-tauri/src/scheduled.rs","src-tauri/src/mcp/scheduled.rs","src/components/ScheduledSection.svelte"]
tickets: ["ESC-448"]
decisions: [168]
compared: null
---

## Ce que c'est

Un ticket d'agent qui revient à heure fixe, projet par projet : une revue des dépendances chaque lundi, un rapport chaque soir. Une fonction d'Escouade à part entière : l'Overseer et le chef d'escouade la pilotent par le serveur MCP, l'utilisateur par sa section de la barre latérale. Elles s'appellent « programmées » pour laisser « Planifier » au bouton de la spec (voir *Planifier*).

## Une tâche

- Comme un ticket : un titre, une consigne (le prompt : la description de ses tickets), des critères, des boucles max, un harness, un compte, un modèle, un effort.
- Un rythme, à une heure : chaque jour ; chaque semaine, aux jours choisis (lundi à dimanche) ; chaque mois, à un jour du mois (1 à 31 ; un mois sans ce jour est sauté) ; ou une expression cron de cinq champs (minute, heure, jour du mois, mois, jour de la semaine), sans secondes ni années. Les rythmes simples sont écrits en cron ; la lecture est celle de la crate `croner`.
- L'heure est celle de la machine, heure d'été comprise : une tâche de 7:00 part à 7:00 avant comme après le changement d'heure.
- Refusées : un titre vide, un rythme sans jour, une heure hors de 00:00–23:59, une expression illisible, un rythme qui ne tombe jamais (« 0 0 31 2 * »), un compte que le projet ne peut pas utiliser.
- **Sur la machine seulement** : gardées dans l'état d'Escouade, jamais partagées par le dépôt (partagées, chaque membre de l'équipe les lancerait). Leurs tickets ne sont pas partagés non plus. Un projet fermé garde ses tâches pour son retour ; effacé, il les emporte.

## À chaque échéance

- Escouade crée un ticket depuis la tâche (titre, consigne, critères, boucles, harness, compte, modèle, effort) et le lance comme « Lancer » : il suit le Kanban comme un ticket lancé à la main, dans ses places et ses quotas, même pilote auto éteint ; une place prise le laisse « À faire » jusqu'à ce qu'elle se libère.
- Son ticket porte la tâche (`scheduled`) : sa carte dit « ⏱ programmée », un lien qui ouvre la tâche (grisé si la tâche a été supprimée depuis), et la carte de son agent dans la barre latérale le dit aussi.
- **Jamais en double** : la dernière échéance traitée est écrite, et enregistrée, avant que le ticket soit créé ; un arrêt entre les deux perd cette exécution, n'en fait jamais deux. Une seule passe à la fois.
- **Une seule à la fois** : une échéance passe son tour tant que le ticket de l'exécution précédente est « À faire » ou « En cours ». « À tester », « En revue » et « Terminé » comptent comme fini, un ticket supprimé aussi.
- **Le rattrapage, tâche par tâche** : une échéance tombée pendant qu'Escouade était fermé part à son retour si « Lancer au retour d'Escouade, une seule fois » est coché (par défaut), sinon elle attend la suivante. Plusieurs échéances manquées ne donnent qu'une exécution, pour la dernière d'entre elles.
- En pause, les échéances passent sans ticket ; reprise, la tâche repart à la prochaine échéance, jamais une de la pause. Un nouveau rythme compte à partir de maintenant.
- Une échéance qui ne crée pas de ticket le dit sur la tâche (`lastNote`) : « passé son tour », « Escouade était fermé », ou pourquoi le ticket n'a pas pu être créé.
- Les échéances sont regardées toutes les 15 s. L'horloge des tâches est celle d'Escouade : les tests la règlent (en debug, `ESCOUADE_SCHEDULE_SPEED` l'accélère et `ESCOUADE_SCHEDULE_TICK_MS` règle la fréquence, pour l'e2e).

## Dans la barre latérale (programmees-barre.png, place A)

- Une section « Programmées » au-dessus de « Lancement », sans encadré, comme « Lancement » et « Terminaux », avec l'icône d'horloge, le nombre de tâches et « ＋ » (« Programmer une tâche »). Ses lignes ont une hauteur limitée et défilent ; dans une fenêtre basse, elle se replie à son en-tête comme les autres.
- Chaque ligne : le titre ; le rythme en mots (« Chaque lundi 7:00 », « Chaque jour 23:30 », « Le 1er du mois 9:00 », « Du lundi au vendredi 7:00 », « Cron 0 7 * * 1 ») ; puis la prochaine échéance (« ce soir », « aujourd'hui 14:00 », « demain 7:00 », « lun. 9 nov. 7:00 »), ou « en cours (DEM-44) », un lien vers le ticket (la conversation de son agent, sinon le Kanban), ou « en pause » (ligne grisée) ; à droite, un interrupteur de pause.
- Un clic sur la ligne ouvre la tâche ; un clic droit : « Modifier », « Mettre en pause / Reprendre », « Supprimer » (confirmé : « Les tickets qu'elle a créés restent sur le Kanban. »).
- Sans tâche : « Aucune tâche programmée : une revue chaque lundi, un rapport chaque soir… ».

## Le formulaire « Programmer » (programmees-formulaire.png)

- Le formulaire du ticket, titré « Nouvelle tâche programmée » ou « Modifier la tâche programmée ». Sous le titre, libellés à gauche : « Rythme » (Chaque jour · Chaque semaine · Chaque mois · Cron), « Jours » (L M M J V S D), « Jour du mois », « Heure » ou « Expression » (cron, avec l'aide de ses cinq champs) ; « Escouade fermé à l'heure dite » : « Lancer au retour d'Escouade, une seule fois même si plusieurs échéances ont été manquées » ; « Jamais deux à la fois : si la précédente tourne encore, l'échéance passe son tour. » ; « Prochaines : » les trois prochaines échéances (la première en gras) et l'expression cron.
- Puis la consigne (« Consigne »), les critères et les boucles, le harness, le compte, le modèle et l'effort, comme un ticket. Ni branche, ni « Après », ni pièces jointes, ni partage.
- Une nouvelle tâche part sur « Chaque semaine », le lundi, à 07:00. Les prochaines échéances viennent d'Escouade ; un rythme refusé dit pourquoi et grise « Programmer » (« Enregistrer » pour une tâche modifiée).

## Serveur MCP

`list_scheduled_tasks`, `create_scheduled_task`, `update_scheduled_task`, `pause_scheduled_task` et `delete_scheduled_task` (voir *Serveur MCP*). Une tâche se nomme dans son projet, par son id ou son titre ; les outils suivent les règles de projet privé des tickets.

## Limites

- Pas de déclenchement sur un événement : un horaire seulement.
- L'heure de la machine : deux machines dans deux fuseaux lancent à deux heures différentes ; les tâches n'étant pas partagées, ce n'est jamais la même tâche.
