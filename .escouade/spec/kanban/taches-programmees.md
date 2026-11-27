---
title: "Tâches programmées"
subject: "kanban"
sources: ["docs/OVERSEER.md","design/overseer/programmees-barre.png","design/overseer/programmees-formulaire.png","src-tauri/src/scheduled.rs","src-tauri/src/mcp/scheduled.rs","src/components/ScheduledSection.svelte","src-tauri/src/inboxviews.rs","Revue de sécurité de la 2.3 (ESC-627), constat SR4-04 (rapport privé)"]
tickets: ["ESC-448","ESC-647"]
decisions: [359,369]
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

## Une tâche programmée par un agent (ESC-647)

Une tâche lance un agent à chaque échéance, pour toujours, et survit à qui l'a posée : c'est une persistance. Un agent détourné par ce qu'il a lu ne doit pas pouvoir s'en servir (constat SR4-04 de la revue de sécurité de la 2.3).

- **Elle attend l'accord de l'utilisateur** : créée ou modifiée par un agent (l'Overseer compris), une tâche naît « en attente » (`awaiting`). Ses échéances passent sans ticket, elle n'a pas de prochaine échéance, jusqu'à ce que l'utilisateur l'accepte. Une pause ou une reprise ne lève pas l'attente ; une modification par l'utilisateur la laisse telle quelle.
- **La demande dans « À toi »** : une ligne « Tâche programmée à accepter », « Claude programme « Maintenance » », qui montre le projet, le rythme (en mots et en cron), la consigne et les critères, avec « Accepter ». La ligne est nommée par ce qu'elle montre : une tâche changée entre-temps (consigne, rythme, compte…) refuse l'accord donné sur l'ancienne, jamais acceptée sans être relue. Aucun agent ne la prend en charge ni ne la coche (`claim_inbox_view`, `settle_inbox_view` refusés).
- **Son auteur reste et se voit** : l'agent, son projet et le ticket sur lequel il travaillait (`author`), gardés après le départ de l'agent, de son ticket et de son worktree. La ligne de la barre latérale dit « par Claude · DEM-3 » (infobulle : « Programmée par l'agent Claude : elle ne lance rien avant ton accord. ») ; `list_scheduled_tasks` le rend (`author`, `awaiting`).
- **Dans son propre projet seulement** : un agent ne crée ni ne modifie une tâche ailleurs que dans son projet, même non privé (« Un agent ne programme des tâches que dans son propre projet… »). L'Overseer, qui travaille pour l'utilisateur sur tous les projets, garde ceux qu'il voit, mais ses tâches attendent l'accord comme les autres.
- **Au plus une fois par quart d'heure** : un agent ne donne pas un rythme dont deux échéances sont à moins de 15 minutes, n'importe où sur ses 500 prochaines (« * * * * * », « */5 * * * * », « 0,10 * * * * », « 0-5 9 * * 1 » refusés ; « */15 * * * * » accepté). Le rythme d'une tâche qu'un agent modifie est vérifié aussi.
- **L'utilisateur n'est pas borné** : sa fenêtre et Claude Code hors d'Escouade (son jeton) créent et modifient sans attente, sans auteur, à tout rythme.

## À chaque échéance

- Escouade crée un ticket depuis la tâche (titre, consigne, critères, boucles, harness, compte, modèle, effort) et le lance comme « Lancer » : il suit le Kanban comme un ticket lancé à la main, dans ses places et ses quotas, même pilote auto éteint ; une place prise le laisse « À faire » jusqu'à ce qu'elle se libère.
- Son ticket porte la tâche (`scheduled`) : sa carte dit « ⏱ programmée », un lien qui ouvre la tâche (grisé si la tâche a été supprimée depuis), et la carte de son agent dans la barre latérale le dit aussi.
- **Jamais en double** : la dernière échéance traitée est écrite, et enregistrée, avant que le ticket soit créé ; un arrêt entre les deux perd cette exécution, n'en fait jamais deux. Une seule passe à la fois.
- **Une seule à la fois** : une échéance passe son tour tant que le ticket de l'exécution précédente est « À faire » ou « En cours ». « À tester », « En revue » et « Terminé » comptent comme fini, un ticket supprimé aussi.
- **Le rattrapage, tâche par tâche** : une échéance tombée pendant qu'Escouade était fermé part à son retour si « Lancer au retour d'Escouade, une seule fois » est coché (par défaut), sinon elle attend la suivante. Plusieurs échéances manquées ne donnent qu'une exécution, pour la dernière d'entre elles.
- En pause, ou en attente de l'accord, les échéances passent sans ticket ; reprise ou acceptée, la tâche repart à la prochaine échéance, jamais une de la pause ou de l'attente. Un nouveau rythme compte à partir de maintenant.
- Une échéance qui ne crée pas de ticket le dit sur la tâche (`lastNote`) : « passé son tour », « Escouade était fermé », ou pourquoi le ticket n'a pas pu être créé.
- Les échéances sont regardées toutes les 15 s. L'horloge des tâches est celle d'Escouade : les tests la règlent (en debug, `ESCOUADE_SCHEDULE_SPEED` l'accélère et `ESCOUADE_SCHEDULE_TICK_MS` règle la fréquence, pour l'e2e).

## Dans la barre latérale (programmees-barre.png, place A)

- Une section « Programmées » au-dessus de « Lancement », sans encadré, comme « Lancement » et « Terminaux », avec l'icône d'horloge, le nombre de tâches et « ＋ » (« Programmer une tâche »). Ses lignes ont une hauteur limitée et défilent ; dans une fenêtre basse, elle se replie à son en-tête comme les autres.
- Chaque ligne : le titre ; le rythme en mots (« Chaque lundi 7:00 », « Chaque jour 23:30 », « Le 1er du mois 9:00 », « Du lundi au vendredi 7:00 », « Cron 0 7 * * 1 ») ; puis la prochaine échéance (« ce soir », « aujourd'hui 14:00 », « demain 7:00 », « lun. 9 nov. 7:00 »), ou « en cours (DEM-44) », un lien vers le ticket (la conversation de son agent, sinon le Kanban), ou « en pause » (ligne grisée), ou « attend ton accord » ; sous elle, l'auteur d'une tâche d'agent (« par Claude · DEM-3 ») ; à droite, un interrupteur de pause, ou « Accepter » tant qu'elle attend l'accord.
- Un clic sur la ligne ouvre la tâche ; un clic droit : « Modifier », « Mettre en pause / Reprendre », « Supprimer » (confirmé : « Les tickets qu'elle a créés restent sur le Kanban. »).
- Sans tâche : « Aucune tâche programmée : une revue chaque lundi, un rapport chaque soir… ».

## Le formulaire « Programmer » (programmees-formulaire.png)

- Le formulaire du ticket, titré « Nouvelle tâche programmée » ou « Modifier la tâche programmée ». Sous le titre, libellés à gauche : « Rythme » (Chaque jour · Chaque semaine · Chaque mois · Cron), « Jours » (L M M J V S D), « Jour du mois », « Heure » ou « Expression » (cron, avec l'aide de ses cinq champs) ; « Escouade fermé à l'heure dite » : « Lancer au retour d'Escouade, une seule fois même si plusieurs échéances ont été manquées » ; « Jamais deux à la fois : si la précédente tourne encore, l'échéance passe son tour. » ; « Prochaines : » les trois prochaines échéances (la première en gras) et l'expression cron.
- Puis la consigne (« Consigne »), les critères et les boucles, le harness, le compte, le modèle et l'effort, comme un ticket. Ni branche, ni « Après », ni pièces jointes, ni partage.
- Une nouvelle tâche part sur « Chaque semaine », le lundi, à 07:00. Les prochaines échéances viennent d'Escouade ; un rythme refusé dit pourquoi et grise « Programmer » (« Enregistrer » pour une tâche modifiée).

## Serveur MCP

`list_scheduled_tasks`, `create_scheduled_task`, `update_scheduled_task`, `pause_scheduled_task` et `delete_scheduled_task` (voir *Serveur MCP*). Une tâche se nomme dans son projet, par son id ou son titre ; les outils suivent les règles de projet privé des tickets, et, pour un agent, celles de *Une tâche programmée par un agent*. Aucun outil n'accepte une tâche : l'accord est le geste de l'utilisateur (`scheduled_accept` de la fenêtre, ou « Accepter » d'« À toi »).

## Limites

- Pas de déclenchement sur un événement : un horaire seulement.
- L'heure de la machine : deux machines dans deux fuseaux lancent à deux heures différentes ; les tâches n'étant pas partagées, ce n'est jamais la même tâche.
- Le plancher de 15 minutes se mesure sur les 500 prochaines échéances : un rythme cron qui ne rapprocherait deux échéances qu'au-delà serait accepté ; il attend de toute façon l'accord de l'utilisateur, qui voit son expression.
