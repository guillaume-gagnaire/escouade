# L'audit du serveur d'équipe vers un SIEM (TS-21) : étude

Étude du 2026-11-21 (ESC-580), pour la 2.3 ou la version d'après. Elle part de la conception du serveur d'équipe, qui fait foi : `docs/TEAM-SERVER.md`, surtout le §3.7 (le journal signé, devenu le groupe MLS par C9-B), le §3.8 (ce que voit le serveur), le §6 (les réglages imposés), le §9 (le coupe-circuit), le §12 (la piste) et le §15 (les choix tranchés). Rien n'est construit : ce document compare les voies, recommande, pose les choix A1 à A8 et découpe le travail en tickets AU-1 à AU-6.

Les coûts sont en **heures d'agent** Claude Code (un ticket de la 2.3 prend une à trois heures, six tournent en parallèle), jamais en jours de développeur. Ce qui attend l'utilisateur, ou des essais réels, est dit à part.

## En bref

- **Ce qui sort** : des événements de sécurité, en liste fermée. Ils couvrent les membres, les appareils et les rôles, les époques de clés, les invitations, les réglages imposés, le coupe-circuit, le store privé, les prises et les passages de tickets, les validations, les autorisations données aux agents, la mémoire validée ou mise en quarantaine, et les refus du cœur.
- **Ce qui ne sort jamais** : le contenu chiffré (titres, descriptions, spec, décisions, documents, souvenirs), les secrets (jetons, clés, mots de secours, secret d'invitation), le texte des conversations, des prompts, des réponses et des sorties, le code et les noms de fichiers. Un test le vérifie champ par champ, comme pour les coûts (TS-16).
- **Trois sources**, de la moins à la plus riche :
  - **S1, le serveur aveugle, seul** : ce que ses routes voient, avec des ids opaques. Il sait qu'un appareil s'inscrit, se connecte, est coupé, prend un ticket ou écrit un commit. Il ne sait ni qui, ni quel projet, ni ce que dit un commit MLS (il part en message privé).
  - **S2, une machine de l'équipe qui déchiffre** (un `escouade-server` en mode serveur, ou un poste d'administrateur) : elle lit le journal des membres, le fichier d'équipe, les coupe-circuits, le store privé et les flux des projets, et nomme ce qui s'y passe.
  - **S3, les postes** : ce qui n'arrive que sur un poste (un refus du cœur, une autorisation donnée, une commande approuvée, un plugin installé). Chaque poste le scelle pour une **clé d'audit** (une paire à part, comme la clé des coûts) et le dépose sur le serveur ; la machine d'audit l'ouvre et l'émet. Un poste ne parle jamais au SIEM.
- **Les formats** : un modèle d'événement unique, en JSON (`escouade.audit/v1`), écrit en **JSON lines** (un fichier tourné, ou la sortie standard) et en **syslog RFC 5424 sur TLS** (RFC 5425), avec CEF en option. Ces deux voies atteignent Splunk, Elastic, Sentinel et Datadog par leurs agents. OpenTelemetry passe par un Collector qui lit le fichier. Pas de connecteur propre à chaque SIEM au départ.
- **L'intégrité** : chaque événement est signé par son émetteur et chaîné au précédent. Il est **ancré** au journal du serveur (le numéro de l'entrée, son époque, l'empreinte de son chiffré) : un événement de S2 se recoupe avec celui de S1 pour la même entrée. La tête de la chaîne d'audit est publiée dans le flux de l'équipe, comme les balises de tête. Un outil, `escouade-audit verify`, vérifie un export.
- **La rétention** est celle du SIEM. Escouade ne garde qu'un tampon d'émission court. L'équipe est responsable du traitement (elle héberge son serveur) : l'étude lui donne le modèle de sa fiche de registre. Le registre d'Escouade (`docs/registre-rgpd.md`) ne change pas, sauf le jour d'un serveur hébergé par l'éditeur.
- **Le découpage** : 6 tickets, environ 14 h d'agent, en quatre vagues. En temps réel, une grosse journée avec la relecture, plus les essais contre de vrais SIEM.

## 1. Ce qu'une équipe veut suivre

Un RSSI veut répondre à quelques questions : qui est entré, avec quels appareils, qui a eu quels droits, qui a changé une règle, qui a arrêté quoi, qui a laissé un agent faire quoi, et si quelqu'un a tenté de contourner une règle. Les événements ci-dessous y répondent. Chacun a un nom stable (`famille.action`), une source et une sévérité (de 1, information, à 5, alerte), reprise telle quelle en syslog et en CEF.

### 1.1 Les membres, les appareils et les rôles (le journal signé)

| Événement | Source | Sév. | Ce qu'il dit |
| --- | --- | --- | --- |
| `team.created` | S2 (S1 : `device.enrolled` du jeton d'`init`) | 3 | la genèse, le propriétaire, l'empreinte de l'équipe en six mots |
| `member.added`, `member.removed` | S2 | 3 | le membre, son rôle, l'administrateur qui l'a signé, l'époque ouverte |
| `device.added` | S2 (S1 : `device.enrolled`) | 2 | l'appareil, sa sorte (poste, téléphone, machine), son membre, qui l'a admis (un administrateur, ou un appareil du même membre) |
| `device.revoked` | S2 et S1 | 3 | l'appareil, son membre, qui l'a coupé, la raison écrite si elle existe |
| `role.changed` | S2 (S1 : le rôle du serveur) | 4 | l'ancien rôle, le nouveau, qui l'a signé |
| `epoch.opened` | S2 (S1 : le numéro de l'époque) | 2 | l'époque, la cause (départ, révocation, rotation à la demande) |
| `invitation.created`, `invitation.used` | S2 | 2 | le rôle et les projets offerts, l'expiration, qui invite ; jamais le lien |
| `invitation.reused` | S1 | 5 | un jeton d'invitation servi une seconde fois (l'alerte du serveur, `token_used`) |
| `recovery.used` | S1 (`recovery-token`) et S2 | 5 | la clé de secours a servi |
| `escrow.set`, `escrow.removed` | S2 | 4 | la clé de séquestre de l'organisation (C7-B) |
| `team.fork`, `team.rollback` | S2 et S3 | 5 | un appareil a vu deux versions de l'équipe, ou un retour en arrière (l'alerte de fourche) |

