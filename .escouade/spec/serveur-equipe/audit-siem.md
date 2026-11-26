---
title: "L'audit vers un SIEM"
subject: "serveur-equipe"
sources: ["docs/TEAM-AUDIT.md (ESC-580, TS-21 : l'étude, les choix A1 à A8, le découpage AU-1 à AU-6)","Réponse de l'utilisateur du 2026-11-21 (demande a-23) : toutes les recommandations, A1-C, A2-A, A3-B, A4-A, A5-B, A6-A, A7-A, A8-A","docs/TEAM-SERVER.md §3.7, §3.8, §6, §9, §12, §13, §15","crates/escouade-team-server/README.md (Ce que le serveur voit et garde)","crates/escouade-team-crypto/src/beacon.rs (les balises de tête)","crates/escouade-team/README.md (Les coûts de l'équipe : la clé des administrateurs)","docs/registre-rgpd.md","crates/escouade-audit/README.md (AU-1, ESC-608 : ce qui est construit)","RFC 8785 (JCS), RFC 5424 et RFC 5425 (syslog sur TLS), RFC 5612 (le numéro 32473)"]
tickets: ["ESC-580","ESC-588","ESC-608","ESC-609","ESC-610","ESC-612","ESC-614","ESC-617","ESC-606"]
decisions: [264,275,265,266,267,268,346,342]
compared: null
---

# L'audit vers un SIEM

> **Décidé ; le modèle est construit (AU-1), rien n'est encore branché.** L'utilisateur a pris toutes les recommandations de l'étude d'ESC-580 (`docs/TEAM-AUDIT.md`) le 2026-11-21 (A1-C, A2-A, A3-B, A4-A, A5-B, A6-A, A7-A, A8-A). Cette page dit ce qui est décidé ; chaque partie se construit par un ticket, de AU-1 (ESC-608) à AU-6 (ESC-617). AU-1 est fait : la crate `escouade-audit` (plus bas). Le rôle de service dont dépend la machine d'audit est construit (OE-1, ESC-606). Le serveur, les postes et la machine d'audit n'émettent encore rien.

## Ce que c'est

Un journal d'audit de l'équipe, envoyé à son outil de sécurité (Splunk, Elastic, Microsoft Sentinel, Datadog…). Il est éteint d'office ; l'équipe l'allume, sur son serveur et sur une machine de l'équipe.

## Ce qui sort, ce qui ne sort jamais (A3-B, A8-A)

- **Sort** (une liste fermée d'événements, chacun avec un nom stable et une sévérité de 1 à 5) : les membres, les appareils, les rôles, les époques et les invitations ; les accès au serveur ; les réglages imposés, les signataires et les accords des postes ; le coupe-circuit ; le store privé et les plugins installés ; les prises et les passages de tickets ; les validations ; les autorisations données aux agents (par qui) ; les consignes de l'Overseer ; la mémoire validée ou mise en quarantaine ; les budgets franchis ; les refus du cœur.
- **Les noms** : les événements nomment les membres, les appareils et les projets, et donnent les clés des tickets, **jamais leurs titres**, descriptions ou critères.
- **Ne sort jamais** : le contenu chiffré (titres, descriptions, spec, décisions, documents, souvenirs), les secrets (jetons, clés, mots de secours, secret d'une invitation, code d'appairage), le texte des conversations, des prompts, des sorties, des diffs et des messages de commit, les noms de fichiers, le code, les comptes des harness. Une commande autorisée sort par **son programme et son empreinte**, jamais son texte. Ni chaque tour d'agent, ni les renouvellements de sessions et de baux.
- Chaque événement est une structure fermée, et un test cherche des données piégées dans tout ce qui sort (AU-1, ESC-608).

## Les sources (A1-C, A2-A)

- **Le serveur seul** (AU-2, ESC-609) : ce que voient ses routes, en ids opaques, signé par une clé d'audit du serveur tirée à `init`.
- **Une machine de l'équipe qui déchiffre** (AU-5, ESC-614), sous le **rôle de service** (OE-1, ESC-606, **fait**) ; un poste d'administrateur en recours, tant qu'il est ouvert. Elle nomme ce qui se passe dans le journal et les projets qu'elle tient, et dit ceux qu'elle ne suit pas.
  - Ce que donne OE-1 : un administrateur l'inscrit par un code court (« Inscrire une machine de l'équipe », puis `escouade-server team pair` sur la machine) ; elle a son propre id au registre, rattaché à l'équipe ; elle lit le flux de l'équipe et les projets qu'on lui ouvre, et ne signe jamais un membre, un rôle, un réglage, un coupe-circuit, un plugin privé, ni ne répond à une invitation (chaque poste le refuse, `policy.rs`) ; le serveur la refuse sur ses routes d'administrateur et sur les flux propres de l'équipe. La même machine peut porter l'Overseer d'équipe (voir « L'Overseer d'équipe »).
