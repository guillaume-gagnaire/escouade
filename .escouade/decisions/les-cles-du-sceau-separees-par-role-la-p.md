---
number: null
id: "les-cles-du-sceau-separees-par-role-la-p"
title: "Les clés du sceau séparées par rôle : la première main bornée à dev.escouade.* par le code, une clé à mot de passe pour les tiers et la révocation"
date: "2026-11-22"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-592"
pages: ["store/revue-et-signature","plugins/livres-avec-escouade","store/store-dans-l-app","store/publier-et-securite"]
tickets: ["ESC-592"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"store/revue-et-signature","added":16,"removed":5},{"page":"plugins/livres-avec-escouade","added":5,"removed":0},{"page":"store/store-dans-l-app","added":6,"removed":0},{"page":"store/publier-et-securite","added":6,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

L'audit de sécurité d'Escouade 2.2.0 (SEC-03, gravité moyenne) a relevé qu'une seule clé minisign signait tout : la première main (dev.escouade.*), la liste de révocation, et demain les tiers. Elle est gardée sans mot de passe sur le poste de l'utilisateur, pour que le chef d'escouade signe seul la première main après sa revue, et l'app acceptait sa signature pour n'importe quel identifiant. Un poste compromis ou un agent détourné pouvait donc sceller une version piégée sous n'importe quel identifiant, et signer une liste de révocation. L'engagement « les tiers seront signés par une clé à mot de passe » n'était pas vérifiable. Le 2026-11-21 (demande a-5), l'utilisateur a choisi l'option « A · Séparer les clés, la première main bornée ».

## Décision

Les clés du store ont des rôles, écrits dans le code de l'app (STORE_KEYS de crates/escouade-plugins/src/store_keys.rs), du store et de leurs outils (par le nom du fichier dans store/keys/) : la clé de la première main (store.pub, la clé de tous les jours d'avant, E0516468FFDCBDD5, sans mot de passe) ne scelle que dev.escouade.* ; une clé distincte, à mot de passe (third-party.pub, créée par l'utilisateur par `npm run plugin-review -- create-key third-party`), scelle les plugins tiers et signe la liste de révocation ; la clé de secours vaut pour tout. Un sceau hors de son rôle est refusé avec sa raison par l'app, le store et l'outillage. La clé de la première main ne signe plus de liste de révocation ; seule la nº 1 qu'elle a signée avant tient encore, par l'empreinte de ses octets. Chaque sceau dit sa clé (catalogue `review.key`, page du plugin au store, vue Plugins de l'app), et la page d'un plugin dit que la revue est faite par un agent d'IA, puis signée. L'outillage choisit la clé par l'identifiant ou le rôle, refuse une clé des tiers sans mot de passe, et laisse minisign demander le mot de passe à chaque signature sans le lire ni le garder.

## Conséquences

Raison du choix : borner la clé par le code rend l'engagement vérifiable (le code et ses tests le montrent, chaque sceau nomme sa clé) ; protéger la clé des tiers et de la révocation par un mot de passe retire le pire cas (un tiers scellé ou une liste signée à l'insu de l'utilisateur) ; la première main garde son rythme, le chef la signe toujours seul. Ce qui suit : l'utilisateur crée la clé des tiers et de la révocation sur son poste (demande « À toi »), puis le chef commite store/keys/third-party.pub et sa ligne de STORE_KEYS ; d'ici là, un tiers et une nouvelle liste ne se signent que par la clé de secours. Les 119 sceaux publiés, tous dev.escouade.*, restent valides. Une app d'avant la 2.2.6 ne connaît pas les rôles et garde sa dernière liste ; un retrait d'urgence qui doit l'atteindre se signe par la clé de secours. Le vol de la clé de la première main reste possible (elle est sans mot de passe), mais il ne donne plus que dev.escouade.*, et une version forgée doit encore être servie et installée d'un clic. Le rapport d'audit public dit SEC-03 corrigé dans la 2.2.6.
