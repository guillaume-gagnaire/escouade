---
title: "Les coûts et les quotas de l'équipe"
subject: "serveur-equipe"
sources: ["docs/TEAM-SERVER.md §2.2, §3, §6.4, §10, §15.2 (C15), §16 (TS-16)","crates/escouade-team/src/costs.rs","crates/escouade-team-crypto/src/costkey.rs","src-tauri/src/teamcosts.rs, src-tauri/src/teams_costs.rs","Revue de sécurité de la 2.3 (ESC-626), constats SR3-03 et SR3-08 : corrigés par ESC-649"]
tickets: ["ESC-547","ESC-649"]
decisions: [225]
compared: null
---

# Les coûts et les quotas de l'équipe

Ce que l'équipe sait de ce que dépensent ses agents : des **totaux par jour**, calculés sur chaque poste, **chiffrés pour les seuls administrateurs**. Le serveur d'équipe ne voit pas les coûts. Jamais un prompt, une réponse, un nom de fichier ni un tour ne part.

## Ce qui remonte

- **Calculé sur le poste**, depuis ses Stats (les tours de ses agents) : seuls les projets de l'équipe qui ont un Kanban d'équipe comptent ; un projet perso ne part jamais.
- **Une fois par heure**, le poste envoie ses totaux du jour et de la veille (et, après un changement de la clé, tout le mois en cours). Par jour, par projet (l'id de son journal, le même sur chaque poste), par harness, par modèle et par sorte de compte :
  - les tokens d'entrée, de sortie et de cache ;
  - le coût estimé ;
  - le plus haut pourcentage de quota que le harness a atteint ce jour-là ;
  - le total de chaque ticket, par sa clé (« API-12 »).
- **Rien d'autre** : ni le titre d'un ticket, ni le nom d'un agent, ni un tour, ni un fichier. Les types du format refusent un champ de plus dans les deux sens, et un test relit les octets exacts de ce qui part.
- **Hors ligne**, les totaux attendent sur le poste (dans sa base chiffrée de l'équipe), un jour à la fois : le plus récent d'un jour remplace celui qui attendait. Ils partent au retour. Un envoi refait (une réponse perdue) ne compte pas deux fois : un administrateur garde, pour chaque poste et chaque jour, le dernier total reçu.
- « Envoyer maintenant », dans l'écran des coûts, n'attend pas l'heure.

## Les bornes d'un envoi (SR3-03, SR3-08)

Un membre ne peut ni remplir les budgets de toute l'équipe (et arrêter le pilote auto de tous les postes), ni cacher sa dépense, ni la mettre au compte d'un autre. L'appareil de l'administrateur **refuse en entier, à la lecture**, des totaux :

- dont un coût n'est pas un nombre fini (`NaN`, infini), est négatif, ou dépasse **10 000 $ par jour et par appareil** (toutes ses lignes du jour ensemble, et chaque ticket) ;
- dont les tokens d'un jour dépassent **10 milliards** ;
- dont un quota n'est pas un nombre fini entre 0 et 1 000 % ;
- trop grands : plus de **62 jours** dans un envoi, plus de **256 lignes** ou de **1 024 tickets** par jour, un jour qui n'est pas une date (`2026-11-20`), un nom (projet, harness, modèle, ticket) de plus de 256 caractères ou avec un caractère de contrôle ;
- **qui nomment un autre membre que celui qui les a signés** : en « par membre », la dépense est toujours celle de l'appareil qui signe l'entrée, jamais d'un nom qu'il donne.

Un envoi refusé ne compte pas : le résumé du mois ne bouge pas, et l'appareil de l'administrateur en garde le nombre. Les sommes du résumé sont saturées (les tokens ne débordent jamais).

- **Sur le poste qui envoie**, la file refuse de même ce que les administrateurs refuseraient : un `NaN` mis en file ne casse plus l'état des coûts du poste. Un coût que le harness donne mal (`NaN`, infini, négatif) compte pour 0 dans les totaux du poste.
- **Le résumé publié** par un administrateur (« Le mois de l'équipe ») est refusé par chaque poste quand un de ses coûts n'est pas fini ou est négatif : l'état de chacun se relit toujours.

## Le chiffrement : la clé des administrateurs

- Une **paire de clés à part** (HPKE, RFC 9180), hors des clés MLS de l'équipe. Chaque poste scelle ses totaux à sa moitié publique ; seul un appareil d'administrateur l'ouvre.
- Sa moitié secrète est **enveloppée pour chaque appareil d'administrateur**, à la clé des coûts propre de cet appareil (tirée sur lui, gardée dans son état chiffré).
- **La rotation** : dès qu'un appareil pour lequel la clé est enveloppée n'est plus un appareil d'administrateur (un administrateur retiré ou redevenu membre, un appareil révoqué), le premier appareil d'administrateur en ligne tire une nouvelle paire. Entre-temps, **aucun poste ne scelle à l'ancienne** : ses totaux attendent. Un nouvel administrateur reçoit l'enveloppe de la clé en cours.
- **Ce que voit le serveur** : qu'une entrée est écrite, sa taille, son heure et l'appareil qui l'écrit. Il ne laisse qu'un administrateur écrire la clé (sinon un membre ferait sceller les totaux à sa propre clé) ; chaque poste refuse aussi, à la lecture, une clé qui ne vient pas d'un administrateur.
- **Un membre ne lit que les siens** : il les a en clair sur son poste, il ne peut pas ouvrir ceux des autres.

## Le grain : par projet d'office, par membre au choix de l'équipe (C15)

- `costs.grain` du fichier de l'équipe : `project` d'office, ou `member`.
- **Par projet** : les totaux partent sans le nom du membre. Les administrateurs voient chaque projet, chaque ticket, chaque harness et modèle, jamais une personne.
- **Par membre** : les administrateurs voient en plus les totaux de chaque membre, par jour. **Chaque membre le voit dit** : à sa prochaine ouverture d'Escouade, l'écran « Ton équipe voit tes totaux par jour » (« J'ai compris », « Voir ce qui remonte »). **Tant qu'il ne l'a pas vu, ses totaux partent sans son nom.** L'équipe repassée par projet, le prochain passage par membre est dit de nouveau. Le membre nommé est toujours celui qui signe (voir « Les bornes d'un envoi »).
- Limite, dite telle quelle : par projet, le serveur sait toujours quel appareil a écrit chaque entrée, et l'appareil d'un administrateur la voit signée par l'appareil de son auteur. Le grain par projet est tenu par l'app, pas par la cryptographie.

## Le tableau (Équipe › Coûts)

- Dans la fenêtre des équipes, bouton « Coûts » de chaque équipe (Réglages › Équipe est TS-4).
- **Un administrateur** voit les totaux de tous les postes ; **un membre** voit les siens et « Le mois de l'équipe » : le total de chaque projet et de l'équipe, que l'appareil d'un administrateur publie (scellé sous la clé de l'équipe, jamais un membre dedans).
- **Regrouper** : par membre (seulement quand l'équipe l'a choisi, pour un administrateur), par projet, par ticket, par harness et modèle. Colonnes : tokens, coût, plus haut quota. Les lignes de ce poste sont marquées.
- **La période** : la semaine (depuis lundi), le mois (depuis le 1er), ou une période choisie (du, au).
- **Un compte à forfait** (Claude Max, ChatGPT Pro : un compte qui a un quota et pas de clé d'API) n'a pas de coût par tour : son coût est un **équivalent en API**, écrit « ≈ 3,20 $ » et marqué « équivalent API » (« en partie un équivalent API » quand la ligne mêle les deux).
- **Ce qui remonte de toi** : « Tes totaux par jour, jamais ton travail », le grain de l'équipe dit en une phrase (« L'équipe Acme voit les totaux par projet : les tiens partent sans ton nom »), les jours qui attendent, « Voir ce qui est parti » (le dernier envoi, tel quel), « Envoyer maintenant ». Sans clé des administrateurs encore, ou sous une clé à changer, l'écran le dit : les totaux attendent.

## Les budgets et les alertes

- Dans le fichier de l'équipe (signé, page « Le .escouade.yml d'équipe ») :

```yaml
enforced:
  costs:
    grain: member              # ou project (d'office)
    budgets:                   # en dollars, par mois
      team: 500
      projects:
        api: 200               # par le nom du projet, ou l'id de son journal
      members:
        "*": 50                # chaque membre ; ou l'id d'un membre
    stopAutopilot: true        # au-delà d'un budget, le pilote auto s'arrête
```

- **Avant un lancement**, le poste pèse les budgets du projet avec son propre total du mois et le dernier total publié de l'équipe. À 80 % ou plus, « Lancer » demande d'abord : « Le projet api a dépensé 92 % de son budget du mois. Le lancer quand même ? » ; au-delà, avec `stopAutopilot`, « … : le pilote auto de ce projet est arrêté par l'équipe Acme. Le lancer quand même ? ».
- **Au-delà d'un budget, avec `stopAutopilot`**, le pilote auto s'arrête (une restriction, §6.4) : le budget d'un projet arrête le pilote auto de ce projet, celui de l'équipe ou de ce membre celui de tous les projets de l'équipe. Le Kanban le dit (« Pilote auto arrêté : Le projet api a dépensé 104 % de son budget du mois (imposé par l'équipe Acme) »), ses tickets « À faire » attendent ; un ticket lancé à la main part après la question. Un budget de l'équipe ou du membre pose le verrou : l'interrupteur « Pilote auto » et la ligne « Pilote auto » des réglages du Kanban sont grisés, « Imposé par l'équipe Acme ». Revenu sous le budget (budget relevé, mois suivant), tout repart.
- **L'appareil d'un administrateur**, ou la machine de l'équipe en mode serveur, pèse l'équipe entière (et chaque membre quand l'équipe est par membre) et laisse une demande dans sa boîte « À toi » : à 80 %, puis à 100 %, une fois par budget et par mois.

## Limites

- Le total du membre est celui de ce poste : deux postes d'un même membre ont chacun le leur.
- Le budget d'un projet se nomme par le nom du projet sur le poste de l'administrateur, ou par l'id de son journal.
- Le plus haut quota du jour est relevé par le poste à chaque minute, pas à chaque tour.
- Un membre peut encore déclarer, sous les bornes, une dépense qu'il n'a pas faite (jusqu'à 10 000 $ par jour et par appareil) : elle reste à son appareil, que l'administrateur voit.