- **Les postes** (AU-3, ESC-610 ; AU-4, ESC-612) : ce qui n'arrive que là (refus du cœur, autorisations, commandes approuvées, plugins, coupe-circuit appliqué), scellé pour une clé d'audit à part et déposé sur le serveur ; la machine l'ouvre. Un poste ne parle jamais au SIEM, et rien ne sort d'un poste en clair. Un poste modifié peut taire ses faits.

## Les formats (A4-A)

Un modèle JSON unique (`escouade.audit/v1`), écrit en JSON lines (fichier tourné ou sortie standard) et en syslog RFC 5424 sur TLS, CEF en option ; OpenTelemetry par un Collector qui lit le fichier ; pas de connecteur propre à un SIEM au départ. Options `--audit-file`, `--audit-stdout`, `--audit-syslog`, `--audit-cef` sur le serveur et la machine ; un tampon sur disque si le SIEM ne répond pas (7 jours ou 1 Gio, puis `audit.dropped`).

## L'intégrité (A5-B)

Chaque événement est signé par son émetteur, chaîné au précédent, et ancré à l'entrée du journal du serveur d'où il vient (son numéro, son époque, l'empreinte de son chiffré). La tête de la chaîne est publiée chaque heure dans le flux de l'équipe, comme les balises de tête. `escouade-audit verify` vérifie un export et recoupe ce que dit le serveur avec ce que dit la machine.

## Ce que livre AU-1 : la crate `escouade-audit` (ESC-608)

La crate `crates/escouade-audit`, au workspace, sans dépendance au cœur de l'app ; son `README.md` détaille ce qui suit.

- **Le modèle** : `schema`, `id` (ULID), `event`, `severity` (1 à 5), `at` (l'heure du fait), `seen` (celle où l'émetteur l'a eu), `team`, `actor`, `target`, `detail`, `source` (`server`, `machine`, `device`), `emitter` (`device`, `seq`, `prev`), `anchor` (`stream`, `n`, `epoch`, `ct`), `sig`. `actor`, `target` et `detail` sont fermés par sorte d'événement : un champ inconnu, un événement hors de la liste, une sévérité qui n'est pas celle de l'événement font refuser la ligne. Les valeurs fermées sont des mots anglais stables (`desktop`, `phone`, `machine`, `recovery`, `admin`, `member`, `service`…).
- **La liste fermée** : les 53 événements du §1 de l'étude, coupe-circuit (TS-15) et store privé (TS-14) compris, plus quatre : `device.enrolled` et `stream.written` (les mots du serveur seul : l'inscription et sa sorte de jeton, l'entrée écrite à un flux, dont l'ancre sert au recoupement), `audit.dropped` (le tampon) et `audit.checkpoint` (le point de contrôle de tête, écrit par AU-5). `permission.answered` est de sévérité 2, et 4 pour « Toujours autoriser ». `escouade-audit events` imprime la liste.
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
| AU-2 (ESC-609) | le serveur émet seul | AU-1 |
| AU-3 (ESC-610) | la clé d'audit et le flux des postes | AU-1 |
| AU-4 (ESC-612) | les faits des postes, ce que voit le membre | AU-3, ESC-546, ESC-579 |
| AU-5 (ESC-614) | la machine d'audit, les noms, le recoupement | AU-2, AU-4, OE-1 (fait), ESC-546, ESC-579 |
| AU-6 (ESC-617) | les guides du site, l'essai contre Splunk et Elastic | AU-5 |

## Ce qui attend l'utilisateur

L'essai contre Sentinel et Datadog (un abonnement Azure et un compte Datadog), et le numéro d'entreprise IANA (PEN) des données structurées de syslog (`32473`, le numéro de documentation, d'ici là : une seule constante à changer, `syslog::ENTERPRISE`).
