---
number: 375
id: "audit-d-equipe-au-3-la-cle-d-audit-des-p"
title: "Audit d'équipe (AU-3) : la clé d'audit des postes pour les machines, sinon les administrateurs ; deux flux, escouade/audit/k et escouade/audit/f ; des faits sans auteur"
date: "2026-11-28"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-610"
pages: ["serveur-equipe/audit-siem"]
tickets: ["ESC-610","ESC-612","ESC-614"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/audit-siem","added":13,"removed":4}]
accepted_at: "2026-11-28T12:49:39+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

AU-3 (ESC-610) construit la clé d'audit des postes et le flux où ils déposent leurs faits scellés (docs/TEAM-AUDIT.md §3.3, S3 ; décisions A1-C et A2-A). L'étude dit « enveloppée pour les seuls appareils qui émettent l'audit (le rôle de service, ou l'administrateur qui le tient) » et nomme un flux `escouade/audit`, sans dire qui émet tant qu'aucun administrateur n'est désigné, ni comment le serveur, qui ne connaît que les rôles, sait qui lit, ni qui signe un fait.

## Décision

1. Les émetteurs sont les appareils des machines de l'équipe (rôle de service) quand l'équipe en a une, sinon les appareils de tous ses administrateurs (AU-5 dira lequel tient l'audit). La paire d'une époque n'est enveloppée que pour eux ; elle est périmée dès qu'un appareil pour qui elle l'est n'émet plus (départ, ou arrivée d'une machine : les administrateurs passent la main), et chaque poste refuse une entrée de la clé dont le signataire n'émettait pas à son époque. Une nouvelle paire prend une époque au-dessus de la plus haute nommée dans le flux. 2. Le flux `escouade/audit` de l'étude devient deux flux propres de l'équipe de 16 octets, `escouade/audit/k` (la clé : écrite par un administrateur ou une machine) et `escouade/audit/f` (les faits : chaque appareil y dépose, seuls un administrateur ou une machine le lisent, par le flux, le direct ou un fichier ; aucun fichier ne s'y rattache). Le serveur ne connaît que les rôles : il refuse les membres ; les postes jugent plus finement, et seuls les émetteurs ouvrent. 3. Un fait de poste ne porte pas d'auteur : l'émetteur le prend de la signature de l'entrée (le membre et l'appareil). Il porte son id (pour le prendre une fois), l'heure du poste (`at`), l'événement (une des 18 sortes que le §1 donne à S3) et sa cible et son détail fermés ; l'émetteur ajoute `seen`, l'heure où le serveur l'a reçu. 4. Les entrées des deux flux sortent dans l'audit du serveur (`stream.written`, flux `escouade.audit.k` et `escouade.audit.f`) : la machine d'audit répète leur ancre. 5. Le code de la clé des coûts devient commun (`EpochKey`, `keyring.rs`) sans changer ses étiquettes, ses octets ni son état.

## Conséquences

Un administrateur ne lit pas ce que les postes déposent quand une machine émet (il voit le flux chiffré, comme le serveur, et ne l'ouvre pas). Tant qu'il n'y a pas de machine, tout administrateur ouvert peut ouvrir les faits : AU-5 pourra resserrer à celui qui tient l'audit, par une entrée de plus du flux de la clé, sans changer le format. Les faits ne disent pas qui les signe dans leur contenu : un poste ne parle pas pour un autre. Le serveur voit un dépôt par lot (une taille, un appareil, une heure), et l'audit du serveur en témoigne. Rien ne s'affiche avant AU-4.
