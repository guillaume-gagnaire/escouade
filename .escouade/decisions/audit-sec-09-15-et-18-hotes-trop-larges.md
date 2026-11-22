---
number: null
id: "audit-sec-09-15-et-18-hotes-trop-larges"
title: "Audit SEC-09, 15 et 18 : hôtes trop larges refusés, liste de révocation attestée chaque semaine, mode hors ligne imposable"
date: "2026-11-22"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-596"
pages: ["fenetre/reseau","store/store-dans-l-app","plugins/programmes-js-ts-en-wasm"]
tickets: ["ESC-596"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/reseau","added":8,"removed":0},{"page":"store/store-dans-l-app","added":9,"removed":0},{"page":"plugins/programmes-js-ts-en-wasm","added":6,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Le rapport d'audit public (docs/security/audit-2026-11/rapport.md) relevait, en gravité faible ou à titre d'information : SEC-09, trois angles morts du réseau des plugins (un domaine de premier niveau déclarable comme hôte, derrière un proxy seul le nom jugé, 6to4 et Teredo non lus) ; SEC-15, une liste de révocation sans date limite, qu'on pouvait tenir en vigueur en empêchant la neuve d'arriver ; SEC-18, des noms réservés de Windows créés par fs/*, des images distantes téléchargées depuis le réseau local, et quatre flux périodiques sans réglage pour les couper. L'utilisateur a dit le 2026-11-21 (a-7) « Oui, applique-les ». Republier la liste chaque semaine « même inchangée » demandait une signature neuve, alors que la clé des listes est sur le poste de l'utilisateur, à mot de passe : choix du 2026-11-22, une clé de fraîcheur en CI qui ne signe jamais une liste.

## Décision

1) Un hôte de permissions.network sans point, ou suffixe public de la section ICANN (liste embarquée : crate psl, schemas/public-suffixes.mjs), est refusé à la lecture du manifeste par l'app et le store ; un suffixe privé (une seule entreprise) reste permis. Derrière un proxy, le nom est aussi résolu sur la machine et refusé s'il donne une adresse locale ; 6to4 et Teredo sont jugés par l'IPv4 qu'ils portent. 2) La liste reste datée et signée avec sa date ; chaque lundi le workflow revocations.yml signe avec une clé de fraîcheur (secret GitHub, rôle à part, jamais une liste) « liste nº N, empreinte X, vaut encore au <date> » ; l'app refuse une attestation d'une autre liste et avertit discrètement quand ce qu'elle tient a plus de 7 jours. Sans clé embarquée, rien ne change et l'âge est celui de la dernière lecture. 3) Les noms réservés de Windows sont écartés de fs/*, files/read et du lecteur d'archives ; une image distante locale ou privée n'est jamais téléchargée ; un « Mode hors ligne » coupe les mises à jour, le store, la liste de révocation et les statistiques, refusé par le cœur, dit dans la barre de statut, imposable par la stratégie network.offline d'un poste géré (mêmes emplacements que la 2.3).

## Conséquences

Un plugin tiers qui déclarait un TLD ou un nom sans point est refusé (aucun de la première main). La clé de fraîcheur reste à créer par l'utilisateur (store/scripts/create-freshness-key.ts) et à poser en secret ; d'ici là le workflow le dit dans son résumé, et l'app avertit seulement d'un store injoignable 7 jours. Le mode hors ligne ne coupe pas ce que l'utilisateur fait d'un geste ni le travail des agents. La lecture de la stratégie est une petite copie de celle d'escouade-policy (2.3), à remplacer par elle au reversement.