### 1.2 Les accès au serveur

| Événement | Source | Sév. | Ce qu'il dit |
| --- | --- | --- | --- |
| `session.opened` | S1 | 1 | un appareil a signé le défi. Une fois par appareil et par adresse, puis une fois par jour : pas à chaque renouvellement de 15 min |
| `session.refused` | S1 | 3 | un défi mal signé, un appareil révoqué ou inconnu |
| `rate.limited` | S1 | 3 | une adresse ou un appareil au-delà de sa limite (429), une fois par minute au plus |
| `client.too_old` | S1 | 1 | un client refusé par la poignée de main des versions |
| `project.access.changed` | S1 et S2 | 3 | qui a accès à un projet (ouvert à l'équipe, ou une liste d'appareils) |

### 1.3 Les réglages imposés et les accords des postes

| Événement | Source | Sév. | Ce qu'il dit |
| --- | --- | --- | --- |
| `settings.changed` | S2 | 4 | la nouvelle version du `.escouade.yml` d'équipe : les clés changées, leur sorte (restreindre, valeur, sensible, exécutable), les anciennes et nouvelles valeurs, les signataires, la raison écrite |
| `settings.signers.changed` | S2 | 5 | la liste des signataires des réglages, ou le quorum (C12-B) |
| `settings.sensitive.answered` | S3 | 4 | un membre a accepté ou refusé une clé sensible sur son poste (un proxy, un fichier d'autorités) |
| `command.approved`, `command.revoked` | S3 | 3 | une commande proposée par l'équipe ou par le dépôt (`worktrees.setup`, `launch`…) approuvée sur un poste, à son empreinte |
| `policy.applied` | S3 | 2 | une stratégie du poste géré lue ou changée (ADMX, profil MDM, `/etc`) : les clés, pas les valeurs d'un proxy |
| `lock.refused` | S3 | 4 | le cœur a refusé d'écrire un réglage verrouillé (par la fenêtre, le MCP, un `.escouade.yml` du dépôt, un export) |

### 1.4 Le coupe-circuit (TS-15)

| Événement | Source | Sév. | Ce qu'il dit |
| --- | --- | --- | --- |
| `killswitch.raised`, `killswitch.lifted` | S2 | 5 | la portée (l'équipe, un projet, un harness, un modèle, un plugin, tous les agents), la raison, le `serial`, l'administrateur |
| `killswitch.applied` | S3 | 3 | un poste l'a appliqué : combien d'agents il a arrêtés, à quelle heure. Ce qui manque au bout d'une heure dit les postes hors ligne |

### 1.5 Le store privé (TS-14)

| Événement | Source | Sév. | Ce qu'il dit |
| --- | --- | --- | --- |
| `store.namespace.declared` | S2 | 4 | l'espace de noms de l'équipe (`fr.acme.*`) et la clé publique qui le signe |
| `store.plugin.published` | S2 | 3 | l'id, la version, l'empreinte de l'archive, les permissions demandées, qui a signé |
| `store.plugin.revoked` | S2 | 4 | une entrée de la liste de révocation de l'équipe, son `serial` |
| `plugin.installed`, `plugin.removed`, `plugin.updated` | S3 | 2 | sur un poste : l'id, l'origine (store public vérifié, store privé, dépôt git), la version, les permissions accordées |
| `plugin.refused` | S3 | 4 | une installation refusée par la politique de l'équipe ou du poste |

### 1.6 Les tickets, les agents et leurs autorisations

| Événement | Source | Sév. | Ce qu'il dit |
| --- | --- | --- | --- |
| `claim.taken`, `claim.released`, `claim.expired` | S1 (id opaque), S2 (la clé du ticket) | 1 | qui tient le ticket, sous quel jeton. Jamais les renouvellements du bail |
| `claim.fenced` | S1 | 3 | une écriture refusée sous un jeton dépassé (`stale_token`) : un poste revenu d'une coupure |
| `handoff.offered`, `handoff.taken`, `handoff.withdrawn` | S1 et S2 | 2 | le passage d'un agent (TS-17) : de qui, à qui (un collègue, une machine), la branche poussée |
| `ticket.approved`, `ticket.rejected` | S3 | 2 | la validation : par qui (un membre, l'Overseer sous une consigne), ce qu'elle a fait (fusion, PR), la branche cible |
| `permission.answered` | S3 | 2, 4 si « Toujours autoriser » | l'autorisation donnée ou refusée à un agent : par qui (le membre, l'Overseer, une règle), l'outil, le programme lancé et l'empreinte de la commande (A8) |
| `agent.mode.changed` | S3 | 3 | un agent passé dans un mode plus large (`acceptEdits`, Bypass quand il est permis) |
| `directive.added`, `directive.revoked` | S3 (S2 pour l'Overseer d'équipe, `docs/TEAM-OVERSEER.md`) | 3 | une consigne durable de l'Overseer : ce qu'elle permet, sa portée, le message qui l'a donnée (son id, pas son texte) |
| `memory.accepted`, `memory.quarantined` | S2 | 3 | un souvenir d'agent validé, ou une mise en quarantaine en masse (« tout ce qu'a écrit DEM-41 ») : l'id, la provenance, jamais le texte |
| `budget.crossed` | S2 | 3 | un budget de TS-16 passé à 80 % ou à 100 %, et le pilote auto arrêté : le budget et le pourcentage, pas le détail |

**Hors de la liste, volontairement** : chaque tour d'agent, chaque lecture de ticket, chaque opération de synchronisation. Un SIEM n'en fait rien, et c'est l'activité de travail de chacun (voir §6).

## 2. Ce qui ne sort jamais

Une liste écrite en dur dans la crate d'audit, que ni l'équipe ni un réglage ne peut changer :

- **le contenu chiffré** : titre, description et critères d'un ticket, pages de spec, décisions, documents et leurs noms, texte d'un souvenir, nom et logo d'un projet hors de son nom court (A3) ;
- **les secrets** : jetons de session, d'inscription et de plugin, clés d'appareil, de projet, d'équipe, d'audit et des coûts, les 24 mots de secours, la graine du séquestre, le secret après le `#` d'une invitation, le code court d'un appairage ;
- **le travail** : le texte des conversations, des prompts, des réponses, des sorties de commandes, des diffs et des messages de commit, les noms de fichiers, le code ;
- **les comptes des harness** : ni le compte, ni l'e-mail du compte, ni son forfait ;
- **le texte entier d'une commande** que l'on autorise (A8) : seulement son programme (`git`, `npm`) et son empreinte. Une commande porte souvent un secret en clair (`TOKEN=… npm publish`).

**Comment c'est tenu** :

- chaque événement est une structure Rust fermée (`deny_unknown_fields`), comme les totaux des coûts ; une chaîne libre n'existe que pour les raisons écrites par un administrateur (un coupe-circuit, un réglage) et les noms ;
- avant l'émission, les chaînes passent par les motifs de jetons qui refusent déjà les secrets dans la mémoire d'équipe (TS-10). Une chaîne qui en contient un est remplacée par `[retiré : ressemble à un secret]`, et l'émetteur le dit ;
- un test de la crate parcourt chaque événement de chaque sorte, sur des données piégées (un titre, un jeton, un nom de fichier mis exprès dans les objets), et cherche ces piégés dans ce qui sort, sur les trois formats.

## 3. Les sources

### 3.1 S1, le serveur aveugle, seul

Il ne lit rien, mais ses routes voient des faits utiles. Il peut les émettre **sans aucune clé**, dès qu'on l'allume (`--audit`) :

- l'inscription d'un appareil (`/v1/enroll`), avec la sorte du jeton (`init`, invitation, récupération) ;
- l'ouverture et le refus d'une session (le défi signé), et l'adresse IP (A6) ;
- la révocation d'un appareil, le changement de son rôle sur le serveur ;
- le second usage d'un jeton d'invitation, le jeton de récupération affiché par `recovery-token` ;
- un commit au flux de l'équipe ou d'un projet : son numéro, son époque, l'appareil auteur, l'empreinte du chiffré ;
- les droits d'un projet (`/v1/projects/{id}/access`) ;
- les prises : prise, rendu, bail expiré, passage (`hand`, `withdraw`), écriture refusée sous un jeton dépassé ;
- les limites de débit, les clients trop vieux, les tailles refusées.

**Ce qu'il ne peut pas dire** : le nom d'un membre ou d'un projet, la clé d'un ticket, ce que fait un commit (ajouter ou retirer un membre, changer un rôle au registre, lancer un coupe-circuit, publier un réglage), ni rien de ce qui se passe sur un poste. Ses événements portent donc des ids opaques. Pour un SIEM, c'est déjà beaucoup : un appareil inconnu, une rafale de refus, une connexion depuis un pays inattendu, un retour du jeton de récupération.

**Sa clé d'audit** : le serveur n'a aujourd'hui aucune clé. Il en gagne une, Ed25519, tirée à `init` et gardée dans son dossier de données, qui ne sert qu'à signer ses événements (§5). Un administrateur l'inscrit au journal des membres (« la clé d'audit du serveur »), pour qu'un vérificateur sache que c'est bien elle.

