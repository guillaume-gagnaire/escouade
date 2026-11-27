# Les pages des agents

Un agent te montre une page dans Escouade, à côté de sa conversation : une revue avant fusion avec ses choix, un schéma, des options à comparer ou son propre HTML. Il la fait défiler et met en évidence ce dont il parle, et tu tranches dans la page, sans qu’elle puisse répondre à ta place.

![Une revue avant fusion montrée par un agent à côté de sa conversation : la bande « montre-moi-fake pilote », le critère en évidence, les repères numérotés sur le rail et dans la conversation](/images/agent-pages.jpg)

## À quoi ça sert

Certaines choses se lisent mieux en page qu’en texte : la revue d’un lot de tickets avec ses critères et ses changements, un schéma d’architecture, trois directions de design côte à côte. L’agent remplit un modèle de ses données, ou écrit son HTML, et Escouade ouvre la page dans une ligne de « Navigateurs », à côté de sa conversation. Puis il t’y guide, geste par geste : « je défile jusqu’au critère 3, celui qui coince ».

C’est le premier des deux modes. Le second, un vrai onglet de l’app développée ou d’un site, que l’agent ouvre et pilote, est dans [les agents et les navigateurs](agent-browsers).

## Ce qu’un agent montre

L’agent appelle `show_page` avec un modèle et ses données, ou du HTML :

| Modèle | Ce qu’il montre |
|---|---|
| « Revue » (`review`) | Une revue de fin de lot ou d’avant fusion : le ticket et sa branche, les tests, les critères d’acceptation (atteint, à moitié, non atteint), les changements avec leurs diffs, les risques, et un onglet par ticket pour un lot. |
| « Schéma » (`diagram`) | Un schéma Mermaid, avec zoom et déplacement. |
| « Comparaison » (`compare`) | De 2 à 8 options côte à côte, leurs images (clair et sombre), leurs pour et contre, la recommandée marquée. |
| « Page HTML » (`html`) | Le HTML de l’agent, ou un fichier `.html` de son dossier de travail avec les fichiers qu’il nomme. |
| « Page à soi » (`custom`) | La page que tu as réglée pour le projet, remplie des données de l’agent (plus bas). |

Les modèles suivent la langue et le thème de l’app, et la couleur du projet. L’agent ne donne que des données : Escouade les vérifie et les écrit en texte, sans jamais exécuter ce qu’elles contiennent.

## Où elle s’ouvre

