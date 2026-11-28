---
number: 362
id: "app-mobile-m1-la-direction-de-l-app-prop"
title: "App mobile, M1 : la direction de l'app est B, « Le fil » (choisie par l'utilisateur)"
date: "2026-11-20"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-549"
pages: ["a-distance/app-mobile"]
tickets: ["ESC-549"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/app-mobile","added":23,"removed":0}]
accepted_at: "2026-11-20T13:56:20+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'app mobile (Capacitor, d'après ESC-445) doit d'abord faire répondre vite (précisé par l'utilisateur au début d'ESC-549). Trois directions ont été rendues avec Impeccable (tirage cade135e), aux deux tailles de téléphone, en clair et en sombre, sur sept écrans : design/mobile-2-3/ (index.html les compare). Elles diffèrent par la navigation, la densité, la place de la boîte « À toi » face aux agents et le geste de réponse.

## Décision

B « Le fil », choisie par l'utilisateur le 2026-11-20 dans la conversation du chef d'escouade, après avoir vu les planches des trois directions : un fil dense de tout ce qui attend, rangé comme la boîte du bureau ; une ligne dépliée sur place ; « Autoriser » en bouton, actif seulement quand la commande a été vue jusqu'au bout, puis le verrou du système ; trois onglets (À toi, Agents, Kanban), des puces d'espace, le « + » de l'en-tête pour créer (M16). B garde de A qu'une demande réglée ouvre la suivante, et de C la plaque de l'espace sur chaque ligne et notification. Les autres directions étaient A « La pile » (ma recommandation d'abord) et C « Les espaces ».

## Conséquences

MO-1 fait la coque de B (onglets, fil) ; MO-5 le dépliage et les boutons, sans glissé (– 1 h par rapport à A) ; MO-7 le glissé de côté d'une ligne du Kanban. Le verrou du système (M3) fait toute la garde d'une autorisation, et une commande de plus de douze lignes s'ouvre en plein écran à 360 px.
