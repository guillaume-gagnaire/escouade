# L'Overseer d'équipe (TS-22) : étude

Étude du 2026-11-21 (ESC-580), pour la 2.3 ou la version d'après. Elle part de deux conceptions qui font foi : `docs/OVERSEER.md` (l'Overseer d'une personne, construit par ESC-462 et ESC-468, page de spec « L'Overseer ») et `docs/TEAM-SERVER.md` (le serveur d'équipe : le §4.2 et TS-6 pour la prise atomique, C11 pour le pilote auto, le §9 pour le coupe-circuit, le §10 et TS-16 pour les coûts, le §11 et TS-17 pour le passage à une machine, le §13 pour la machine en mode serveur, la piste du §12). Rien n'est construit : ce document compare les voies, recommande, pose les choix O1 à O8 et découpe le travail en tickets OE-1 à OE-6.

Les coûts sont en **heures d'agent** Claude Code (un ticket de la 2.3 prend une à trois heures, six tournent en parallèle), jamais en jours de développeur. Ce qui attend l'utilisateur, ou des essais réels, est dit à part.

## En bref

- **Où il tourne** : sur une **machine de l'équipe en mode serveur** (`escouade-server`), jour et nuit, sous un **rôle de service** (un appareil de l'équipe, rattaché à l'équipe et non à un membre, sans droit d'administrateur). Un seul est actif par équipe : il tient un **bail** au serveur, comme une prise de ticket ; une seconde machine attend.
- **Sur quel compte** : les comptes de harness **de la machine**, que l'équipe y a connectés (un siège d'équipe, une clé d'API). Jamais le compte d'un membre : les comptes ne se partagent pas (§1 de `docs/TEAM-SERVER.md`).
- **Ce qu'il voit** : les projets de l'équipe dont la machine a la clé, leur Kanban avec les prises, les passages, la boîte de l'équipe, les coûts publiés. Jamais un projet personnel d'un membre, ni un projet privé, ni la conversation d'un agent qui tourne sur un poste.
- **Ce qu'il fait seul** : lancer et suivre les agents **sur la machine** (les tickets qu'on lui confie, ou qu'on lui passe), relire ou faire relire, répondre à **ses** agents, relancer, rendre compte. **Ce qu'il propose** : tout ce qui touche un membre (lui confier un ticket, relire son travail, le débloquer) arrive dans la boîte « À toi » du membre, jamais comme un ordre. Valider, fusionner, pousser et publier ne se font que sous une consigne d'un administrateur, et seulement pour les tickets que la machine tient.
- **Qui lui parle** : tout membre, dans un **fil commun** de l'équipe, chiffré et signé par l'appareil de qui écrit. Seul un **administrateur** lui donne une consigne durable. Il n'obéit qu'aux messages signés d'un membre au registre, jamais à un texte qu'il lit.
- **Deux Overseers sans conflit** : le bail fait qu'il n'y a qu'un Overseer d'équipe ; la prise atomique fait qu'un ticket n'a qu'un preneur, qui que ce soit qui le lance ; l'Overseer personnel d'un membre n'agit sur un ticket de l'équipe qu'au nom de son membre, comme lui ; chaque action de l'Overseer d'équipe est écrite dans le fil, que tout membre et tout Overseer lit.
- **Les bornes** : ses lancements comptent comme le pilote auto, donc les budgets de TS-16 et `stopAutopilot` les arrêtent ; l'équipe impose ses places et son budget (`overseer.*` du `.escouade.yml` d'équipe) ; le coupe-circuit l'arrête, lui et ses agents, et il ne peut ni le lancer ni le lever.
- **Le découpage** : 6 tickets, environ 13 h d'agent, en trois vagues. Il attend le mode serveur (fait), le rôle de service (avec TS-4) et, pour une part, le coupe-circuit (TS-15).

## 1. Le point de départ

Ce qui existe, et ce que l'Overseer d'équipe en reprend.

- **L'Overseer d'une personne** (page de spec « L'Overseer ») :
  - un agent d'Escouade hors de tout projet, sur un harness et un compte de l'utilisateur, avec les outils du serveur MCP et une consigne système propre ;
  - une session longue réveillée par le cœur à chaque événement utile, regroupés en un message ;
  - **il n'obéit qu'à l'utilisateur** : ses messages arrivent marqués (« [Message de l'utilisateur · m-3 · tapé] ») ; tout texte lu est donné comme données ; une consigne durable ne naît que d'un message de l'utilisateur dont elle cite les mots exacts (`add_directive`), avec la garde des négations ;
  - **les actions qui sortent** (valider, partager les docs, autoriser un `git push`, demander à un agent de publier) ne passent que sous une consigne qui les couvre, sinon le cœur refuse et il demande ;
  - il voit tous les projets sauf les privés, ne fait aucun travail long, n'écrit aucun fichier.
