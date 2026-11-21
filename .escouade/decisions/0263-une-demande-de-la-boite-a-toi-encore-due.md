---
number: 263
id: "une-demande-de-la-boite-a-toi-encore-due"
title: "Une demande de la boîte « À toi » encore due passe au chef d'escouade quand son auteur s'en va, au lieu d'être retirée"
date: "2026-11-21"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-585"
pages: ["harness/boite-a-toi"]
tickets: ["ESC-585"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/boite-a-toi","added":19,"removed":10}]
accepted_at: "2026-11-21T20:41:00+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Le 2026-11-21, la demande a-10 (les deux essais avant l'app mobile, déposée par l'agent de l'ESC-549) a été retirée d'office (« Son agent a été archivé. ») à la validation de son ticket : l'utilisateur ne la voyait plus, alors que c'était toujours à lui de faire, et la réponse n'avait plus personne à qui revenir. Valider 532, 551, 567, 580 ou 581 aurait retiré de même a-12, a-14 à a-18. La spec voulait qu'un agent supprimé ou archivé, ou un ticket terminé, retire ses demandes.

## Décision

Une demande encore due (ouverte, reportée, ou répondue sans que son auteur ait lu la réponse) dont l'auteur s'en va (le ticket de son agent validé, l'agent archivé ou supprimé, à la validation ou à la main) n'est jamais retirée : elle passe au chef d'escouade vivant du projet, qui en devient l'auteur ; sa date, ses fichiers et les tickets qui l'attendent restent ; l'histoire dit « Reprise par <chef> : le ticket ESC-N est validé » ; la réponse revient au chef en message, qui dit d'où elle vient, et une réponse déjà partie vers l'ancien auteur et pas lue repart vers le chef. Sans chef, elle reste sans auteur vivant (ni l'ancien agent ni Claude hors Escouade ne la changent), la réponse reste dans la demande et sa trace, et elle passe au premier chef que le projet nomme. La suppression à la main suit la même règle, sans question à la suppression. Restent retirées : un ticket supprimé, un ticket renvoyé « À faire » en cours, un projet retiré, une date passée. La fenêtre le dit discrètement : « reprise de <agent> » ou « sans auteur » sur la ligne, une phrase sous le titre du panneau, « Changée de main » dans l'histoire. Ce qui a été retiré avant n'est pas rouvert.

## Conséquences

L'utilisateur ne perd plus ce qui l'attend en validant un ticket, et un ticket qui attend une demande l'attend toujours. Le chef reçoit des demandes qu'il n'a pas déposées (son message le dit) et peut les retirer quand elles n'ont plus lieu d'être. Le modèle gagne `Ask::handed` (premier auteur, cause, clé du ticket, heure), lu par la fenêtre (et le téléphone). Une demande sans auteur vivant ne compte plus pour Claude hors Escouade.
