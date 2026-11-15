---
number: 172
id: "le-dossier-de-la-vue-vs-code-et-son-obse"
title: "Le dossier de la vue VS Code et son observateur, par leur nom sur le disque"
date: "2026-11-15"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-463"
pages: ["editeur/editeur"]
tickets: ["ESC-463"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"editeur/editeur","added":1,"removed":0}]
accepted_at: "2026-11-15T08:27:35+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La vue VS Code reconnaît un fichier changé en cherchant son chemin sous la racine de la source, par son début. La racine venait du toplevel de git (nom long), l'observateur surveillait le dossier du projet tel qu'enregistré (nom court 8.3 sur le runner Windows de la CI, lien, jonction, /var de macOS) : aucun chemin ne correspondait et le fichier n'était pas relu (CI de release/2.3 rouge, vscode-view.spec.ts).

## Décision

Le cœur nomme aux deux bouts le dossier comme le disque le nomme (paths::on_disk : canonique sans \\?\, liens et noms courts résolus) : chaque dossier surveillé par GitService::watch, et la racine que rend fs_tree. Un chemin qui n'existe pas (encore) ou sur un partage (WSL compris) garde son écriture. La vue ne change pas : elle compare toujours par préfixe.

## Conséquences

Les chemins de FsChanged et la racine de fs_tree sont désormais dans l'écriture réelle (sous Windows en \, plus en / comme git les donne). Rien d'autre ne lisait ces écritures : les commandes de l'éditeur prennent des chemins relatifs. Un dossier de projet sur un partage n'est pas résolu : un lien vers un partage garderait le défaut.
