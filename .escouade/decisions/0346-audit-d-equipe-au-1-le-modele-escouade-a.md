---
number: 346
id: "audit-d-equipe-au-1-le-modele-escouade-a"
title: "Audit d'équipe (AU-1) : le modèle escouade.audit/v1 tel que construit dans escouade-audit"
date: "2026-11-26"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-608"
pages: ["serveur-equipe/audit-siem"]
tickets: ["ESC-608","ESC-609","ESC-610","ESC-612","ESC-614"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/audit-siem","added":12,"removed":3}]
accepted_at: "2026-11-26T20:56:15+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

AU-1 (ESC-608) construit la crate escouade-audit d'après docs/TEAM-AUDIT.md (§1, §2, §4.1, §4.2, §4.4, §5) et les choix A1-C, A3-B, A4-A, A5-B, A8-A. L'étude laissait ouverts des points de forme que l'écriture du modèle oblige à fixer : les événements qui manquent aux tables du §1, comment tenir « ce qui ne sort jamais » dans les chaînes, la correspondance des sévérités, le jour d'un fichier tourné, ce qui arrive à un événement trop gros pour syslog, et où vivent les motifs de jetons.

## Décision

1. La liste fermée compte les 53 événements du §1, plus device.enrolled et stream.written (les mots du serveur seul du §3.1 ; l'ancre de stream.written sert au recoupement), audit.dropped (qui porte aussi les numéros first et last de la chaîne, pour que verify accepte le trou qu'il dit) et audit.checkpoint (seq, head). 2. Les chaînes d'un événement sont des mots d'une grammaire fixe (id, clé de ticket, clé de réglage, mot d'un vocabulaire, id de plugin, version, empreinte, clé publique, IP) qui refusent une espace, un séparateur, un jeton ou une adresse e-mail ; seuls les noms (80 caractères) et les raisons (500) sont libres et nettoyés (jeton, TOKEN=valeur, suite aléatoire de 32 caractères ou plus, chemin, adresse e-mail → « [retiré : ressemble à …] »), l'émetteur comptant ce qu'il retire. 3. Une branche (passage, validation) sort par son empreinte, comme une commande (programme + SHA-256 du texte) ; une valeur de réglage sensible par son empreinte, une exécutable par programme et empreinte. 4. Les motifs de jetons sont une seule liste, dans escouade-audit (never::secret_kind) ; la mémoire d'équipe de l'app (memory::secret_in) l'applique. 5. Sévérité syslog = 7 − sévérité (5 → critique, 1 → information), facilité 13 (log audit), la nôtre dans les données structurées ; sévérité CEF = 2 × la nôtre ; la signature ne passe pas en CEF. 6. Un fichier JSON lines se tourne par taille (100 Mio par défaut) et par jour UTC de seen, renommé <nom>-<jour>.<n>.jsonl. 7. Un événement dont le message syslog ou CEF le plus long dépasserait 8 Kio n'est pas scellé (il ne prend pas de numéro) : l'émetteur le découpe ; une liste a 32 éléments au plus. 8. Les valeurs fermées et les sorties de verify sont en anglais stable (desktop, phone, machine, recovery ; admin, member, service), comme source et les messages du serveur d'équipe.

## Conséquences

AU-2 à AU-5 émettent par ce modèle sans le rouvrir : ils donnent des mots typés, découpent un réglage trop gros en plusieurs settings.changed, et passent au poste les noms tels qu'ils sont (la crate les nettoie). Un nom de fichier sans espace ni séparateur a la forme d'un mot (une clé, un id) : la grammaire ne le voit pas, c'est à l'émetteur de ne donner à une clé que des clés (README de la crate). Une branche n'est plus lisible en clair dans le SIEM, seulement comparable. Changer le numéro d'entreprise IANA est une seule constante (syslog::ENTERPRISE). L'app dépend désormais d'escouade-audit (pour les motifs, puis pour AU-4).