### 3.2 S2, une machine de l'équipe qui déchiffre

C'est la seule façon de nommer les choses sans casser le bout en bout (C10-A) : un appareil de l'équipe, qui a les clés, lit et traduit. C'est l'idée du §13 : une machine en mode serveur, membre de l'équipe.

- **Ce qu'elle lit** : le groupe MLS de l'équipe (membres, appareils, rôles, époques), l'annuaire chiffré (les noms), le `.escouade.yml` d'équipe et sa chaîne de versions, les entrées du coupe-circuit, la déclaration du store privé et sa liste de révocation, les offres de passage, la mémoire d'équipe (provenance et quarantaine), le résumé des coûts. Pour les tickets, les flux des projets dont elle a la clé.
- **Ce qu'elle émet** : les mêmes faits que S1, nommés (le membre, l'appareil, le projet, la clé du ticket), plus ceux que S1 ne voit pas (`settings.changed`, `killswitch.raised`, `member.added`…). Chaque événement porte l'ancre de l'entrée du serveur d'où il vient (§5).
- **Son rôle** (A2) : elle n'a besoin que de lire. Elle ne signe ni réglage, ni coupe-circuit, ni membre. D'où le rôle de **service** recommandé, à créer avec TS-4 : un appareil de l'équipe, rattaché à l'équipe et pas à un membre, qui lit ce qu'on lui ouvre et ne signe rien d'administratif. Le même rôle porte l'Overseer d'équipe (`docs/TEAM-OVERSEER.md`, O2) ; une même machine peut faire les deux.
- **Les projets gardés à quelques membres** : elle ne voit que ceux où elle est. Ses événements de tickets s'arrêtent donc là où s'arrêtent ses clés, et l'écran de l'audit le dit (« 3 projets sur 12 ne sont pas suivis : la machine d'audit n'y a pas accès »). S1 continue d'émettre leurs prises, sous des ids opaques.
- **Sans machine** : un poste d'administrateur peut tenir ce rôle tant qu'il est ouvert. Ce qui s'est passé pendant qu'il était fermé s'émet à son retour, dans l'ordre du journal, avec l'heure de l'entrée.

