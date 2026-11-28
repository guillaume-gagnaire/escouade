---
title: "L'audit vers un SIEM"
subject: "serveur-equipe"
sources: ["docs/TEAM-AUDIT.md (ESC-580, TS-21 : l'étude, les choix A1 à A8, le découpage AU-1 à AU-6 ; §3.1 précisé par AU-2 ; §3.3 construit par AU-3)","Réponse de l'utilisateur du 2026-11-21 (demande a-23) : toutes les recommandations, A1-C, A2-A, A3-B, A4-A, A5-B, A6-A, A7-A, A8-A","docs/TEAM-SERVER.md §3.7, §3.8, §6, §9, §12, §13, §15","crates/escouade-team-server/README.md (Ce que le serveur voit et garde ; L'audit vers un SIEM, AU-2 ; Les défenses, l'audit des postes, AU-3)","crates/escouade-team-crypto/src/beacon.rs (les balises de tête)","crates/escouade-team/README.md (Les coûts de l'équipe : la clé des administrateurs ; L'audit des postes, AU-3)","crates/escouade-team-crypto/README.md (Les clés par époque d'un usage, AU-3)","docs/registre-rgpd.md","crates/escouade-audit/README.md (AU-1, ESC-608 : ce qui est construit)","RFC 8785 (JCS), RFC 5424 et RFC 5425 (syslog sur TLS), RFC 5612 (le numéro 32473), RFC 9180 (HPKE)"]
tickets: ["ESC-580","ESC-588","ESC-608","ESC-609","ESC-610","ESC-612","ESC-614","ESC-617","ESC-606"]
decisions: [264,275,265,266,267,268,346,342,360,375]
compared: null
---

# L'audit vers un SIEM

> **Décidé ; le modèle est construit (AU-1), le serveur émet seul (AU-2), la clé d'audit et le flux des postes sont là (AU-3).** L'utilisateur a pris toutes les recommandations de l'étude d'ESC-580 (`docs/TEAM-AUDIT.md`) le 2026-11-21 (A1-C, A2-A, A3-B, A4-A, A5-B, A6-A, A7-A, A8-A). Cette page dit ce qui est décidé ; chaque partie se construit par un ticket, de AU-1 (ESC-608) à AU-6 (ESC-617). AU-1 est fait : la crate `escouade-audit` (plus bas). AU-2 est fait : le serveur d'équipe émet son propre audit (plus bas, *Ce qu'émet le serveur seul*). AU-3 est fait : la clé d'audit des postes et le flux où ils déposent leurs faits scellés (plus bas, *La clé d'audit et le flux des postes*). Le rôle de service dont dépend la machine d'audit est construit (OE-1, ESC-606). Les postes ne déposent encore rien (AU-4 branche leurs faits) et la machine d'audit n'émet rien (AU-5).

## Ce que c'est

Un journal d'audit de l'équipe, envoyé à son outil de sécurité (Splunk, Elastic, Microsoft Sentinel, Datadog…). Il est éteint d'office ; l'équipe l'allume, sur son serveur et sur une machine de l'équipe.

## Ce qui sort, ce qui ne sort jamais (A3-B, A8-A)

