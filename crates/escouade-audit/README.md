# escouade-audit : l'audit de l'équipe vers un SIEM (AU-1)

Ticket ESC-608 (AU-1) de `docs/TEAM-AUDIT.md` §8, d'après les choix A1-C, A3-B, A4-A, A5-B et A8-A (tranchés le 2026-11-21, demande a-23). La crate porte **le modèle** : ce qu'est un événement d'audit, ce qui n'en sort jamais, comment il s'écrit (JSON lines, syslog, CEF), comment il attend quand la sortie ne répond pas, comment il est signé et chaîné, et ce que vérifie `escouade-audit verify`. **Rien n'est branché** : le serveur émet par elle avec AU-2 (ESC-609), les postes avec AU-3 et AU-4 (ESC-610, ESC-612), la machine d'audit avec AU-5 (ESC-614).

Elle ne dépend pas du cœur de l'app (ni Tauri ni rien de `src-tauri`) : serde, Ed25519 (`ed25519-dalek`), SHA-256, rustls pour syslog sur TLS. Le cœur, lui, en prend une chose : les motifs de jetons (`never::secret_kind`), que la mémoire d'équipe applique aussi (`src-tauri/src/memory.rs`, `secret_in`) ; il n'y a qu'une liste.

## Le modèle `escouade.audit/v1` (§4.1)

```json
{
  "schema": "escouade.audit/v1",
  "id": "01M7YV5YDJ6QZX7SDEG62KXHPX",
  "event": "settings.changed",
  "severity": 4,
  "at": "2026-11-21T10:42:07.120Z",
  "seen": "2026-11-21T10:42:07.410Z",
  "team": { "id": "9f2c", "name": "Acme" },
  "actor": { "member": "m-3c1a", "name": "Claire Martin", "device": "d-77e0", "kind": "desktop" },
  "target": { "keys": ["agents.engines"] },
  "detail": { "version": 12, "changes": [{ "restrict": { "key": "agents.engines", "to": { "words": ["claude", "codex"] } } }], "signers": [{ "member": "m-3c1a", "name": "Claire Martin" }], "reason": "Seuls les fournisseurs sous contrat DPA." },
  "source": "machine",
  "emitter": { "device": "d-0b5e", "seq": 48213, "prev": "sha256:5c0e…" },
  "anchor": { "stream": "team", "n": 1287, "epoch": 41, "ct": "sha256:aa91…" },
  "sig": "ed25519:Jx3…"
}
```

- `id` : un ULID (48 bits de millisecondes, 80 aléatoires) ; un SIEM déduplique par lui.
- `at` : l'heure du fait (l'entrée du journal, le poste) ; `seen` : celle où l'émetteur l'a eu. RFC 3339 en UTC, à la milliseconde.
- `source` : `server` (S1, le serveur aveugle), `machine` (S2, une machine de l'équipe qui déchiffre), `device` (S3, le fait d'un poste ouvert par la machine).
- `actor`, `target` et `detail` sont **fermés par sorte d'événement** : une enum Rust (`event::Fact`), chaque partie une structure `deny_unknown_fields`. Un champ inconnu, un événement hors de la liste ou une sévérité qui n'est pas celle de l'événement sont refusés à la lecture (`Record::from_line`).
- `emitter`, `anchor` et `sig` portent l'intégrité (plus bas).
- Les lignes s'écrivent les membres de chaque objet dans l'ordre alphabétique (celui du JSON canonique) ; un lecteur ne doit pas compter sur un ordre.
- Les valeurs fermées sont des mots anglais stables (`desktop`, `phone`, `machine`, `recovery` ; `admin`, `member`, `service`…), comme `source`.

## Les événements (§1)

La liste est fermée et dans `src/event.rs` (macro `events!`) : un nom stable `famille.action`, une sévérité de 1 (information) à 5 (alerte), et les types de ses trois parties. `escouade-audit events` l'imprime. Le test `the_list_is_the_one_of_team_audit_section_1_with_its_severities` relit le §1 de `docs/TEAM-AUDIT.md` et vérifie que chaque événement y est, avec sa sévérité.