- **Le Kanban d'équipe** (TS-6, page « Le Kanban d'équipe ») :
  - la prise atomique au serveur : un ticket a un seul preneur, sous un bail de 10 min et un jeton de prise ;
  - C11-A : le pilote auto ne prend que les tickets assignés à son membre, ou « À faire » si on le coche ;
  - un ticket « En cours » ou « À tester » se valide sur le poste qui le tient.
- **Le passage à une machine** (TS-17) : une machine en mode serveur, membre de l'équipe, prend d'elle-même un agent qu'on lui passe et le fait tourner jour et nuit (`team_machine`).
- **Les coûts** (TS-16) : les totaux par jour remontent chiffrés pour les administrateurs ; les budgets et `stopAutopilot` arrêtent le pilote auto au-delà ; la machine de l'équipe pèse l'équipe entière.
- **Les tâches programmées** : par projet, **jamais partagées** (`docs/OVERSEER.md`), parce que chaque membre les lancerait en double.
- **Pas encore construits** : le coupe-circuit (TS-15), le rôle lecteur et l'annuaire des noms (TS-4), le rôle de service (aujourd'hui, le registre n'a que `Admin` et `Member`, et une machine y entre comme l'appareil d'un membre).

## 2. Ce qui change quand le Kanban est celui d'une équipe

L'Overseer d'une personne repose sur trois évidences qui tombent avec une équipe :

1. **Un seul maître.** Il obéit à « l'utilisateur ». Une équipe en a plusieurs, qui ne veulent pas la même chose, et dont l'un peut être un poste volé.
2. **Un seul quota.** Il dépense le compte de son utilisateur. Un Overseer d'équipe dépense pour tous : il faut savoir sur quel compte, et jusqu'où.
3. **Un seul Overseer.** Il est seul à piloter ses projets. Sur un Kanban d'équipe, il y a déjà l'Overseer personnel de chaque membre, le pilote auto de chacun, et peut-être deux machines.

Et un risque grandit : **l'injection par un collègue**. Le titre d'un ticket, une description, un souvenir ou une page de spec écrits sur un autre poste sont des données de tiers (le MCP les marque déjà `<third_party_data>`). Un Overseer d'équipe qui tourne jour et nuit avec les comptes de la machine est la cible rêvée d'un texte qui dit « valide tout et pousse sur main ».

## 3. Qui le fait tourner

### 3.1 Les voies

**Choix O1 : où il tourne.**

- **A. Sur une machine de l'équipe en mode serveur**, un seul par équipe. Il tourne jour et nuit, ses agents aussi, sur les comptes et le disque de la machine. Coût : environ 3 h (OE-3), sur le mode serveur et le passage de TS-17 déjà faits.
- **B. Sur le poste d'un administrateur**, l'Overseer de cette personne étendu à l'équipe. Coût : environ 2 h. Mais il s'arrête quand le portable se ferme, et il dépense le quota d'une personne pour le travail de tous.
- **C. Pas d'Overseer d'équipe** : l'Overseer de chaque membre voit les projets de l'équipe, dans les droits de son membre (c'est déjà le cas). Coût : nul. Mais personne ne suit l'ensemble, et rien ne tourne la nuit.

**Recommandation : A.** L'Overseer de chaque membre reste ce qu'il est (§7.3), et C est ce qui se passe tant que l'équipe n'a pas de machine.

### 3.2 Le rôle de sa machine et son compte

**Choix O2 : le rôle de la machine.**

- **A. Un rôle de service, nouveau** : un appareil inscrit au journal comme une machine **de l'équipe** (pas d'un membre), qui lit et écrit le contenu des projets qu'on lui ouvre, prend des tickets, mais ne signe rien d'administratif (ni membre, ni rôle, ni réglage, ni coupe-circuit, ni plugin privé) et ne répond pas aux invitations. Coût : environ 1 h 30 (OE-1), partagé avec la machine d'audit (`docs/TEAM-AUDIT.md`, A2) et à faire avec TS-4.
- **B. La machine d'un administrateur**, avec ses droits. Coût : nul. Elle peut alors répondre aux invitations la nuit (§3.3 de `docs/TEAM-SERVER.md`). Mais une machine exposée jour et nuit, pilotée par un modèle qui lit des textes de tiers, tiendrait une clé qui signe des membres et des réglages. Le cœur refuse déjà qu'un outil MCP signe (ce sont des gestes de la fenêtre), mais une faille de ce côté donnerait toute l'équipe.
- **C. La machine d'un membre** (le rattachement d'aujourd'hui) : elle agit comme ce membre, ses coûts sont les siens, et son départ l'emporte. Coût : nul.