- **Sort** (une liste fermée d'événements, chacun avec un nom stable et une sévérité de 1 à 5) : les membres, les appareils, les rôles, les époques et les invitations ; les accès au serveur ; les réglages imposés, les signataires et les accords des postes ; le coupe-circuit ; le store privé et les plugins installés ; les prises et les passages de tickets ; les validations ; les autorisations données aux agents (par qui) ; les consignes de l'Overseer ; la mémoire validée ou mise en quarantaine ; les budgets franchis ; les refus du cœur.
- **Les noms** : les événements nomment les membres, les appareils et les projets, et donnent les clés des tickets, **jamais leurs titres**, descriptions ou critères.
- **Ne sort jamais** : le contenu chiffré (titres, descriptions, spec, décisions, documents, souvenirs), les secrets (jetons, clés, mots de secours, secret d'une invitation, code d'appairage), le texte des conversations, des prompts, des sorties, des diffs et des messages de commit, les noms de fichiers, le code, les comptes des harness. Une commande autorisée sort par **son programme et son empreinte**, jamais son texte. Ni chaque tour d'agent, ni les renouvellements de sessions et de baux.
- Chaque événement est une structure fermée, et un test cherche des données piégées dans tout ce qui sort (AU-1, ESC-608).

## Les sources (A1-C, A2-A)

- **Le serveur seul** (AU-2, ESC-609, **fait**) : ce que voient ses routes, en ids opaques, signé par une clé d'audit du serveur tirée à `init`.
- **Une machine de l'équipe qui déchiffre** (AU-5, ESC-614), sous le **rôle de service** (OE-1, ESC-606, **fait**) ; un poste d'administrateur en recours, tant qu'il est ouvert. Elle nomme ce qui se passe dans le journal et les projets qu'elle tient, et dit ceux qu'elle ne suit pas.
  - Ce que donne OE-1 : un administrateur l'inscrit par un code court (« Inscrire une machine de l'équipe », puis `escouade-server team pair` sur la machine) ; elle a son propre id au registre, rattaché à l'équipe ; elle lit le flux de l'équipe et les projets qu'on lui ouvre, et ne signe jamais un membre, un rôle, un réglage, un coupe-circuit, un plugin privé, ni ne répond à une invitation (chaque poste le refuse, `policy.rs`) ; le serveur la refuse sur ses routes d'administrateur et sur les flux propres de l'équipe. La même machine peut porter l'Overseer d'équipe (voir « L'Overseer d'équipe »).
- **Les postes** (AU-3, ESC-610, **fait** pour la clé et le flux ; AU-4, ESC-612, pour les faits) : ce qui n'arrive que là (refus du cœur, autorisations, commandes approuvées, plugins, coupe-circuit appliqué), scellé pour une clé d'audit à part et déposé sur le serveur ; la machine l'ouvre. Un poste ne parle jamais au SIEM, et rien ne sort d'un poste en clair. Un poste modifié peut taire ses faits.

## Les formats (A4-A)

Un modèle JSON unique (`escouade.audit/v1`), écrit en JSON lines (fichier tourné ou sortie standard) et en syslog RFC 5424 sur TLS, CEF en option ; OpenTelemetry par un Collector qui lit le fichier ; pas de connecteur propre à un SIEM au départ. Options `--audit-file`, `--audit-stdout`, `--audit-syslog`, `--audit-cef` sur le serveur (construites, voir « Le serveur d'équipe ») et la machine ; un tampon sur disque si le SIEM ne répond pas (7 jours ou 1 Gio, puis `audit.dropped`).

## L'intégrité (A5-B)

Chaque événement est signé par son émetteur, chaîné au précédent, et ancré à l'entrée du journal du serveur d'où il vient (son numéro, son époque, l'empreinte de son chiffré). La tête de la chaîne est publiée chaque heure dans le flux de l'équipe, comme les balises de tête. `escouade-audit verify` vérifie un export et recoupe ce que dit le serveur avec ce que dit la machine.

## Ce qu'émet le serveur seul (AU-2, ESC-609)

Le serveur d'équipe (`escouade-team-server serve --audit-*`, voir « Le serveur d'équipe », *Son audit vers un SIEM*) émet par la crate `escouade-audit`, sous la source `server`, signé par sa clé d'audit.

- **En ids opaques**, en hexadécimal : les appareils, les équipes, les projets, et l'id de la prise d'un ticket ; jamais un nom ni une clé de ticket (le serveur ne les connaît pas). Un jeton servi deux fois sort sous un id opaque tiré de son hachage, jamais le jeton ni son hachage.
- **Une chaîne pour le serveur entier**, qui peut porter plusieurs équipes : chaque événement dit son équipe, et `server` quand le serveur n'en connaît aucune (une adresse au-delà de sa limite, un client trop vieux, un appareil inconnu qui se connecte).
- **Ce qu'il émet** :
  - l'inscription d'un appareil (`device.enrolled`, avec la sorte du jeton : `init`, invitation, récupération) ; le jeton de récupération quand il sert (`recovery.used`) ; le second usage d'un jeton (`invitation.reused`) ;
  - l'ouverture d'une session (`session.opened`, **une fois par appareil et par adresse, puis une fois par jour**) et son refus (`session.refused` : appareil inconnu, révoqué, mauvaise signature ; une fois par minute pour un même appareil, une même cause et une même adresse), avec l'adresse IP ;
  - la révocation d'un appareil (`device.revoked`) et le changement de son rôle sur le serveur (`role.changed`, l'appareil et ses deux rôles) ;
  - l'écriture d'une entrée (`stream.written`) : chaque entrée du flux de l'équipe et de ses flux propres (la liste des projets, les réglages, les propositions, la clé des coûts, le store privé, le coupe-circuit, la clé d'audit des postes et les dépôts des postes ; pas les totaux des coûts), les commits d'un projet ; **jamais les opérations d'un projet**, qui sont le travail de chacun ;
  - les droits d'un projet (`project.access.changed`) ;
  - les prises : `claim.taken`, `claim.released`, `claim.expired` (quand un autre appareil reprend un ticket dont le bail était fini), `handoff.offered`, `handoff.taken`, `handoff.withdrawn`, et `claim.fenced` (une écriture sous un jeton dépassé) ; **jamais les renouvellements** ;
  - les refus avant toute route : `rate.limited` (une fois par minute, avec le nombre de refus depuis), `client.too_old` (une fois par heure pour une adresse et une version), `request.too_large` (un corps refusé pour sa taille, et la taille la plus grande de la route).