| Famille | Événements |
| --- | --- |
| Membres, appareils, rôles | `team.created`, `member.added`, `member.removed`, `device.added`, `device.enrolled`, `device.revoked`, `role.changed`, `epoch.opened`, `invitation.created`, `invitation.used`, `invitation.reused`, `recovery.used`, `escrow.set`, `escrow.removed`, `team.fork`, `team.rollback` |
| Accès au serveur | `session.opened`, `session.refused`, `rate.limited`, `client.too_old`, `project.access.changed`, `stream.written` |
| Réglages imposés, accords des postes | `settings.changed`, `settings.signers.changed`, `settings.sensitive.answered`, `command.approved`, `command.revoked`, `policy.applied`, `lock.refused` |
| Coupe-circuit (TS-15) | `killswitch.raised`, `killswitch.lifted`, `killswitch.applied` |
| Store privé et plugins (TS-14) | `store.namespace.declared`, `store.plugin.published`, `store.plugin.revoked`, `plugin.installed`, `plugin.removed`, `plugin.updated`, `plugin.refused` |
| Tickets, agents, autorisations | `claim.taken`, `claim.released`, `claim.expired`, `claim.fenced`, `handoff.offered`, `handoff.taken`, `handoff.withdrawn`, `ticket.approved`, `ticket.rejected`, `permission.answered`, `agent.mode.changed`, `directive.added`, `directive.revoked`, `memory.accepted`, `memory.quarantined`, `budget.crossed` |
| L'audit lui-même | `audit.dropped`, `audit.checkpoint` |