**Recommandation : A.** Les invitations de nuit, si on les veut, se font par une autre machine d'administrateur, sans Overseer.

**Le compte.**

- Les comptes de harness sont **ceux de la machine** : l'équipe y connecte un compte (un siège d'un forfait d'équipe, une clé d'API d'organisation), par la fenêtre d'un client connecté au serveur, comme pour tout espace serveur (`docs/REMOTE.md`).
- **Jamais un compte de membre copié** : ni le serveur d'équipe, ni la machine ne reçoivent le compte d'un autre.
- **Son modèle** se règle sur la machine, comme Réglages › Overseer (Haiku sur Claude Code par défaut) ; un administrateur peut l'imposer (`overseer.model`, une borne du fichier d'équipe).
- **Les agents qu'il lance** prennent les comptes de la machine qui servent le projet, avec les règles des comptes d'aujourd'hui (le premier utilisable, la bascule à la limite d'usage).

### 3.3 Avec quels droits

Il a les droits d'un **membre de service** sur les projets qu'on lui ouvre, sous les réglages imposés de l'équipe (§6 de `docs/TEAM-SERVER.md`), et sous les stratégies du poste de la machine. Ce que l'équipe interdit (un harness, un mode, un plugin), il ne l'a pas. Il n'a jamais `bypassPermissions`.

## 4. Ce qu'il voit

| Il voit | Il ne voit pas |
| --- | --- |
| les projets de l'équipe dont la machine a la clé (un projet gardé à quelques membres seulement si on l'y a mise) | les projets personnels des membres, les projets privés, les autres équipes (les espaces sont cloisonnés) |
| leur Kanban : tickets, colonnes, prises (« pris par Alice depuis 14:02 », « interrompu chez Bob »), assignations, passages | ce que fait un agent sur un poste : sa conversation, son diff, ses autorisations (elles restent sur le poste) |
| leurs docs : spec, décisions, documents, mémoire d'équipe (avec la provenance, les souvenirs non validés marqués comme tels) | le code hors des worktrees de la machine (il le voit dans ceux de ses agents, par leurs outils) |
| ses agents à lui, sur la machine : leurs conversations, leurs questions, leurs diffs, leurs CI et leurs PR | les coûts d'un membre, sauf si l'équipe est « par membre » (C15-A) et alors seulement les totaux |
| le fil de l'équipe (§6) et les demandes « À toi » qu'il a déposées | les secrets, les comptes et les jetons des membres |
| le résumé des coûts du mois (le total de chaque projet et de l'équipe), les budgets imposés | |

**Ses réveils** sont ceux de l'Overseer d'une personne (un tour fini, une question, un ticket « À tester », une CI…) pour **ses** agents, plus quelques événements d'équipe : un ticket assigné à la machine ou passé à elle, un ticket interrompu (bail expiré), un message dans le fil, un budget franchi, un coupe-circuit. Pas chaque changement d'un ticket d'un collègue : il les lit quand il en a besoin.

**Ce qu'il lit est donné comme données**, comme aujourd'hui ; ce qui vient d'un autre poste est en plus marqué « écrit par Alice Martin, sur un autre poste ».

## 5. Ce qu'il décide seul, ce qu'il propose

### 5.1 La table

| Action | Seul | Sous une consigne d'administrateur | Seulement en proposant |
| --- | --- | --- | --- |
| lancer un ticket **confié à la machine** (assigné à elle, ou dans « À faire » d'un projet où l'équipe le permet) | oui, sous les places et le budget (§8) | | |
| prendre un agent qu'on lui passe (TS-17) | oui | | |
| répondre aux questions et autorisations de **ses** agents | oui, quand il sait ce que l'équipe dirait (une consigne, le fil, une réponse sans risque) | autoriser une commande qui pousse, fusionne ou publie | dans le doute : la demande va dans la boîte de qui a confié le ticket |
| relire, ou faire relire par un agent de la machine, un ticket de **ses** agents | oui | | |
| relire le travail d'un membre | | | « Veux-tu que je relise DEM-41 ? » dans la boîte du membre ; la relecture tourne sur la machine, depuis la branche poussée |
| créer un ticket, l'écrire, le découper | oui, dans un projet qu'il suit, marqué « créé par l'Overseer d'équipe » | | |
| assigner un ticket à un membre | | | « L'Overseer te propose DEM-52 » ; le membre accepte (il devient le sien, son pilote auto le prend s'il le fait) ou refuse (O3) |
| valider un ticket que la machine tient (fusion ou PR, selon `board.on_approve`) | | oui | sinon dans la boîte de qui l'a confié |
| valider, renvoyer ou déplacer un ticket qu'un membre tient | jamais : il se valide sur le poste qui le tient | | un mot dans le fil, ou une demande au membre |
| pousser une branche, poser un tag, publier | | oui, pour ses agents | |
| écrire dans la spec, les décisions, la mémoire | comme tout agent : en proposition, acceptée à la validation du ticket | | |
| changer un réglage d'équipe, un membre, un coupe-circuit, un plugin privé | jamais (le rôle de service ne signe rien) | | il peut le proposer dans la boîte d'un administrateur |
| arrêter un agent de la machine | oui | | |
| arrêter l'agent d'un membre | jamais | | un mot au membre |