- **L'ancre** de chaque entrée : le flux (`team`, le nom d'un flux propre de l'équipe comme `escouade.setting`, ou l'id du projet), le numéro de l'entrée, son époque (celle d'un commit, celle où en est son flux pour un message) et l'empreinte SHA-256 de ses octets chiffrés, tels que le serveur les sert. La machine d'audit (AU-5) répète la même ancre pour la même entrée : c'est le recoupement.
- **Les fichiers et les instantanés** n'émettent rien : leurs octets sont le contenu des projets.
- **L'adresse IP** est en entier par défaut, coupée à son /24 ou son /48, ou retirée (`--audit-ip`).
- Un test lit tout ce qui sort pendant un parcours complet (équipe, invitation, projet, tickets, passage, documents), sur SQLite puis PostgreSQL, et n'y trouve aucun contenu, jeton, clé ni session ; `escouade-audit verify` vérifie la sortie.

## La clé d'audit et le flux des postes (AU-3, ESC-610)

Le modèle est la clé des coûts (voir « Les coûts et les quotas de l'équipe »), dont le code est devenu commun sans que les coûts changent (mêmes étiquettes, mêmes octets, même état).

- **Les émetteurs** : les appareils des machines de l'équipe (rôle de service) quand l'équipe en a une ; sinon les appareils de ses administrateurs (le recours d'A2-A ; AU-5 dira lequel tient l'audit, tous peuvent ouvrir d'ici là). Personne d'autre.
- **La clé d'audit** : une paire HPKE (RFC 9180) à part des clés de l'équipe et de celle des coûts, une par époque. Chaque émetteur publie sa propre clé d'audit ; un émetteur tire la paire de l'époque et l'enveloppe pour les seuls émetteurs, puis pour ceux qui viennent après. Une clé enveloppée pour un appareil qui n'émet plus (une machine retirée, un administrateur parti, ou une machine qui arrive : les administrateurs passent la main) est périmée : aucun poste n'y scelle, et un émetteur en tire une nouvelle. Chaque poste refuse une entrée de la clé dont le signataire n'émettait pas à son époque (un administrateur ne glisse pas sa paire quand une machine émet). Un chiffré ou une enveloppe des coûts ne s'ouvre jamais comme ceux de l'audit, ni l'inverse.
- **Le flux des postes** (`escouade/audit` dans l'étude ; deux flux propres de l'équipe, `escouade/audit/k` pour la clé et `escouade/audit/f` pour les faits, chacun de 16 octets comme les autres) : chaque poste y dépose ses faits scellés. **Le serveur refuse qu'un membre le lise**, ses propres faits compris, par le flux, le direct ou un fichier ; seuls un administrateur ou une machine le lisent, et seuls les émetteurs l'ouvrent. Un membre n'écrit pas la clé (le serveur le refuse). Le serveur n'en voit qu'une taille, un appareil et une heure.
- **Ce qu'un poste dépose** : des faits de la liste fermée que le §1 donne aux postes (`team.fork`, `team.rollback`, `settings.sensitive.answered`, `command.approved`, `command.revoked`, `policy.applied`, `lock.refused`, `killswitch.applied`, `plugin.installed`, `plugin.removed`, `plugin.updated`, `plugin.refused`, `ticket.approved`, `ticket.rejected`, `permission.answered`, `agent.mode.changed`, `directive.added`, `directive.revoked`), chacun avec sa cible et son détail fermés (AU-1), son id et l'heure du poste. **Jamais d'auteur** : l'émetteur le prend de la signature de l'entrée (le membre et l'appareil), un poste ne parle pas pour un autre. Un autre événement, ou un champ en trop, est refusé au dépôt comme à l'ouverture.
- **Hors ligne** : les faits attendent dans une file chiffrée sur le poste, dans leur ordre, comme les totaux des coûts, et partent au retour par lots (et tant que la clé est périmée ou absente). La file garde 10 000 faits ; au-delà, les plus vieux tombent et le lot suivant dit combien. Chaque fait garde l'heure du poste (`at`) ; l'émetteur ajoute l'heure où le serveur l'a reçu (`seen`). Un lot envoyé deux fois (une réponse perdue) ne compte qu'une fois : l'émetteur prend chaque fait une fois, par son id.
- **Rien ne s'affiche encore** : AU-4 branche les faits du poste et montre au membre ce qui part (Réglages › Équipe) ; AU-5 fait la machine qui les ouvre et les émet. Aucun outil du MCP ne lit ni ne dépose dans ce flux.
- Des tests le vérifient contre le vrai serveur, sur SQLite puis PostgreSQL : la paire par époque, l'enveloppe pour les émetteurs seuls, la rotation au départ d'un émetteur et à l'arrivée d'une machine, un membre qui ne peut ouvrir aucun fait, le serveur qui refuse sa lecture à un membre, la file hors ligne et sa reprise, `at` et `seen`, et les coûts inchangés.

## Ce que livre AU-1 : la crate `escouade-audit` (ESC-608)

La crate `crates/escouade-audit`, au workspace, sans dépendance au cœur de l'app ; son `README.md` détaille ce qui suit.

- **Le modèle** : `schema`, `id` (ULID), `event`, `severity` (1 à 5), `at` (l'heure du fait), `seen` (celle où l'émetteur l'a eu), `team`, `actor`, `target`, `detail`, `source` (`server`, `machine`, `device`), `emitter` (`device`, `seq`, `prev`), `anchor` (`stream`, `n`, `epoch`, `ct`), `sig`. `actor`, `target` et `detail` sont fermés par sorte d'événement : un champ inconnu, un événement hors de la liste, une sévérité qui n'est pas celle de l'événement font refuser la ligne. Les valeurs fermées sont des mots anglais stables (`desktop`, `phone`, `machine`, `recovery`, `admin`, `member`, `service`…).
- **La liste fermée** : les événements du §1 de l'étude, coupe-circuit (TS-15) et store privé (TS-14) compris, `request.too_large` venu avec AU-2, plus quatre : `device.enrolled` et `stream.written` (les mots du serveur seul : l'inscription et sa sorte de jeton, l'entrée écrite à un flux, dont l'ancre sert au recoupement), `audit.dropped` (le tampon) et `audit.checkpoint` (le point de contrôle de tête, écrit par AU-5). `permission.answered` est de sévérité 2, et 4 pour « Toujours autoriser ». La cible de `role.changed` est un membre (la machine) ou un appareil (le serveur). `escouade-audit events` imprime la liste.
- **Ce qui ne sort jamais** est écrit en dur. Les chaînes d'un événement sont des mots d'une grammaire fixe (ids, clés de tickets, clés de réglages, programmes, versions, empreintes, adresses IP) qui refusent une espace, un séparateur de chemin, un jeton ou une adresse e-mail. Seuls les noms (80 caractères au plus) et les raisons écrites par un administrateur (500) sont libres : un jeton, une valeur donnée à une variable (`TOKEN=…`), une suite aléatoire, un chemin ou une adresse e-mail y sont remplacés par `[retiré : ressemble à …]`, et l'émetteur compte ce qu'il a retiré. Les motifs de jetons sont ceux de la mémoire d'équipe : une seule liste, dans la crate, que la mémoire applique aussi. Une commande sort par son programme et l'empreinte SHA-256 de son texte (celle qu'approuve `.escouade.yml`) ; une branche (un passage, une validation) par son empreinte, son nom pouvant dire de quoi parle le ticket ; une valeur de réglage sensible par son empreinte, une exécutable par programme et empreinte.
- **Les écritures** : JSON lines sur la sortie standard, ou dans un fichier tourné par taille (100 Mio par défaut) et par jour (le jour UTC où l'émetteur a vu l'événement, le même après un redémarrage), renommé `<nom>-<jour>.<n>.jsonl` ; syslog RFC 5424 sur TCP ou TLS (RFC 5425, jamais UDP), `APP-NAME escouade`, `MSGID` = l'événement, les données structurées `[escouade@32473 …]` (le numéro de documentation de la RFC 5612, d'ici le numéro d'entreprise IANA), le JSON entier en message, la facilité 13 (journal d'audit) et la sévérité syslog `7 − sévérité` ; CEF en option (sévérité CEF = 2 × la nôtre, nos champs dans `cs1` à `cs6`, sans la signature). Un événement qui ferait plus de 8 Kio en syslog n'est pas scellé : l'émetteur le découpe.
- **Le tampon** : chaque événement est écrit sur le disque avant de partir ; si la sortie ne répond pas, il attend (7 jours ou 1 Gio par défaut), puis les plus vieux tombent et `audit.dropped` dit combien, de quand à quand et leurs numéros dans la chaîne. La reprise se fait à la position du dernier écrit (le numéro de la chaîne, la position de chaque flux lu), sans trou ni doublon de l'émetteur.
- **L'intégrité** : la signature Ed25519 de l'émetteur sur le JSON canonique (RFC 8785) sans `sig`, la chaîne (`seq` de 1 en 1, `prev` = l'empreinte du JSON canonique du précédent), le point de contrôle de tête.
- **`escouade-audit verify`** : sans réseau, sur un export JSON lines, avec les clés des émetteurs (`--keys`) et les points de contrôle lus du flux de l'équipe (`--checkpoints`) ; il vérifie chaque ligne, chaque signature, chaque chaîne (un trou qu'un `audit.dropped` dit n'en est pas un), chaque point de contrôle (une fin retirée se voit), et la première règle du recoupement (une même entrée dont l'empreinte du chiffré diffère entre deux sources). Les deux autres règles du recoupement viennent avec AU-5. Il répond 0 (export entier), 1 (un écart, avec son fichier et sa ligne) ou 2 (illisible).

## La rétention et le RGPD (A6-A, A7-A)

- L'équipe qui allume l'audit est responsable du traitement : elle héberge son serveur. Le registre d'Escouade ne change pas, sauf un jour pour un serveur hébergé par l'éditeur.
- Escouade ne garde qu'un tampon d'émission (7 jours ou 1 Gio) ; la durée est celle du SIEM (de 6 mois à 1 an recommandé). Les adresses IP sont en entier dans les événements du serveur, réglables (`--audit-ip=full|truncated|off`).
- Allumer l'audit écrit une entrée au journal. Chaque membre voit dans Réglages › Équipe que l'audit est allumé, et ce qui part de son poste.
- Le guide du site (AU-6, ESC-617) donne le modèle de fiche de registre et l'information du CSE.

## Les tickets

| Ticket | Ce qu'il construit | Après |
| --- | --- | --- |
| AU-1 (ESC-608), **fait** | la crate `escouade-audit` : le modèle, les événements, ce qui ne sort jamais, les écritures, le tampon, la signature, `verify` | ESC-588 |
| AU-2 (ESC-609), **fait** | le serveur émet seul : sa clé d'audit, ses options, les événements de ses routes et leurs ancres | AU-1 |
| AU-3 (ESC-610), **fait** | la clé d'audit des postes (le code de la clé des coûts rendu commun) et le flux des postes, la file hors ligne, les champs fermés de chaque fait | AU-1 |
| AU-4 (ESC-612) | les faits des postes, ce que voit le membre | AU-3, ESC-546, ESC-579 |
| AU-5 (ESC-614) | la machine d'audit, les noms, le recoupement ; l'inscription de la clé d'audit du serveur au journal des membres | AU-2, AU-4, OE-1 (fait), ESC-546, ESC-579 |
| AU-6 (ESC-617) | les guides du site, l'essai contre Splunk et Elastic | AU-5 |

## Ce qui attend l'utilisateur

L'essai contre Sentinel et Datadog (un abonnement Azure et un compte Datadog), et le numéro d'entreprise IANA (PEN) des données structurées de syslog (`32473`, le numéro de documentation, d'ici là : une seule constante à changer, `syslog::ENTERPRISE`).