- **À côté de la conversation de l’agent, si tu la regardes**, et la conversation reste le volet actif : tu continues de la lire ou d’y écrire. Si tu es ailleurs, la ligne de la page attend dans « Navigateurs » avec un point ambre, et s’ouvre quand tu reviens sur la conversation ou que tu cliques la ligne. Ton écran n’est jamais pris (voir [les agents et les navigateurs](agent-browsers#ton-ecran-n-est-jamais-pris)).
- **Sa ligne** `page:p-12` porte l’icône d’une page à la couleur du projet, son titre et sa version (« v2 »). Ses gestes sont ceux d’un navigateur : clic, `Ctrl`+clic, glisser dans un volet, et son menu (« Ouvrir à côté », « Ouvrir en dessous », « Fermer la page (elle reste gardée) »).
- **Sa carte dans la conversation** dit « Montrée » ou « Mise à jour », où est la page (« à côté », « attend dans Navigateurs », « fermée ») et propose « Afficher », ou « Rouvrir » une fois fermée.

## Ses versions

L’agent met sa page à jour au lieu d’en ouvrir une autre : la nouvelle version se recharge à sa place. La barre de la page porte **« v2 ▾ »**, le menu des versions gardées (les 10 dernières). Sur une version plus ancienne, une bande dit « Tu regardes la v1 ; la dernière est la v3 », avec « Revenir à la v3 ».

La barre a aussi les largeurs (pleine, tablette, mobile), comme un navigateur, mais ni adresse, ni retour, ni rechargement : la page est une copie, pas un site.

## Le pilotage d’une page

Sur sa page, l’agent montre seulement : il fait défiler, met en évidence une partie (un critère, un fichier, un nœud du schéma), change de largeur, ouvre un onglet de la page (celui d’un ticket d’un lot) ou montre une autre version. Il ne clique, ne tape ni ne choisit rien.

- **La bande** au-dessus de la page dit « esc-412 pilote » et la légende de son dernier geste, avec « Reprendre la main ».
- **Les repères** : chaque geste est une rangée numérotée dans la conversation (« ③ Met en évidence « Critère 3 » ») et le même numéro sur un rail à gauche de la page. Un clic sur un repère, ou sur la rangée, ramène la page où l’agent l’avait mise.
- **Ton premier geste dans la page** ou « Reprendre la main » l’arrête aussitôt ; « La rendre » le laisse reprendre. « Continuer » paraît quand il attend que tu aies regardé.

La main, la bande et les bornes sont les mêmes que pour un onglet : voir [ce que tu vois pendant qu’il pilote](agent-browsers#ce-que-tu-vois-pendant-qu-il-pilote).

## Tes choix

Une page peut porter une question, déposée en même temps dans ta boîte [« À toi »](inbox) : la revue a sa section « Ton verdict » avec ses options, chaque option d’une comparaison son bouton « Choisir ».

- **Un clic dans la page propose, rien de plus.** Sous la page, une bande d’Escouade dit ce que tu as choisi avec les mots de la question, jamais ceux de la page (« Choisi dans la page : A · Fusionner dans main »).
- **« Envoyer à esc-412 »**, dans cette bande, envoie ta réponse ; « Annuler » la laisse tomber. Rien ne part sans ce geste : une page ne peut ni répondre seule, ni changer ton choix sous ton clic.
- **Un feu vert** (fusionner, publier, signer) ne se donne jamais depuis la page : la bande le dit et porte son propre « Oui » et « Non ».
- Ta réponse revient à l’agent comme toute réponse de la boîte, et la page la montre, ses options figées. Dans « À toi », la demande montre une vignette de la page et « Ouvrir la page ».

## Une page à part, sans réseau

La mention de la bande le dit : « Écrite par esc-412 · hors réseau, sans accès à Escouade ni à tes fichiers ».

- Escouade **copie** la page au moment de l’appel et la sert dans sa propre webview, à son adresse à elle. Elle ne charge rien du web, n’appelle aucune commande d’Escouade, ne garde rien d’une ouverture à l’autre, et n’ouvre ni fenêtre, ni téléchargement.
- Les fichiers qu’elle nomme viennent du dossier de travail de l’agent seulement : un chemin qui en sort, un fichier caché (`.env`, `.git`) ou absent est laissé de côté.
- Un lien vers le web n’est pas suivi : une bande dit « La page veut ouvrir… », avec « Ouvrir dans le navigateur du système » et « Ignorer ».
- Ce qu’elle dit vient de l’agent : relis-la avant de copier une commande. N’y attends aucun secret, et l’agent n’y met que ce qu’il a relu.

Une page appartient à l’agent qui l’a montrée : un autre agent ne la connaît pas, et rien de ce que tu fais dedans ne lui revient, sauf la réponse que tu envoies. Elle reste gardée tant que son agent existe, puis part après le délai de « Garder les fichiers des demandes closes ».

## La page à soi

Réglages › Projets › *ton projet* › Lancement, groupe « Pages des agents » : **« Page à soi »** est un document `.html` du projet que tes agents remplissent de leurs données (le modèle « Page à soi »). Elle tourne dans la même cloison que les autres pages et lit ses données dans `window.escouade`. Un dépôt peut en proposer une par `pages.custom` de [.escouade.yml](escouade-yml), appliquée seulement après ton accord ; celle de ton poste passe devant.

## L’interrupteur

Réglages › Agents, sous « Les agents peuvent piloter Escouade », **« Pages des agents »** (allumé par défaut). Coupé, les agents n’ont plus `show_page`, `get_page` ni `close_page`, les lignes des pages se ferment et leurs cartes disent « Les pages des agents sont coupées (Réglages › Agents) ». Rien n’est effacé : rallumé, les pages sont là. Les outils sont décrits dans [le serveur MCP](mcp#les-pages-des-agents).

> [!TIP]
> Les artefacts de Claude Code, publiés sur claude.ai, ne changent pas : un agent Claude Code les garde pour ce qui se publie, et peut t’ouvrir le dernier dans un onglet (voir [les agents et les navigateurs](agent-browsers#ouvrir-un-onglet-a-cote)).
