---
number: 289
id: "securite-2-3-un-code-d-appairage-faux-ne"
title: "Sécurité 2.3 : un code d'appairage faux ne l'annule plus, les échecs se comptent par source, et la fin d'une connexion lâche ses appels"
date: "2026-11-23"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-654"
pages: ["a-distance/api-distante","a-distance/mode-serveur","a-distance/se-connecter-a-un-serveur","a-distance/atteindre-un-reseau-prive"]
tickets: ["ESC-654"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/api-distante","added":20,"removed":9},{"page":"a-distance/mode-serveur","added":5,"removed":5},{"page":"a-distance/se-connecter-a-un-serveur","added":2,"removed":2},{"page":"a-distance/atteindre-un-reseau-prive","added":1,"removed":1}]
accepted_at: "2026-11-23T18:57:10+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'audit Mantis du serveur qu'OshOEz a écrit sur un fork de notre 2.2.7 (données d'un tiers) décrit six défauts d'une API distante ; la revue privée SR-1 (SR1-04) avait vu le premier chez nous. Vérifiés dans notre remoteapi (ESC-654) : cinq codes faux de n'importe qui finissaient le code d'appairage en attente ; 20 poignées de main ratées par minute, de n'importe où, fermaient la porte à tous, le bon jeton du propriétaire compris ; 8 connexions muettes tenaient toutes les places d'une poignée de main ; un appel qui ne rendait pas la main continuait après la fin de sa connexion ou la révocation de son appareil, et ceux qui attendaient derrière lui sur sa file partaient ensuite. Un appareil ne pouvait pas prendre les 16 places (4 par appareil, aucun appareil né d'un appareil depuis ESC-630). Le mot de passe du proxy hors trousseau n'existe que sur la ligne 2.2 (ESC-595, corrigé par ESC-653) ; le Referer ne fuit pas (liens ouverts dans le navigateur du système, aucune page servie par l'API).

## Décision

1. Un code d'appairage faux est refusé sans annuler le code qui attend ; la force brute est tenue par les bornes du serveur : 20 poignées de main ratées par source et par minute (au-delà, les codes de cette source ne sont pas lus : raison « wait »), 120 codes faux par minute de toutes les sources (au-delà, aucun code n'est lu). Au plus 1 200 codes lus pendant les 10 minutes d'un code, sur 2^40 : une chance sur 900 millions environ.
2. La source est l'adresse de l'autre bout (127.0.0.1 pour la machine, un tunnel SSH ou un proxy local ; l'adresse d'une machine du tailnet ou du réseau de --listen), ou le pair du tunnel du téléphone. Un bon jeton passe toujours : les échecs ne ferment plus la porte.
3. Les poignées de main en cours (8) se font en 10 s en tout ; toutes prises, la plus ancienne est fermée pour la nouvelle.
4. 4 places, au-delà des 16, sont gardées à la première connexion d'un appareil : un appareil qui n'est pas connecté entre toujours, pour révoquer celui qui a pris les autres.
5. À la fin d'une connexion (fermée, appareil révoqué), ses appels asynchrones sont abandonnés et ses appels qui attendaient sur une file ne partent plus ; un appel bloquant déjà parti va au bout, sans réponse.
6. La page de la fenêtre dit referrer no-referrer, les pages des extensions de l'éditeur Referrer-Policy: no-referrer ; un essai de l'API distante garantit que le mot de passe du proxy n'arrive dans l'état d'aucun client distant, une fois la 2.2 reversée.

## Conséquences

Personne ne bloque plus l'appairage ni l'accès d'un appareil appairé en envoyant des codes ou des jetons faux ; derrière un proxy TLS ou sur la boucle locale, toutes les connexions comptent comme une seule source, et un attaquant local peut encore faire attendre un appairage d'une minute (le code reste valable). Le client dit « Trop d'essais faux en une minute : le code vaut toujours, réessaie dans une minute » (raison « wait ») ; il lit encore « tries » d'un serveur d'avant. escouade-server pair ne parle plus de « 5 essais ». Coût : un compteur par source (4 096 sources au plus), un signal de fin par connexion.
