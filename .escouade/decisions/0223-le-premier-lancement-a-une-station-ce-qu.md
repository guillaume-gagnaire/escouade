---
number: 223
id: "le-premier-lancement-a-une-station-ce-qu"
title: "Le premier lancement a une station « Ce que tu veux brancher » : sources de tickets, voix et hébergeur de code, en cartes à cocher"
date: "2026-11-17"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-512"
pages: ["fenetre/premier-lancement","fenetre/a-decouvrir"]
tickets: ["ESC-512"]
documents: ["docs/ONBOARDING.md"]
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/premier-lancement","added":34,"removed":11},{"page":"fenetre/a-decouvrir","added":5,"removed":3}]
accepted_at: "2026-11-20T07:29:44+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Le 2026-11-17, en choisissant la direction C « La ligne » de l'onboarding, l'utilisateur a demandé que le premier lancement propose les fonctionnalités optionnelles : sources de tickets, fournisseurs de voix (Whisper ou des services externes), et les autres branchements que la conception retient, sans allonger le parcours ni rien installer sans geste (docs/ONBOARDING.md § 10). Il fallait trancher sa place sur la ligne, ce qu'elle contient, comment elle installe et connecte sans second chemin, et ce que deviennent les cartes passées.

## Décision

Une station de plus, la 4e, entre les réglages de départ et le premier projet (un hébergeur branché là sert à la station du projet pour cloner). Trois cartes à cocher, aucune cochée d'avance, chacune avec son « Plus tard » : une source de tickets, la voix, un hébergeur de code. Le second harness et le serveur MCP pour un autre client n'y sont pas et restent à la découverte (le premier est déjà la station du harness ; le second ouvre une porte de la machine, ce qu'un premier lancement ne fait pas). Ce que chaque carte offre est la liste réelle du cœur (registre des sources, tables du tri de l'amorce, amorce, plugins installés, stratégies : commande onboarding_offers), jamais écrite dans la fenêtre. Choisir une chose dans une carte est le geste : seul son plugin s'installe, depuis l'amorce et sans réseau (onboarding_need, qui refuse ce qu'une stratégie refuse) ; un plugin hors de l'amorce vient du store et s'installe depuis Plugins. La connexion s'ouvre dans la station par les composants de Réglages : le bloc d'une source de Services connectés devient le composant SourceAccount, partagé ; les hébergeurs par ForgesSection ; la clé d'un service de voix par OnlineKey ; les réglages de la voix par le même enregistrement que Réglages › Voix. La voix propose Whisper sur la machine (poids dit avant, téléchargement suivi et annulable) ou un service en ligne (ce qui part dit), puis un essai du micro dans un champ de la station, rien envoyé à un agent. « Continuer » marque chaque carte branchée faite et chaque autre passée ; l'état gardé (state.json, onboarding.hookups) dit à la découverte (ON-3) les cartes passées. Hors ligne, ce qui demande le réseau le dit et attend. Une carte verrouillée par une stratégie le dit avec qui l'impose (O12).

## Conséquences

Le parcours passe à 7 stations : harness, compte, réglages de départ, ce que tu veux brancher, premier projet, premier agent (ESC-505), statistiques. L'état du parcours gagne hookups (rétrocompatible : absent, vide). Deux commandes servies de plus (onboarding_offers, onboarding_need, dans list.rs et docs/REMOTE.md). Plugins gagne is_installed et in_seed. La découverte (ESC-506) gagne « Importer des tickets » (sources, voie Planifier, faite par un compte de source connecté, « Essayer » ouvre Réglages › Services connectés) : la carte des sources passée y revient, celle de la voix dans « Dicter un message », celle de l'hébergeur dans « Connecter GitHub » ; la carte compte 18 découvertes sur un poste sans serveur d'équipe (19 en tout). Aucun événement de statistiques nouveau (un plugin installé compte déjà). Les e2e du premier lancement et du premier agent traversent la nouvelle station.