Quatre ne sont pas dans les tables du §1 : `device.enrolled` et `stream.written` sont les mots de S1 que le §3.1 décrit (l'inscription et sa sorte de jeton ; l'entrée écrite à un flux, dont l'ancre sert au recoupement du §5.3) ; `audit.dropped` est celui du tampon (§4.4) ; `audit.checkpoint` est le point de contrôle de tête (§5.2, lien 4), dont AU-5 écrit les occurrences. `permission.answered` est de sévérité 2, et 4 quand la réponse est « Toujours autoriser ».

Ce qui n'est pas dans la liste, volontairement : chaque tour d'agent, chaque lecture, chaque opération de synchronisation, les renouvellements de sessions et de baux.

## Ce qui ne sort jamais (§2)

Écrit en dur dans `src/never.rs` (`NEVER`, imprimé par `escouade-audit never`), et tenu ainsi :

- **les mots typés** (`src/words.rs`) : un id opaque, une clé de ticket (`ESC-608`, jamais son titre), une clé de `.escouade.yml`, un mot d'un vocabulaire (harness, modèle, outil, programme), un id de plugin, une version, une empreinte `sha256:`, une clé publique, une adresse IP. Chacun a sa grammaire ; une espace, un séparateur de chemin, un jeton reconnu ou une adresse e-mail le font refuser. L'événement n'est alors pas construit.
- **les deux chaînes libres**, les noms (de membres, d'appareils, de projets, d'équipes, de budgets : A3-B) et les raisons écrites par un administrateur (un coupe-circuit, un réglage, une révocation), passent par `never::scrub` : un mot qui est un jeton, une valeur donnée à une variable (`TOKEN=…`), une suite aléatoire de 32 caractères ou plus, un chemin ou une adresse e-mail est remplacé par `[retiré : ressemble à un secret]`, `[retiré : ressemble à un chemin]` ou `[retiré : ressemble à une adresse]`. Un nom fait 80 caractères au plus, une raison 500. L'émetteur compte les chaînes retirées (`Sealed::redacted`) : « l'émetteur le dit ».
- **une commande** autorisée ou approuvée sort par son programme et l'empreinte SHA-256 de son texte entier (`CommandPrint::of`, A8-A), la même empreinte que celle qu'approuve `.escouade.yml` ; jamais le texte.
- **une branche** (un passage, une validation) sort par son empreinte : son nom peut dire de quoi parle le ticket.
- **une valeur de réglage** sort comme un drapeau, un nombre ou des mots ; une clé sensible par les empreintes de ses valeurs, une clé exécutable par programme et empreinte (`event::Change`).
- **le test des données piégées** (`trapped_data_put_in_every_slot_of_every_event_is_found_in_nothing_that_goes_out`) : un désérialiseur de test (`src/fill.rs`) remplit chaque emplacement de chaque événement, dans chacune de ses variantes, et met tour à tour dans chacun des jetons (GitHub, Anthropic, AWS, Bearer, une clé privée, le secret d'une invitation), un compte de harness, des chemins (Unix, Windows), une commande entière, un titre, un nom de fichier, un texte de conversation et les 24 mots de secours ; rien de tout cela n'est trouvé dans ce qui sort, en JSON lines, en syslog et en CEF. Les titres, les textes et les mots de secours ne sont pas cherchés dans les deux chaînes libres : ce sont les mots d'un administrateur, qu'aucun motif ne distingue d'une raison.

**Ce que la grammaire ne voit pas** : un nom de fichier sans espace ni séparateur (`plan.md`) a la forme d'une clé de réglage ou d'un id. La grammaire refuse ce qui ressemble à un chemin, à un texte ou à un secret ; c'est à l'émetteur (AU-2 à AU-5) de ne donner à une clé que des clés.

## Les voies (§4.2, §4.4)

- **JSON lines** (`sink.rs`) : `Lines` sur la sortie standard (Docker, journald), une ligne écrite et vidée à la fois ; `RotatingFile` dans `<dossier>/<nom>.jsonl`, tourné quand il passerait sa taille (100 Mio par défaut) ou quand le jour change (en UTC, le jour où l'émetteur a vu l'événement, `seen` : le même après un redémarrage) ; le fichier tourné devient `<nom>-<jour>.<n>.jsonl`. Un agent de SIEM suit `<nom>*.jsonl`.
- **syslog RFC 5424** (`syslog.rs`) sur TCP ou TLS (RFC 5425, trames à longueur : `LEN SP MSG`), **jamais UDP** : `tls://hôte:port` ou `tcp://hôte:port`. Le certificat est vérifié contre un fichier PEM (`--audit-syslog-ca` d'AU-2) ou les autorités du système. Le message : `<PRI>1 AT HÔTE escouade - <événement> [escouade@32473 schema=… id=… event=… severity=… team=… source=… device=… seq=…] <le JSON entier>`. La facilité est 13 (journal d'audit) ; la sévérité syslog est `7 - sévérité` (5 → 2 critique, 1 → 6 information), la nôtre reste dans les données structurées. `32473` est le numéro de documentation de la RFC 5612, en une seule constante (`syslog::ENTERPRISE`), d'ici le numéro d'entreprise IANA que demandera l'utilisateur.
- **CEF**, en option de syslog : `CEF:0|Escouade|escouade-team|<version>|<événement>|<événement>|<2 × sévérité>|rt=… externalId=<id> suid=<membre> suser=<nom> src=<IP> cs1=<équipe> cs2=<appareil> cs3=<cible> cs4=<détail> cs5=<chaîne> cs6=<ancre>`, la cible et le détail en JSON canonique. La signature ne passe pas en CEF : on vérifie le JSON lines ou le JSON de syslog.
- **Sous 8 Kio** : `Emitter::seal` refuse un événement dont le message syslog ou CEF le plus long (un hôte de 255 caractères) dépasserait 8 Kio ; il ne prend alors pas de numéro, et l'émetteur le découpe. Une liste d'un événement a 32 éléments au plus.

## Le tampon et la reprise (§4.4)

`Outbox` (`outbox.rs`) : chaque événement scellé est d'abord écrit sur le disque (`spool/<n>.jsonl`, des segments de 16 Mio, `sync_data`), puis l'état de la chaîne (`chain.json`), puis envoyé ; le curseur (`cursor.json`) dit la première ligne pas encore partie et avance ligne à ligne. `Audit` relie l'émetteur, le tampon et la sortie.

- Si la sortie ne répond pas, les événements attendent : **7 jours ou 1 Gio** par défaut (`Limits`). Au-delà, les plus vieux tombent, et un `audit.dropped` le dit : combien, de quand à quand (leurs `at`), et leurs numéros (`first`, `last`). Un `audit.dropped` qui tombe à son tour passe ce qu'il disait au suivant.
- La reprise : un nouveau processus relit `chain.json` (le dernier `seq`, l'empreinte du dernier événement, la position de chaque flux lu, prise de l'ancre) ; s'il s'est arrêté entre l'écriture de la ligne et celle de l'état, la dernière ligne du tampon fait foi. Il reprend là, **sans trou ni doublon de son côté** ; une coupure entre l'envoi et l'avancée du curseur renvoie une ligne, que le SIEM déduplique par `id`.

## L'intégrité (§5.2)

1. **La signature** : Ed25519, par l'appareil qui émet, sur le **JSON canonique** de l'événement sans `sig` (RFC 8785, `jcs.rs` : membres triés par unités UTF-16, nombres écrits comme ECMAScript, chaînes au plus court). `sig` : `ed25519:` puis la signature en base64.
2. **La chaîne** : `emitter.seq` (de 1 en 1) et `emitter.prev`, l'empreinte SHA-256 du JSON canonique entier du précédent (`sig` comprise) ; absent pour le premier.
3. **L'ancre** : `anchor` (le flux, le numéro de l'entrée, son époque, l'empreinte de ses octets chiffrés), donnée par l'émetteur (AU-2, AU-5).
4. **Le point de contrôle** : `audit.checkpoint` (`seq` et `head`, l'empreinte du dernier événement), qu'AU-5 écrit chaque heure et à chaque fermeture dans la sortie et dans le flux de l'équipe.

## `escouade-audit verify` (§5.4)

```sh
escouade-audit verify --keys keys.json --checkpoints checkpoints.jsonl export.jsonl
```

Sans réseau. `--keys` : `{"keys": {"<appareil>": "ed25519:<base64>"}}`, la liste qu'exportera la machine d'audit (AU-5) ; sans elle, seules les chaînes sont vérifiées, et il le dit. `--checkpoints` : les `audit.checkpoint` lus du flux de l'équipe. Il vérifie que chaque ligne est du modèle, chaque signature, chaque chaîne (un trou, une fourche, un `prev` qui ne suit pas ; un trou qu'un `audit.dropped` dit n'en est pas un, un export dans le désordre ou une ligne en double non plus), chaque point de contrôle (une tête qui n'est pas l'événement de son numéro ; une fin retirée), et la première règle du recoupement de S1 et S2 (§5.3 : deux événements d'une même entrée dont l'empreinte du chiffré diffère). Les deux autres règles du recoupement viennent avec AU-5 (`verify::crosscheck`). Il rend `48213 events, 2 emitters, whole chains, 0 problems`, ou chaque écart avec son fichier et sa ligne ; il répond 0, 1 s'il a trouvé un écart, 2 s'il n'a pas pu lire ce qu'on lui a donné.

## Les tests et les vecteurs

`cargo test -p escouade-audit` (depuis `src-tauri`) : chaque événement relu tel qu'écrit dans chacune de ses variantes, les champs inconnus refusés à chaque niveau, la liste du §1, les données piégées, la signature et la chaîne, `verify` sur une ligne changée, retirée, une fourche, un export dans le désordre, une fin coupée, le recoupement, le tampon, ses limites et `audit.dropped`, la reprise après un arrêt, la rotation du fichier, syslog sur TLS contre un collecteur local (et un certificat refusé), sur TCP, le tampon vidé quand le collecteur revient, CEF ; `tests/verify_cli.rs` lance le binaire.

- `tests/vectors/jcs/` : les exemples de la RFC 8785 (§3.2.2.3, §3.2.3) et ceux de son dépôt de test (tableaux, structures, tri français, caractères étranges), en entrée et en sortie ; l'annexe B (les nombres par leurs bits IEEE 754) est dans `jcs.rs`.
- `tests/vectors/audit/` : un export signé de deux émetteurs (`export.jsonl`), ses clés (`keys.json`, tirées des graines de test 1 et 3, jamais une vraie clé), son point de contrôle, et trois exports abîmés (`changed`, `removed`, `cut`). `ESCOUADE_AUDIT_VECTORS=write cargo test -p escouade-audit the_vectors_of_the_chain` les réécrit ; les ids changent alors (ils sont tirés au hasard).
