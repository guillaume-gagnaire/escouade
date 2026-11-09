---
number: 113
id: "l-amorce-n-installe-que-les-plugins-de-c"
title: "L'amorce n'installe que les plugins de ce que l'utilisateur a configuré"
date: "2026-11-07"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-373"
pages: ["plugins/livres-avec-escouade"]
tickets: ["ESC-373"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/livres-avec-escouade","added":18,"removed":7}]
accepted_at: "2026-11-07T19:48:21+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Avec ESC-338, `Plugins::sync_seed` installait au lancement tous les plugins de la première main que l'amorce tient, sauf ceux que l'utilisateur avait désinstallés : une fois Z2 passé, chaque utilisateur aurait eu 23 plugins installés, harness, sources, hébergeurs et voix qu'il n'utilise pas. L'utilisateur a demandé le 2026-11-07 : « je veux un tri ».

## Décision

Au lancement, l'amorce n'installe que ce que la configuration utilise, lu dans les réglages, les projets, les agents et les comptes, jamais sur le réseau (`plugins/sort.rs`) : un harness par un compte, un agent, le harness par défaut d'un projet ou celui des textes ; une source ou un hébergeur par un projet (sa source, l'hôte de ses remotes) ou un compte déjà connecté ; un service de voix choisi « En ligne » ou dont la clé est gardée ; isola par un `.isola.toml` ; Discord, Slack, Contraste élevé et Règles de l'équipe seulement s'ils sont déjà installés. Ce qui ne se lit pas sûrement (fichier illisible, projet hors d'atteinte, identifiant sans règle) installe. Une entrée `builtin` de la 2.1 non désignée est retirée, ses choix gardés, sans être notée désinstallée. Une première installation n'installe rien. Le geste qui demande un plugin absent (compte, source, harness ou hébergeur d'un projet, connexion d'un compte, voix, isola) l'installe depuis l'amorce sans réseau ni fenêtre et le dit (« Plugin Codex installé ») ; c'est le seul chemin qui remet un plugin désinstallé. La vue Plugins propose les autres (« Dans l'app, sans téléchargement »), installables d'un clic.

## Conséquences

Une mise à jour depuis la 2.1 garde tout ce qui marchait, sans un clic, et seulement cela. Le geste vaut accord : un plugin de première main signé s'installe sans la fenêtre de permissions (comme « Installer » dans la vue, quand l'amorce a la version la plus récente). Un remote sur un hôte inconnu sans compte ne désigne aucun hébergeur (en 2.1, il ne marchait pas sans compte). Un disque débranché au lancement installe tous les hébergeurs et isola. Le contenu de `seed.json` reste à Z2 (ESC-354).
