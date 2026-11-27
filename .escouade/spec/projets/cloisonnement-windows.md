---
title: "Cloisonnement sous Windows natif (choisi, à construire)"
subject: "projets"
sources: ["docs/WINDOWS-ISOLATION.md","docs/windows-isolation/maquette.ps1 (lancée le 2026-11-26)","docs/security/audit-2026-11/rapport.md (SEC-02)","src-tauri/src/fence.rs","Choix de l'utilisateur, demande a-4 (2026-11-21)","Choix de l'utilisateur, demande a-44 (2026-11-27), option a","OpenAI, Building the Codex Windows sandbox","Microsoft Learn : Windows Sandbox, wsb, AppContainer, CreateRestrictedToken"]
tickets: ["ESC-591"]
decisions: [349,350,351,352,353,354]
compared: null
---

> **Choisi le 2026-11-27, pas encore construit.** L'utilisateur a pris les recommandations de l'étude `docs/WINDOWS-ISOLATION.md` (ESC-591), demande a-44, option a : la voie B, et W2 à W8 comme recommandés. Rien de cette page n'est construit : les tickets WB-1 à WB-8 la feront, et WB-1 est une porte (§ « La porte »). Le comportement d'aujourd'hui reste celui de *Projets privés*.

## Ce que c'est

Sous Windows natif, la clôture d'un projet privé est une liste de règles de refus donnée au harness (« Garde-fou » pour Claude Code, OpenCode, Copilot CLI ; « Barrière » pour Codex, par son propre bac à sable) : une commande qui construit son chemin autrement lit le dossier (audit de la 2.2.0, SEC-02, point 1). Le cloisonnement choisi fait de la clôture une barrière du système pour tout l'arbre des agents des harness d'Escouade.

## Les voies comparées

| Voie | Ce qu'elle ferme | Ce qu'elle casse | Droits | Éditions | Coût |
| --- | --- | --- | --- | --- | --- |
| A, AppContainer | fichiers (liste d'accès), registre, réseau, processus | la boucle locale (serveur MCP, hooks, proxy local), chaque outil du profil à accorder | aucun, sauf l'exemption de la boucle locale (administrateur) | toutes | ≈ 14 tickets, ≈ 35 h d'agent |
| **B, jeton restreint à faible intégrité** (choisie) | les fichiers des autres projets (liste de refus), l'écriture hors des dossiers étiquetés « bas », les processus et les fenêtres de l'utilisateur | les écritures hors du dossier de l'agent (caches, mise à jour du harness) | aucun | toutes | ≈ 8 tickets, ≈ 18 h d'agent |
| C, un compte par ensemble | fichiers, registre, processus ; pas le presse-papiers | chaque harness à connecter sous chaque compte, `safe.directory` de git, la lecture des sessions | administrateur à chaque compte | toutes | ≈ 11 tickets, ≈ 28 h d'agent |
| D, Windows Sandbox | tout ce qui n'est pas partagé | tout : une VM sans nos outils, sans la boucle locale de l'hôte, une instance | administrateur, redémarrage | Pro, Entreprise, Éducation | ≥ 40 h, écartée |
| E, WSL (existe) | les commandes du bac à sable de Claude Code, s'il est activé | un dépôt sous `/mnt/c` lent ; MCP en réseau miroir | administrateur pour installer WSL | toutes | 1 ticket pour l'activer d'office (W6) |

La maquette de l'étude (`docs/windows-isolation/maquette.ps1`, jetable, défaite à chaque lancement) a montré qu'un jeton restreint qui garde le SID de l'utilisateur ouvre un processus ordinaire de l'utilisateur en écriture : il s'en échapperait. En faible intégrité, il ne l'ouvre plus, refuse le dossier privé en lecture et en écriture, et garde git, node, Claude Code et la boucle locale.

## La voie choisie, B

- **Un lanceur** (`escouade-fence-run.exe`, signé) se met devant la commande d'un agent clôturé, comme `wsl.exe` devant un Claude Code de WSL : il lance le harness et **tout son arbre** (W2) sous un jeton restreint, en faible intégrité, avec ses poignées et sa console héritées. Le Job d'Escouade tient l'arbre comme aujourd'hui.
- **Un SID par ensemble** (`S-1-9-…`, dérivé de l'id du projet, porté par aucun compte) ; une ACE de refus héritée pour ce SID sur chaque dossier que l'agent ne doit pas toucher : les autres projets, leurs worktrees et dossiers d'agents, leurs fichiers joints, tickets et sessions rangées par projet, `state.json`, les conversations et les journaux d'Escouade.
- **Ce que le harness tient lui-même** pour tous les projets (`.claude.json`, `history.jsonl`, sessions de Codex) reste un garde-fou, par les règles d'aujourd'hui.
- **L'agent écrit** (W4) dans son dossier, ses worktrees, un dossier temporaire à lui (`TEMP`, `TMP`) et le dossier du compte de son harness, étiquetés « bas », et dans des caches à lui par variables (`npm_config_cache`, `DISABLE_AUTOUPDATER` pour Claude Code) ; le reste lui est refusé, et sa conversation le dit.
- **Quand la barrière ne peut pas tenir** (W3 : volume sans ACL, ACE refusée), l'agent n'est pas lancé ; le refus dit le dossier, la raison et le geste possible (déplacer le projet sur un volume NTFS, retirer « Projet privé »).
- **Les terminaux que l'utilisateur ouvre lui-même** restent hors de la barrière (W7).
- **Les harness des plugins tiers** restent refusés tant qu'un projet privé existe (W5), réexaminés après les essais de la barrière.
- **WSL** (W6) : pour un agent clôturé dont le Claude Code tourne dans WSL, Escouade active son bac à sable et vérifie que l'interop ne le contourne pas.
- **Réglages › Confidentialité**, « Ses fichiers » : « Barrière » pour les harness d'Escouade sous Windows, et une ligne, dite une fois à l'allumage de « Projet privé » sans accord à part (W8) : « Sous Windows, Escouade pose sur les dossiers des projets une règle de refus pour les agents des autres projets, et une étiquette d'intégrité basse là où ses agents écrivent. »
- **Retrait** : quand le projet n'est plus privé, quand il est retiré, et à la désinstallation.

## La porte

WB-1 fait tourner de vraies sessions de Claude Code, Codex, OpenCode et Copilot CLI sous ce jeton, sur le poste de l'utilisateur (≈ 45 min d'essais de sa part). Si elles n'y tiennent pas, rien de plus n'est construit sous Windows natif : restent W6 (WSL) et le conseil d'un compte par ensemble dans la doc du site.

## Limites de la voie B

- Ce n'est pas une liste d'accès : un fichier de l'utilisateur hors des projets reste lisible par un agent, comme aujourd'hui.
- Le réseau n'est pas fermé.
- Un volume sans ACL (FAT32, exFAT, un partage réseau) ne porte pas la barrière (d'où W3).
- Un fichier déplacé dans un dossier depuis le même volume garde son ACL jusqu'à ce qu'Escouade repose l'héritage.
- Le presse-papiers, `ssh-agent` et le gestionnaire d'identifiants de git sont à essayer (WB-1).
