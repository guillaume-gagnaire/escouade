---
title: "L'espace de l'auteur du store (/account)"
subject: "store"
sources: ["docs/STORE.md §2.2","design/store-account/INVENTAIRE.md","design/store-account/a-modal.png","store/app/components/StoreAccount.vue","store/app/components/StoreDialog.vue"]
tickets: ["ESC-229","ESC-232","ESC-318"]
decisions: [28]
compared: null
---

# L'espace de l'auteur du store (/account)

Détail des actions : `docs/STORE.md` §2.2 (il fait foi). Inventaire des états et aperçus : `design/store-account/`.

## La page

- **Mes plugins · login** : les bandeaux qui attendent l'auteur (un transfert qu'on lui offre, un dépôt passé chez lui sur GitHub, une version trouvée sur une nouvelle étiquette qui attend son clic), puis la liste de ses plugins : état (vérifié, publié non vérifié, en revue, retiré de la liste, supprimé…), dernière version, installations, 30 jours. Une première version refusée dit pourquoi sous sa ligne ; une soumission pas finie a une flèche « Reprendre la soumission ».
- Sous la liste, **l'App sur tes dépôts**.
- À droite, **le compte seul** : avis par e-mail, « Applications connectées » (ESC-317, le MCP du store, entre l'e-mail et les données), Mes données (export, déconnexion, suppression du compte, refusée tant qu'il reste des plugins).
- Sur un téléphone (≤ 720 px), une colonne : la colonne du compte passe sous la liste.

## La modale d'un plugin (ESC-318)

- Chaque ligne publiée a un **« ⋯ »** (nom accessible « Gérer <nom> ») qui ouvre la modale du plugin **sans quitter la liste**.
- Dans l'ordre (direction « la fiche », choisie sur aperçu) : nom, identifiant et état ; dépôt d'origine, transfert reçu, gel de 7 jours, suppression en cours avec « Annuler la suppression » ; installations, dont 30 jours, mises à jour et les 12 dernières semaines ; **Versions** (publier celle qui attend le clic, retirer une version publiée) ; **Publication** (nouvelles étiquettes publiées seules ou non ; dans la liste ou non) ; puis, à part dans un cadre rouge, **Céder ou supprimer** : transférer (login GitHub du compte, ou annuler un transfert proposé) et supprimer (l'identifiant retapé). Au pied : « Voir sa page publique ».
- **L'adresse garde l'état** : `?plugin=<id>` ouvre la modale (un lien, un rechargement). Sans `?plugin=`, aucune modale ; un `?plugin=` qui ne nomme aucun plugin publié de l'auteur n'en ouvre pas.
- **Sans JavaScript** tout marche : chaque action est un formulaire (POST puis redirection vers `?plugin=<id>&notice=…`), la page revient avec la modale ouverte et sa notice dedans ; la croix et le voile sont des liens vers `/account`.
- **Accessible** : `<dialog>` modal (`aria-modal`, `aria-labelledby` sur son titre), le focus va sur le titre et reste dedans (Tab, Maj+Tab), Échap, la croix ou un clic sur le voile la ferment et rendent le focus au « ⋯ » ; la page dessous ne défile pas.
- **Téléphone** : une feuille qui monte du bas, de toute la largeur ; ouvrir le transfert ou la suppression met le focus dans son champ.
- La notice d'une action va dans la modale quand elle est ouverte, sinon en haut de la page.
- Français et anglais (`store/app/data/account.ts`).
