---
number: 244
id: "windows-l-installeur-d-une-mise-a-jour-a"
title: "Windows : l'installeur d'une mise à jour attend la fin de l'app, met de côté les programmes en cours d'usage et relance l'app après un échec"
date: "2026-11-18"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-533"
pages: ["introduction/livraison"]
tickets: ["ESC-533"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"introduction/livraison","added":4,"removed":0}]
accepted_at: "2026-11-21T11:12:27+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Sur le poste de l'utilisateur, la 2.2.2 et la 2.2.3 ne s'installaient pas (sept tentatives), sans rien montrer : l'app se fermait et rien ne revenait. L'app lance l'installeur NSIS silencieux puis sort ; Windows met du temps à défaire un gros process et ne peut alors plus l'arrêter. Le contrôle « app ouverte » du gabarit de Tauri (Restart Manager) voyait encore ce process, `RmShutdown` rendait ERROR_FAIL_SHUTDOWN (351), et l'installeur abandonnait sans rien écrire (événement 10006 « Unknown App », Status 17). Reproduit avec une app d'essai sous un autre nom (EscouadeEssai533). Constaté au passage : un `escouade-hook.exe` ou `escouade-plugin.exe` en cours d'usage était sauté en silence, l'installeur se disant réussi avec un dossier à moitié à jour.

## Décision

Dans `NSIS_HOOK_PREINSTALL` (`src-tauri/windows/installer-hooks.nsh`) : en mode mise à jour seulement, attendre, 30 s au plus et par pas de 250 ms, qu'aucun process ne tienne `escouade.exe` (une session du Restart Manager neuve à chaque pas), puis laisser le contrôle de Tauri faire comme avant ; une installation à la main n'attend pas. Dans tous les modes, un `escouade-hook.exe` ou `escouade-plugin.exe` qu'on ne peut pas ouvrir en écriture est renommé `<nom>.<n>.old` (son process continue dessus) pour que le nouveau soit écrit ; ce qui a été mis de côté est effacé à l'installation suivante et à la désinstallation, et remis en place si l'installation échoue. Une mise à jour lancée avec `/R` qui échoue relance l'app sur sa version (`.onInstFailed`), qui dit au démarrage que la mise à jour n'a pas pu s'installer, avec « Réessayer ».

## Conséquences

La correction vaut dès la mise à jour vers la version qui la porte, puisque c'est l'installeur de la nouvelle version qui tourne. Une app vraiment restée ouverte pendant une mise à jour n'est fermée qu'au bout de 30 s. Une attente ajoute au journal « Application » deux événements du Restart Manager par pas. Aucun texte nouveau n'est montré. macOS et Linux ne sont pas touchés : l'app y remplace elle-même son paquet avant de redémarrer, sans contrôle d'un autre process.