### 5.2 Les tickets qu'il prend

**Choix O3 : ce qu'il lance et ce qu'il demande aux membres.**

- **A. Il lance sur la machine, il propose aux membres.** Il ne prend que ce qu'on confie à la machine (C11-A, appliqué à elle) ; toucher le travail d'un membre passe par sa boîte « À toi ». Coût : compris dans OE-3 et OE-4.
- **B. Il assigne directement** : un ticket assigné à un membre est pris par le pilote auto de ce membre (C11-A), donc l'Overseer dépense le quota de ce membre sans qu'il l'ait voulu. Coût : nul de plus.
- **C. A, et B pour les membres qui le cochent** (« Accepter les tickets que m'assigne l'Overseer d'équipe », un réglage de ce poste et de son membre, éteint d'office). Coût : environ 30 min de plus.

**Recommandation : A**, C plus tard si on le demande. C'est la raison de C11 : le quota de chacun ne sert que le travail qu'il a choisi.

**Choix O5 : ce qu'il valide seul.**

- **A. Rien sans consigne** : valider, fusionner, pousser ou publier un ticket que la machine tient demande une consigne d'un **administrateur**, avec sa portée (un projet, une release, une date) ; jamais un ticket qu'un membre tient. Coût : la garde des consignes existe, il faut la lier aux auteurs (OE-3).
- **B. Sous la consigne de tout membre.** Coût : le même. Mais un membre pourrait faire fusionner sur `main` d'un projet qu'il ne valide pas d'habitude, et un poste volé de simple membre le pourrait aussi.
- **C. Jamais, même sous consigne** : tout ce qui sort attend un humain. Coût : nul de plus. L'Overseer ne sert plus la nuit qu'à préparer.

**Recommandation : A.** Un réglage imposé `board.on_approve: pr` (déjà possible) en fait une PR, jamais une fusion directe : c'est la borne que la plupart des équipes voudront.

## 6. Qui lui parle, et l'autorité

### 6.1 Le fil de l'équipe