### 3.3 S3, les postes

Certains faits n'existent que sur un poste : le cœur qui refuse un réglage verrouillé, un membre qui autorise une commande, une commande approuvée à son empreinte, un plugin installé, une clé sensible acceptée, un coupe-circuit appliqué, une validation de ticket.

- **Trois façons de les faire sortir** :
  - **chaque poste envoie au SIEM** : il faut alors que le SIEM soit joignable de chaque portable, avec un jeton sur chacun, et l'équipe ne sait plus ce qui sort de quel poste. Écarté ;
  - **chaque poste écrit en clair dans un flux du serveur** : le serveur lirait alors ce que fait chacun, contre C10-A. Écarté ;
  - **chaque poste scelle le fait pour une clé d'audit**, le dépose dans un flux `escouade/audit` du serveur, et la machine d'audit l'ouvre et l'émet. **Recommandé.**
- **La clé d'audit** : une paire HPKE à part, sur le modèle de la clé des coûts (`costkey.rs`) : une paire par époque, enveloppée pour les seuls appareils qui émettent l'audit (le rôle de service, ou l'administrateur qui le tient), tournée quand l'un d'eux part. Un membre ne lit pas les faits des autres ; le serveur non plus.
- **Hors ligne** : les faits attendent dans une file, comme les totaux des coûts, et partent au retour. Chacun garde l'heure du poste (`at`) ; l'émetteur ajoute l'heure où le serveur l'a reçu (`seen`).
- **Ce que le membre voit** : comme pour les coûts, « Ce que l'audit de l'équipe reçoit de ce poste », dans Réglages › Équipe : la liste des sortes d'événements, et les derniers partis.
- **La confiance** : un poste modifié peut taire ses faits. L'audit des postes dit ce que font des postes honnêtes, il ne prouve pas l'absence de ce qu'un poste piraté cache. Les faits de S1 et S2 (le serveur, le journal) ne dépendent pas de lui.

**Choix A1 : les sources.**

- **A. Le serveur seul (S1).** Coût : environ 2 h. Rien ne demande de machine, mais tout est en ids opaques, et rien ne dit ce que l'équipe a décidé (réglages, coupe-circuit, membres).
- **B. Le serveur et une machine qui déchiffre (S1 et S2).** Coût : environ 7 h. Les noms, et tout ce qui passe par le journal. Rien de ce qui n'arrive que sur un poste.
- **C. B, plus les faits des postes scellés pour la clé d'audit (S3).** Coût : environ 12 h (sans les guides). Tout le §1.

**Recommandation : C**, en vagues : S1 d'abord (il sert seul, et sert de témoin aux deux autres), puis S2 et S3. Les refus du cœur et les autorisations données sont ce qu'un RSSI demande en premier après les membres.

**Choix A2 : qui déchiffre.**

- **A. Une machine en mode serveur, sous un rôle de service nouveau** (lit, ne signe rien d'administratif). Coût : environ 1 h 30 pour le rôle, partagé avec l'Overseer d'équipe.
- **B. Un appareil d'administrateur** (poste ou machine). Coût : nul de plus. Mais une machine qui tourne jour et nuit avec les droits d'un administrateur est une cible : sa clé signe des réglages et des membres.
- **C. N'importe quel appareil membre.** Coût : nul, mais un membre lirait alors les faits de tous les postes.

**Recommandation : A**, avec B en recours tant que l'équipe n'a pas de machine.

## 4. Les formats et les voies

### 4.1 Un modèle d'événement, plusieurs écritures

Un seul modèle, en JSON, versionné (`schema: "escouade.audit/v1"`), dont les champs sont :

```json
{
  "schema": "escouade.audit/v1",
  "id": "01JD7…",
  "event": "settings.changed",
  "severity": 4,
  "at": "2026-11-21T10:42:07.120Z",
  "seen": "2026-11-21T10:42:07.410Z",
  "team": { "id": "9f2c…", "name": "Acme" },
  "actor": { "member": "m-3c1a…", "name": "Claire Martin", "device": "d-77e0…", "kind": "poste" },
  "target": { "keys": ["agents.engines", "permissions.modes"] },
  "detail": { "version": 12, "signers": ["Claire Martin", "Paul Leroy"], "reason": "Seuls les fournisseurs sous contrat DPA." },
  "source": "machine",
  "emitter": { "device": "d-0b5e…", "seq": 48213, "prev": "sha256:5c0e…" },
  "anchor": { "stream": "team", "n": 1287, "epoch": 41, "ct": "sha256:aa91…" },
  "sig": "ed25519:Jx3…"
}
```

- `at` est l'heure du fait (celle de l'entrée, ou du poste) ; `seen`, celle où l'émetteur l'a eu.
- `actor`, `target` et `detail` sont fermés par sorte d'événement. Les noms (`name`) ne viennent que de S2 (A3).
- `emitter`, `anchor` et `sig` portent l'intégrité (§5).
- **Une correspondance vers OCSF** (Open Cybersecurity Schema Framework : les classes « Account Change », « Authentication », « Entity Management », « API Activity ») est écrite dans le guide, en table, pour les SIEM qui normalisent en OCSF. On ne l'émet pas : OCSF change encore de version, et notre modèle est plus petit.

### 4.2 Les formats

| Format | Ce que c'est | Pour | Contre | Coût |
| --- | --- | --- | --- | --- |
| **JSON lines** | un événement JSON par ligne, dans un fichier tourné ou sur la sortie standard | tout SIEM le lit par son agent (Splunk Universal Forwarder, Elastic Agent ou Filebeat, Datadog Agent, l'agent Azure Monitor pour Sentinel), l'OpenTelemetry Collector aussi (`filelog`) ; rien à configurer côté Escouade ; la sortie standard sert Docker et journald | il faut un agent sur la machine qui émet | 1 h, dans AU-1 |
| **syslog RFC 5424** sur TLS (RFC 5425) | l'en-tête syslog (sévérité, heure, hôte, `APP-NAME escouade`, `MSGID` = l'événement), quelques paramètres en données structurées (`[escouade@<PEN> team="…" event="…"]`), le JSON entier en message | le chemin classique des SIEM : rsyslog et syslog-ng, Splunk Connect for Syslog, l'agent Azure Monitor (Sentinel), QRadar ; pas d'agent à poser si un collecteur syslog existe | la taille d'un message (on reste sous 8 Kio) ; UDP perd des messages, donc TCP et TLS seulement ; un numéro d'entreprise IANA (PEN) à demander, gratuit | 1 h 30, dans AU-1 et AU-2 |
| **CEF** (ArcSight) | `CEF:0\|Escouade\|escouade-team\|2.3\|settings.changed\|…\|4\|` puis des clés `suser`, `duser`, `src`, `cs1`… sur syslog | les connecteurs CEF de Sentinel (CEF par l'agent Azure Monitor), ArcSight, QRadar ; des champs déjà rangés | un vocabulaire fermé : nos champs vont dans `cs1` à `cs6`, et le reste se perd ou se colle en `msg` | 30 min de plus, une option de la sortie syslog |
| **OpenTelemetry Logs** (OTLP) | des `LogRecord` en gRPC ou HTTP, vers un Collector ou un SIEM qui le parle | la voie qui monte : Elastic, Datadog, Splunk et Azure Monitor ont un récepteur OTLP ou un exportateur du Collector | une pile de plus dans le binaire (`opentelemetry-otlp`, tonic ou reqwest) pour un gain faible : le Collector lit déjà le JSON lines ; la sémantique des logs d'audit n'y est pas encore fixée | 2 h ; **pas au départ**, le Collector à la place |
| **Webhook** | un POST HTTPS de JSON, signé (HMAC) | une alerte vers un salon, un outil maison | pas une voie de SIEM : pas d'ordre garanti, des reprises à inventer, un secret de plus ; les plugins ont déjà `contributes.webhooks` pour les faits des postes | 1 h ; **non**, sauf une voie d'alerte plus tard |
| **Une API par SIEM** (Splunk HEC, Elastic Bulk, Sentinel Logs Ingestion, Datadog HTTP) | un client par produit, avec son jeton | aucun agent à poser | quatre clients à tenir, quatre jetons à garder sur la machine | 1 h à 1 h 30 chacun ; **plus tard**, Splunk HEC d'abord si on le demande |

### 4.3 Les SIEM

| SIEM | Par JSON lines | Par syslog | Par OTLP (Collector) | À essayer (AU-6) |
| --- | --- | --- | --- | --- |
| **Splunk** | le Universal Forwarder lit le fichier (`sourcetype` JSON) | Splunk Connect for Syslog | exportateur `splunk_hec` du Collector | Splunk Enterprise en conteneur, licence d'essai |
| **Elastic** | Elastic Agent ou Filebeat, `filestream` et l'analyseur `ndjson` | l'entrée syslog d'Elastic Agent | exportateur `elasticsearch`, ou l'entrée OTLP d'Elastic | Elasticsearch et Kibana en conteneurs |
| **Microsoft Sentinel** | l'agent Azure Monitor lit un fichier JSON (une règle de collecte, une table `_CL`) | syslog ou CEF par l'agent Azure Monitor, le chemin documenté des connecteurs | pas de voie directe simple : le Collector écrit un fichier ou du syslog, que lit l'agent Azure Monitor | un espace Log Analytics : **demande un abonnement Azure, donc l'utilisateur** |
| **Datadog** | l'Agent suit le fichier (`logs_config`) | l'écoute TCP de l'Agent | l'ingestion OTLP de l'Agent | un compte d'essai : **l'utilisateur** |

Les noms exacts des récepteurs et des règles se vérifient aux essais : ils changent avec les versions des agents.

**Le coût côté SIEM** : un SIEM se paie au volume. Un membre actif fait de l'ordre de 50 à 200 événements par jour (les sessions comptées une fois par jour et par adresse, les prises sans leurs renouvellements), soit environ 100 Kio. Une équipe de 100 membres envoie donc de l'ordre de 10 Mio par jour, 300 Mio par mois : quelques dollars par mois au tarif au gigaoctet des SIEM. Les sessions renouvelées toutes les 15 minutes et les baux toutes les 3 minutes, s'ils sortaient, multiplieraient ce volume par vingt : c'est pourquoi ils ne sortent pas.

**Choix A4 : le format et les voies.**

- **A. Le modèle JSON unique, en JSON lines et en syslog RFC 5424 sur TLS, CEF en option ; OpenTelemetry par le Collector, documenté ; aucune API propre à un SIEM au départ.** Coût : environ 2 h 30 dans AU-1 et AU-2.
- **B. OpenTelemetry seul, en OTLP natif.** Coût : environ 2 h. Une voie, mais une pile de plus, et Sentinel ou un collecteur syslog demandent alors un Collector entre les deux.
- **C. Un connecteur par SIEM** (HEC, Bulk, Logs Ingestion, HTTP Datadog). Coût : environ 5 h, et quatre jetons sur la machine.

**Recommandation : A.** Le JSON lines et syslog couvrent les quatre SIEM par leurs agents, sans jeton dans Escouade ; Splunk HEC viendra si une équipe le demande.

### 4.4 Où se branche la sortie

- **Le serveur** (S1) : `escouade-team-server serve --audit-file <chemin>` (tourné par taille et par jour), `--audit-stdout`, `--audit-syslog tls://siem.acme.fr:6514` (avec `--audit-syslog-ca`), `--audit-cef`. Éteint par défaut.
- **La machine d'audit** (S2 et S3) : les mêmes options sur `escouade-server team audit`, ou, sur un poste d'administrateur, Réglages › Équipe › Audit (le chemin du fichier, l'adresse syslog et son certificat).
- **Les deux sorties peuvent viser le même SIEM** : c'est ce qui permet le recoupement (§5.3).
- **La reprise** : l'émetteur garde la position du dernier événement écrit (le numéro de chaque flux lu, le `seq` de sa chaîne). Il reprend là, sans trou ni doublon de son côté ; un SIEM déduplique par `id` ce qu'une coupure aurait fait renvoyer.
- **Le tampon** : si syslog ne répond pas, les événements attendent sur le disque (7 jours ou 1 Gio par défaut). Au-delà, les plus vieux tombent et un événement `audit.dropped` le dit, avec combien et de quand à quand.

## 5. L'intégrité : chaque événement rattaché au journal

### 5.1 Ce qu'un SIEM doit pouvoir montrer

Qu'un événement vient bien d'Escouade, qu'il n'a pas été changé ni retiré en route, et qu'il correspond à ce qui s'est vraiment passé dans l'équipe. Un SIEM garde ce qu'il reçoit ; il ne sait pas vérifier une signature dans ses requêtes. La vérification se fait donc en deux temps : des règles simples dans le SIEM (des trous, des points de contrôle manquants), et un outil qui vérifie tout sur un export.

### 5.2 Les quatre liens

1. **La signature de l'émetteur.** Chaque événement est signé (Ed25519) par l'appareil qui l'émet : la clé du serveur (§3.1), ou la clé d'appareil de la machine d'audit, inscrite au journal. Ce qui est signé : le JSON canonique de l'événement (RFC 8785, JCS), sans `sig`.
2. **La chaîne de l'émetteur.** Chaque événement porte son numéro (`emitter.seq`) et l'empreinte du précédent (`emitter.prev`). Un événement retiré au milieu, ou changé, casse la chaîne.
3. **L'ancre au journal du serveur.** Un événement né d'une entrée d'un flux (un commit MLS, une opération, un fait scellé d'un poste, une prise) porte `anchor` : le flux, le numéro de l'entrée, son époque, et l'empreinte SHA-256 de ses octets chiffrés. Le serveur a les mêmes octets : S1 émet, pour la même entrée, la même empreinte.
4. **La tête publiée dans le journal de l'équipe.** Toutes les heures, et à chaque fermeture, l'émetteur écrit un point de contrôle `audit.checkpoint` (son `seq` et l'empreinte de son dernier événement) **dans sa sortie et dans le flux de l'équipe**, signé par sa feuille, comme une balise de tête (`beacon.rs`). Tous les membres le reçoivent : retirer la fin d'un export se voit, puisque la tête est ailleurs.

Ce sont les principes de la transparence des journaux (un journal en ajout seul, des têtes publiées) et des balises de tête qu'Escouade a déjà. Rien n'est inventé.

### 5.3 Le recoupement de S1 et S2

Pour une même entrée du journal, le serveur (S1) dit « l'appareil d-77e0 a écrit l'entrée 1287 du flux de l'équipe, époque 41, chiffré `aa91…` », et la machine (S2) dit « Claire Martin a changé deux réglages, entrée 1287, chiffré `aa91…` ». Le SIEM les joint par `(anchor.stream, anchor.n)` :

- **une empreinte qui diffère** : l'un des deux ment (un serveur qui a servi d'autres octets à la machine) ;
- **un événement de S2 sans entrée de S1** : la machine invente, ou S1 n'émettait pas encore ;
- **une entrée de S1 sans événement de S2** pendant plus d'une heure : la machine ne suit plus, ou n'a pas la clé de ce projet (ce qu'elle dit par ailleurs).

Le guide (AU-6) donne ces trois règles en SPL (Splunk), KQL (Sentinel), ES|QL (Elastic) et en requête Datadog.

### 5.4 Ce que vérifie `escouade-audit verify`

Un petit binaire (Rust, sans réseau), livré avec le serveur, que le RSSI lance sur un export du SIEM, ou que l'on programme chaque nuit :

- chaque signature, contre les clés d'émetteur inscrites (`--keys`, la liste que la machine d'audit exporte depuis le journal des membres, elle-même signée) ;
- chaque chaîne, sans trou ni fourche ;
- chaque point de contrôle, contre la tête publiée (`--checkpoints`, ceux que la machine a lus du flux de l'équipe) ;
- le recoupement de S1 et S2 du §5.3.

Il rend un rapport : « 48 213 événements, 2 émetteurs, chaînes entières, 0 écart » ou chaque écart avec sa place.

### 5.5 Ce que l'intégrité ne prouve pas

- **Le contenu d'un commit MLS** : la machine l'a déchiffré et vérifié (MLS, puis les règles de `policy.rs`) avant de l'émettre. Un vérificateur extérieur, sans les clés, la croit sur ce point : il vérifie qu'elle a signé, pas que MLS l'aurait accepté.
- **Ce qu'un poste piraté tait** (§3.3).
- **Une machine d'audit prise** peut signer de faux événements ; le recoupement avec S1 la trahit pour tout ce qui passe par le serveur, pas pour les faits des postes qu'elle seule ouvre.

**Choix A5 : l'intégrité.**

- **A. Signature, chaîne et ancre** (les liens 1 à 3). Coût : environ 1 h 30.
- **B. A, plus la tête publiée dans le flux de l'équipe** (le lien 4) et `escouade-audit verify`. Coût : environ 1 h de plus.
- **C. Rien de plus que TLS vers le SIEM.** Coût : nul, mais l'audit ne prouve rien face à un administrateur système du SIEM ou du serveur.

**Recommandation : B.** Le lien 4 réutilise les balises de tête, et sans l'outil personne ne vérifie les signatures.

## 6. La rétention et le RGPD

### 6.1 Qui est responsable

- **Le serveur d'équipe s'héberge par l'équipe** (§15.1). L'organisation qui l'installe est **responsable du traitement** de son audit : elle choisit de l'allumer, de l'envoyer à son SIEM, et sa durée. GAGNAIRE DEVELOPPEMENT n'en reçoit rien : `docs/registre-rgpd.md` **ne change pas**.
- **Le jour d'un serveur hébergé par l'éditeur** (l'idée du 2026-11-14), l'éditeur devient **sous-traitant** de l'équipe pour S1 : le registre gagne alors une section, avec le contrat de sous-traitance (article 28). Ce n'est pas l'objet de cette étude.

### 6.2 Ce que c'est, pour les membres

- **Des données personnelles** : un membre et ses appareils, ses heures de connexion, ses adresses IP, ses prises de tickets, ce qu'il a autorisé à ses agents. C'est un **contrôle de l'activité des salariés** : une finalité de sécurité, une base d'intérêt légitime (article 6.1.f), une information préalable de chaque membre, et, en France, une information et une consultation du CSE avant la mise en œuvre (Code du travail, L. 2312-38).
- **La finalité** s'écrit dans la fiche : la sécurité du serveur et des projets de l'équipe, la détection des abus et des intrusions, l'enquête après un incident. **Pas** la mesure du travail : c'est pourquoi les tours d'agent, les lectures et le volume de travail ne sortent pas, et pourquoi le grain des coûts reste celui de C15.
- **Chaque membre le voit** dans l'app : une ligne de Réglages › Équipe, « Cette équipe envoie un journal d'audit à son outil de sécurité », la liste des sortes d'événements, et ce qui est parti de son poste (§3.3). Allumer l'audit écrit au journal une entrée que chaque membre voit, comme le séquestre (C7-B).

### 6.3 Les durées

- **Escouade ne garde rien d'autre que son tampon** : 7 jours ou 1 Gio par défaut, tant que le SIEM n'a pas reçu (§4.4).
- **La durée est celle du SIEM**, réglée par l'équipe. La recommandation donnée dans le guide : **de 6 mois à 1 an** pour les journaux de sécurité, ce que recommande la CNIL pour la journalisation (sa recommandation du 14 octobre 2021) ; plus long seulement sur une obligation écrite.
- **Les adresses IP** (A6) : la donnée la plus utile à un SIEM (un pays inattendu, une adresse connue pour attaquer), et la plus personnelle.
- **Le droit d'accès d'un membre** passe par l'équipe, qui cherche son id dans son SIEM. Le droit à l'effacement cède devant la finalité de sécurité pendant la durée choisie (article 17.3) : la fiche le dit.

### 6.4 Le modèle de fiche pour l'équipe

AU-6 livre, sur le site, une fiche de registre à recopier, sur le modèle de `docs/registre-rgpd.md` : la finalité, la base légale, les personnes, la liste des données (le §1, et le §2 pour ce qui n'y est jamais), les destinataires (l'équipe de sécurité), les sous-traitants (l'éditeur du SIEM, son hébergeur), les transferts hors UE (selon le SIEM : à dire par l'équipe), la durée, la sécurité (le §5), et l'information des membres et du CSE.

**Choix A6 : les adresses IP.**

- **A. En entier dans les événements du serveur**, l'option `--audit-ip=full|truncated|off` pour la changer. Coût : compris dans AU-2.
- **B. Tronquées d'office** (un /24 en IPv4, un /48 en IPv6) : le pays et l'opérateur restent, la personne moins. Coût : le même.
- **C. Jamais.** Coût : nul, mais la moitié de l'intérêt d'un SIEM part avec.

**Recommandation : A**, puisque l'équipe allume elle-même l'audit et règle sa durée ; le guide dit d'en tenir compte dans la fiche.

**Choix A7 : ce qu'Escouade garde.**

- **A. Le tampon d'émission seul** (7 jours, 1 Gio), la durée est celle du SIEM. Coût : compris.
- **B. Une archive signée tenue par la machine d'audit**, N mois, rejouable vers un nouveau SIEM. Coût : environ 1 h 30, et une seconde copie des données à protéger et à purger.

**Recommandation : A.** Un SIEM garde, et le recoupement suffit à le contrôler.

## 7. Les choix à trancher

| N° | Question | Options | Recommandation |
| --- | --- | --- | --- |
| A1 | Les sources | A le serveur seul · B le serveur et une machine qui déchiffre · C B et les faits des postes scellés | C, en vagues |
| A2 | Qui déchiffre | A une machine sous un rôle de service · B un appareil d'administrateur · C tout appareil membre | A, B en recours |
| A3 | Ce que nomment les événements | A des ids opaques seuls · B les noms des membres, des appareils et des projets, les clés des tickets, jamais leurs titres · C B et les titres des tickets | B |
| A4 | Le format et les voies | A JSON lines et syslog RFC 5424 sur TLS, CEF en option, OTel par le Collector · B OTLP seul · C un connecteur par SIEM | A |
| A5 | L'intégrité | A signature, chaîne et ancre · B A, la tête publiée au journal et `escouade-audit verify` · C TLS seul | B |
| A6 | Les adresses IP | A en entier, réglable · B tronquées d'office · C jamais | A |
| A7 | Ce qu'Escouade garde | A le tampon d'émission seul · B une archive signée de N mois | A |
| A8 | Une commande autorisée | A son programme et son empreinte · B son texte, les secrets masqués par motifs · C rien | A |

**A3, en détail.** Sans noms (A), le SIEM doit tenir une table des ids, exportée par la machine et à refaire à chaque arrivée : le RSSI ne lit plus ses alertes. Avec les titres (C), le titre d'un ticket (« Rachat de la société X ») sortirait du bout en bout vers un outil tiers. B nomme qui et où, jamais quoi. Coût : le même pour les trois, dans AU-5.

**A8, en détail.** Le masquage par motifs (B) laisse passer un secret qu'il ne connaît pas, et une commande en dit long sur le travail. Le programme (`git`, `gh`, `npm`, `curl`) et l'empreinte suffisent à repérer une commande inhabituelle, et l'empreinte se compare à celle qu'approuve `.escouade.yml`. Coût : nul de plus.

## 8. Le découpage en tickets

L'ordre suit les sources : le modèle, puis le serveur seul (utile tout de suite), puis la clé d'audit et les faits des postes, puis la machine qui nomme, puis les guides et les essais.

| Ticket | Contenu | Après | Coût |
| --- | --- | --- | --- |
| AU-1 | La crate `escouade-audit` : le modèle `escouade.audit/v1` et la liste fermée des événements du §1, leurs champs fermés, la liste de ce qui ne sort jamais (§2) et son test sur des données piégées, les écritures JSON lines, syslog RFC 5424 (TLS) et CEF, la signature et la chaîne (JCS, Ed25519), le tampon et sa reprise, `escouade-audit verify` (signatures, chaînes) ; des vecteurs de test | — | 2 h 30 |
| AU-2 | Le serveur émet seul (S1) : sa clé d'audit tirée à `init`, `--audit-file`, `--audit-stdout`, `--audit-syslog`, `--audit-cef`, `--audit-ip` ; les événements de ses routes (§3.1), l'ancre et l'empreinte du chiffré de chaque entrée, les sessions et les prises sans leurs renouvellements ; un test qui lit tout ce qui sort et n'y trouve aucun contenu, sur SQLite puis PostgreSQL | AU-1, TS-1 | 2 h |
| AU-3 | La clé d'audit et le flux des postes : une paire HPKE par époque sur le modèle de `costkey.rs` (le code des coûts rendu commun), enveloppée pour les émetteurs, tournée à leur départ ; le flux `escouade/audit` (le serveur refuse qu'un membre le lise) ; la file hors ligne ; les champs fermés de chaque fait | AU-1, TS-16 | 2 h |
| AU-4 | Les faits des postes (S3) : les refus du cœur (`locks.rs`), les autorisations données (par qui : le membre, l'Overseer, une règle), les commandes approuvées, les plugins installés ou refusés, les clés sensibles, les validations de tickets, les consignes de l'Overseer, le coupe-circuit appliqué ; « Ce que l'audit reçoit de ce poste » dans Réglages › Équipe | AU-3, TS-11, TS-13 | 2 h 30 |
| AU-5 | La machine d'audit (S2) : `escouade-server team audit`, ou un poste d'administrateur (Réglages › Équipe › Audit) ; le rôle de service au registre s'il n'est pas déjà fait (A2, avec TS-4 et l'Overseer d'équipe) ; la lecture du groupe de l'équipe, de l'annuaire, du fichier d'équipe, des passages, de la mémoire, du résumé des coûts et des flux des projets qu'elle tient ; les noms (A3) ; l'ouverture des faits des postes ; le point de contrôle au flux de l'équipe ; l'entrée « l'audit est allumé » que voit chaque membre ; l'export des clés d'émetteur pour `verify` et le recoupement S1–S2 dans l'outil ; un e2e à deux clients, une machine et un collecteur syslog de test | AU-2, AU-4, TS-4 | 3 h |
| AU-6 | Les guides du site (fr, en) : brancher Splunk, Elastic, Sentinel et Datadog (agents, syslog, CEF, OpenTelemetry Collector), la table OCSF, les règles de recoupement en SPL, KQL, ES\|QL et Datadog, vérifier un export, le modèle de fiche de registre et l'information du CSE ; l'essai contre Splunk et Elastic en conteneurs | AU-5 | 2 h |

**En tout** : 6 tickets, environ 14 h d'agent. Avec le parallélisme (AU-2 et AU-3 ensemble, puis AU-4, puis AU-5 et AU-6), quatre vagues, soit une grosse journée en comptant la relecture et les e2e relancés.

**Ce qui ne va pas à ce rythme** :

- **l'essai contre Sentinel et Datadog** : un abonnement Azure et un compte Datadog, à l'utilisateur ; Splunk et Elastic tournent en conteneurs, l'agent les essaie seul ;
- **le numéro d'entreprise IANA** (PEN) des données structurées de syslog : une demande en ligne, gratuite, à faire par l'utilisateur au nom de la société ; d'ici là, le paramètre se nomme avec le numéro de documentation (`32473`, RFC 5612) ;
- **les événements des chantiers pas encore faits** : le coupe-circuit (TS-15) et le store privé (TS-14) ne sont pas construits. AU-1 décrit leurs événements ; chacun de ces tickets les émet en plus (environ 15 min dans chacun). L'audit sort avec ce qui existe.

**Ce que ça ne change pas** : le serveur reste aveugle (S1 n'émet que ce que montre déjà le §3.8, et l'adresse IP qu'il voit déjà), le bout en bout reste le seul mode (C10-A), et rien ne sort d'un poste en clair.