- **Un fil commun** par équipe (ou par projet, comme on l'ouvre), un flux du serveur chiffré par la clé d'équipe (ou du projet), où chaque message est **signé par l'appareil de qui l'écrit**. On lui écrit depuis un poste ou le téléphone, dans l'espace de l'équipe : « Overseer d'équipe ».
- **Ses réponses et ses actions** y sont écrites aussi, avec « Annuler » pour qui peut annuler (l'auteur de la demande, ou un administrateur). Tout le monde voit qui a demandé quoi et ce qu'il a fait : c'est ce qui évite que deux membres le tirent dans deux sens sans le savoir.
- **Un message privé** (un fil d'une personne avec lui) n'est pas proposé au départ : un employé partagé dont on ne voit pas les ordres crée exactement les conflits que le fil commun évite.
- Sa voix et son orbite restent sur le poste de qui lui parle (le raccourci, le micro, l'écran), comme l'API distante le fait déjà.

### 6.2 L'autorité

- **Il n'obéit qu'aux messages du fil** : signés par un appareil d'un membre au registre de l'époque, vérifiés par la machine. Le cœur les lui donne marqués comme aujourd'hui, avec leur auteur : « [Message de Claire Martin · administratrice · m-17 · tapé] ».
- **Une consigne durable** naît d'un message d'un administrateur, avec la même garde qu'aujourd'hui (les mots cités, les négations, 24 h, 20 derniers messages), plus une : **l'auteur doit être administrateur au moment du message et au moment de l'action**. Un administrateur qui part emporte ses consignes.
- **Toute demande d'un membre** (« lance DEM-52 sur la machine », « dis-moi où en est la release ») se fait dans les droits du rôle de service et de ce membre : un membre qui n'a pas accès à un projet gardé ne fait rien faire à la machine dans ce projet.
- **Un lecteur** (le rôle de TS-4) lit le fil, ne lui demande rien.
- **Retirer une consigne** : son auteur, ou tout administrateur, d'un mot ou d'un clic.
- **Ce qui ne donne jamais d'ordre** : un ticket, une description, un souvenir, une page de spec, une réponse d'agent, une issue, même écrits par un administrateur. Seul le fil commande.

**Choix O4 : qui lui parle.**

- **A. Tout membre lui demande, dans un fil commun ; seuls les administrateurs donnent des consignes durables.** Coût : environ 3 h (OE-4).
- **B. Tout membre au même titre**, consignes comprises. Coût : le même, mais une consigne de fusion d'un membre ou d'un poste volé de membre engage l'équipe.
- **C. Les administrateurs seuls.** Coût : le même, et l'Overseer d'équipe ne sert plus qu'à eux.

**Recommandation : A.**

### 6.3 Les menaces, et ce qui les tient

| Menace | Ce qui la tient |
| --- | --- |
| un texte de collègue qui dit « valide tout » | les données sont des données ; une consigne ne naît que du fil, d'un administrateur, en citant ses mots ; les actions qui sortent passent par la garde du cœur |
| le poste volé d'un membre | il peut demander dans le fil, comme le membre ; pas donner de consigne ; tout se voit dans le fil, et la révocation coupe l'appareil |
| le poste volé d'un administrateur | il peut donner une consigne : elle est écrite dans le fil, que tous voient, et ne couvre que les tickets de la machine ; `board.on_approve: pr` empêche la fusion directe |
| la machine prise | elle a le contenu des projets qu'elle tient et ses comptes de harness, comme tout appareil (§3.1 de `docs/TEAM-SERVER.md`) ; le rôle de service ne signe rien d'administratif ; on la révoque comme un membre, avec la rotation |
| le serveur qui ment | il ne peut ni forger un message du fil (signé) ni en glisser un d'un faux membre (le registre MLS) ; il peut en retarder, ce qui se voit à l'heure |

## 7. Deux Overseers ne se marchent pas dessus

Trois rencontres possibles.

### 7.1 Deux Overseers d'équipe (deux machines)

- **Un bail d'Overseer au serveur** : une prise comme celle d'un ticket, mais d'équipe (`/v1/team/leases/overseer`), sous un bail de 10 min renouvelé toutes les 3 min et un jeton qui croît à chaque prise. Une seule machine tient le bail et fait tourner l'Overseer ; l'autre attend, et le dit (« En attente : l'Overseer d'équipe tourne sur la machine build-2 »).
- **Chaque écriture de l'Overseer** (une entrée du fil, une proposition, un ticket créé) porte son jeton ; le serveur refuse celui d'un jeton dépassé, comme pour les tickets (`stale_token`). Une machine revenue d'une coupure ne parle plus au nom de l'Overseer.
- **La reprise** : la machine qui prend le bail lit le fil depuis le dernier message traité (écrit au fil par la précédente), et reprend les agents de la machine d'avant **seulement par le passage de TS-17** (l'agent reste là où il tourne : une machine qui reprend le bail ne relance rien qui tourne ailleurs).

**Choix O6 : un seul Overseer d'équipe.**

- **A. Un bail d'équipe au serveur, une machine active, les autres en attente.** Coût : environ 1 h 30 (OE-2).
- **B. Un Overseer par projet**, chaque projet désignant sa machine. Coût : environ 2 h 30. Utile à une grosse équipe, mais un fil commun ne sait plus à qui il parle.
- **C. Aucun garde** : la prise des tickets évite déjà les doublons de lancement. Coût : nul. Mais deux Overseers répondraient au même message, proposeraient deux fois, et se contrediraient dans le fil.

**Recommandation : A**, B plus tard si une équipe le demande, par le même bail nommé par projet.

### 7.2 L'Overseer d'équipe et le pilote auto des membres

- **La prise atomique** tranche : un ticket n'a qu'un preneur, qu'il soit lancé par un membre, son pilote auto, son Overseer ou l'Overseer d'équipe. Le second reçoit « pris par … » et passe.
- Le pilote auto d'un membre ne prend que ce qui est assigné à son membre (C11-A) ; l'Overseer d'équipe ne prend que ce qui est confié à la machine. Leurs tas ne se croisent pas, sauf « À faire » coché des deux côtés : la prise départage.
- Un ticket interrompu (bail expiré) n'est repris par personne seul : ni le pilote auto, ni l'Overseer d'équipe. Il propose « Reprendre sur la machine » à qui le tenait, par le passage de TS-17.

### 7.3 L'Overseer d'équipe et l'Overseer personnel d'un membre

- **L'Overseer personnel reste celui de son membre** : sur un projet de l'équipe, il agit au nom de son membre, avec ses droits, ses comptes et ses consignes, sur les tickets que son membre tient ou se fait assigner. Il ne parle pas au nom de l'équipe et n'écrit pas dans le fil (son membre le peut).
- **Il voit l'Overseer d'équipe** : ses actions sont dans le fil, et les tickets que la machine tient disent « Pris par l'Overseer d'équipe (machine build-1) ». Il ne lui donne pas d'ordre ; il peut dire à son membre « l'Overseer d'équipe relit déjà DEM-41 ».
- **L'Overseer d'équipe ne parle jamais à un Overseer personnel**, ni l'inverse : chacun ne reçoit d'ordres que de ses humains. Un message de l'un dans un ticket est une donnée pour l'autre.

### 7.4 Les tâches programmées d'un projet de l'équipe

Aujourd'hui, elles restent sur la machine de qui les crée, jamais partagées : partagées, chaque membre les lancerait. La machine de l'équipe est la place naturelle d'une **tâche programmée de l'équipe** : une seule, tenue par la machine qui tient le bail, visible de tous, lancée sous le budget de l'équipe. Ce n'est pas dans ce découpage ; c'est une suite naturelle (environ 2 h), à décider après.

## 8. Les bornes : coûts et quotas

- **Ses lancements comptent comme le pilote auto.** Un budget de TS-16 franchi avec `stopAutopilot` les arrête : celui d'un projet arrête ses lancements dans ce projet, celui de l'équipe tous. Il le dit dans le fil (« Je ne lance plus rien dans api : 104 % de son budget du mois, imposé par l'équipe ») et laisse une demande à l'administrateur qui tient le budget. Ce qu'il a déjà lancé finit son tour, puis s'arrête entre deux tours, comme le pilote auto.
- **Ses coûts sont ceux de la machine**, comptés par TS-16 sous un membre « machine de l'équipe » (jamais sous un membre réel), par projet et par ticket ; sa propre session (ses réveils) compte à part, sous « Overseer d'équipe ».
- **L'équipe lui impose des bornes** (`overseer.*` du `.escouade.yml` d'équipe, des clés de la sorte « restreindre ») :
  - `overseer.enabled` : permis ou non ;
  - `overseer.projects` : les projets où il agit (par défaut, ceux de la machine) ;
  - `overseer.maxAgents` : ses agents en même temps (3 par défaut) ;
  - `overseer.budget` : en dollars par mois, pour ses agents et lui (rien par défaut : les budgets de TS-16 suffisent) ;
  - `overseer.model`, `overseer.engines` : ses modèles et harness permis, croisés avec ceux de l'équipe ;
  - `overseer.mayApprove` : s'il peut valider sous consigne (O5-A ; `false` en fait C).
- **Un compte à forfait** (un siège Claude, ChatGPT Pro) n'a pas de coût par tour : le budget s'y compte en équivalent API, comme dans le tableau de TS-16 ; la limite d'usage du compte arrête ses agents comme les autres, et il le dit.
- **Les nuits** : il tourne la nuit sous les mêmes bornes. Pas de « mode nuit » plus large.

**Choix O7 : les bornes des coûts.**

- **A. Ses lancements comptent comme le pilote auto, plus les clés `overseer.*` imposables.** Coût : environ 2 h (OE-5), sur les budgets de TS-16.
- **B. Les budgets de TS-16 seuls.** Coût : environ 30 min. Mais rien ne borne ses agents en même temps, ni lui à part.
- **C. Un budget à lui seul, hors de ceux de l'équipe.** Coût : environ 1 h, et une seconde comptabilité à tenir à côté de TS-16.

**Recommandation : A.**

## 9. Le coupe-circuit

Le coupe-circuit (TS-15, pas encore construit) est une entrée signée par un administrateur au journal, que chaque appareil en ligne applique dans la seconde (§9 de `docs/TEAM-SERVER.md`).

- **Il arrête l'Overseer d'équipe comme un agent** : « arrêter tous les agents » de l'équipe arrête ses agents **et lui**. Retirer un harness ou un modèle arrête ceux qui en dépendent, lui compris s'il tourne dessus.
- **Une portée de plus, « Overseer d'équipe »** : l'arrêter lui seul, ses agents finissant leur tour puis s'arrêtant, sans toucher aux agents des membres. C'est le geste d'après une consigne mal donnée, ou un comportement inattendu.
- **Il ne peut ni le lancer, ni le lever** : le rôle de service ne signe rien d'administratif. Il ne le contourne pas : ce n'est pas sa consigne qui l'arrête, c'est le cœur de la machine qui n'ouvre plus de tour.
- **Hors ligne, la machine ne lance rien** : la stratégie « sans contact avec le serveur depuis N heures, pas de nouvel agent » vaut d'office pour l'Overseer d'équipe, avec N = 1 h. Une machine coupée du serveur ne pilote plus l'équipe à l'aveugle.
- **Sur la machine elle-même** : `escouade-server team overseer stop` l'arrête en local, sans le serveur, pour qui administre la machine.
- **Le retour** : levé, il repart à son réveil suivant, relit le fil depuis l'arrêt, et ne relance rien seul de ce que l'arrêt a interrompu : il dit ce qui était en cours et propose de le reprendre.

**Choix O8 : le coupe-circuit.**

- **A. Il l'arrête avec ses agents, une portée « Overseer d'équipe » en plus, la machine muette hors ligne.** Coût : environ 1 h dans OE-5, et environ 30 min dans TS-15 pour la portée.
- **B. Le coupe-circuit d'aujourd'hui seulement** (tous les agents, un harness, un modèle, un plugin). Coût : nul de plus, mais arrêter l'Overseer seul demande d'arrêter tous les agents de l'équipe.

**Recommandation : A.**

## 10. Les choix à trancher

**Tranchés par l'utilisateur le 2026-11-21** (demande a-23) : O1 à O8 en A, toutes les recommandations. Décisions proposées par ESC-588 ; `docs/TEAM-SERVER.md` §15.3.

| N° | Question | Options | Recommandation |
| --- | --- | --- | --- |
| O1 | Où il tourne | A une machine de l'équipe en mode serveur · B le poste d'un administrateur · C pas d'Overseer d'équipe | A |
| O2 | Le rôle de sa machine | A un rôle de service, sans droit d'administrateur · B une machine d'administrateur · C la machine d'un membre | A |
| O3 | Ce qu'il lance, ce qu'il demande aux membres | A il lance sur la machine, il propose aux membres · B il assigne directement · C A, et B pour qui le coche | A |
| O4 | Qui lui parle | A tout membre dans un fil commun, les consignes aux administrateurs · B tout membre, consignes comprises · C les administrateurs seuls | A |
| O5 | Ce qu'il valide seul | A rien sans consigne d'un administrateur, et seulement ce que tient la machine · B sous la consigne de tout membre · C jamais | A |
| O6 | Deux Overseers d'équipe | A un bail d'équipe, une machine active · B un par projet · C aucun garde | A |
| O7 | Les bornes des coûts | A comme le pilote auto, plus les clés `overseer.*` · B les budgets de TS-16 seuls · C un budget à lui seul | A |
| O8 | Le coupe-circuit | A il l'arrête avec ses agents, une portée « Overseer d'équipe », muet hors ligne · B celui d'aujourd'hui seulement | A |

## 11. Le découpage en tickets

**Écrits par ESC-588** : OE-1 = ESC-606, OE-2 = ESC-611, OE-3 = ESC-613, OE-4 = ESC-615, OE-5 = ESC-616 (après ESC-546, le coupe-circuit, écrit sans la portée « Overseer d'équipe » : OE-5 l'ajoute), OE-6 = ESC-618.

L'ordre : le rôle et le bail (sans eux, deux machines ou un administrateur exposé), puis l'Overseer sur la machine, puis le fil depuis les postes, puis les bornes, puis la cohabitation avec les Overseers personnels.

| Ticket | Contenu | Après | Coût |
| --- | --- | --- | --- |
| OE-1 | **Fait (ESC-606)**. Le rôle de service au registre : une machine de l'équipe inscrite comme telle (ni membre, ni administrateur), ce qu'elle signe (ses appareils, le contenu des projets ouverts, ses prises) et ce qu'elle ne signe jamais (les règles de `policy.rs`), le serveur qui la refuse sur les routes d'administrateur ; `escouade-server team pair` en service ; son rang dans Réglages › Équipe. Partagé avec la machine d'audit (`docs/TEAM-AUDIT.md`, A2) | TS-4 (ou avec lui) | 1 h 30 |
| OE-2 | Le bail d'équipe : `/v1/team/leases/{nom}` (prendre, renouveler, rendre, sous jeton), le jeton vérifié sur les écritures du fil, la machine en attente et ce qu'elle dit ; tests à deux machines sur SQLite puis PostgreSQL | OE-1 | 1 h 30 |
| OE-3 | L'Overseer d'équipe sur la machine : `escouade-server team overseer`, un Overseer par équipe dans le cœur sans fenêtre, sa portée (les projets de l'équipe que la machine tient), ses réveils d'équipe (§4), ses outils MCP bornés à l'équipe, ce qu'il prend (O3-A : ce qui est confié à la machine), la garde des actions qui sortent liée à l'auteur administrateur d'une consigne (O5-A), ses propositions déposées dans la boîte « À toi » des membres et des administrateurs | OE-2, mode serveur | 3 h |
| OE-4 | Le fil de l'équipe : un flux chiffré par la clé de l'équipe ou du projet, chaque message signé par son appareil et vérifié au registre ; l'écran « Overseer d'équipe » dans l'espace de l'équipe (la discussion, ses actions et « Annuler », les consignes avec leur auteur) sur le poste et par l'API distante pour le téléphone ; les messages marqués de leur auteur et de son rôle ; les consignes seulement d'un administrateur (O4-A) | OE-3 | 3 h |
| OE-5 | Les bornes : ses lancements comptés comme le pilote auto (`stopAutopilot`), ses coûts sous « machine de l'équipe » et « Overseer d'équipe » (TS-16), les clés `overseer.*` du fichier d'équipe (sortes, croisement), la machine muette sans contact depuis 1 h, `escouade-server team overseer stop` ; avec TS-15 : le coupe-circuit qui l'arrête et sa portée « Overseer d'équipe » | OE-3, TS-16 (TS-15 pour la part du coupe-circuit) | 2 h |
| OE-6 | La cohabitation : l'Overseer personnel sur les projets de l'équipe (au nom de son membre, la mention « Pris par l'Overseer d'équipe »), aucun message entre Overseers, la reprise d'un bail par une autre machine (le fil relu, rien relancé) ; un e2e avec deux clients, une machine et deux machines en attente | OE-4, OE-5 | 2 h |

**En tout** : 6 tickets, environ 13 h d'agent. Avec le parallélisme (OE-1, puis OE-2, puis OE-3, puis OE-4 et OE-5 ensemble, puis OE-6), cinq vagues courtes, soit une grosse journée avec la relecture et les e2e relancés.

**Ce qui ne va pas à ce rythme** :

- **l'essai réel** sur une vraie machine de l'équipe (le VPS d'essai), avec un vrai compte de harness connecté à la machine, et deux membres qui lui parlent : à l'utilisateur, qui seul connecte un compte ;
- **le coupe-circuit** (TS-15) n'est pas construit : OE-5 fait sa part sans lui, et TS-15 ajoute la portée « Overseer d'équipe » (environ 30 min de plus dans TS-15) ;
- **le rôle de service** demande un changement des règles du groupe MLS (`policy.rs`) : un point de plus pour l'audit externe de la cryptographie (TS-20).

**Ce que ça ne change pas** : l'Overseer de chacun reste celui de son utilisateur, la prise atomique reste la seule garantie contre les doublons, et le serveur d'équipe reste aveugle : il ne voit du fil que des entrées chiffrées et du bail qu'un id, une machine et une heure.
