# Permissions de Contraste élevé

## Ce qu’il est

Un plugin **déclaratif** : un manifeste, `escouade-plugin.json`, qui contient un thème (`contributes.themes`), et rien d’autre. Il n’a pas de programme : rien de lui ne s’exécute sur ta machine.

## Ce qu’il demande

Rien. La fenêtre d’installation et l’onglet **Permissions** le disent : « Il ne demande rien : ses contributions sont lues, rien de lui ne s’exécute. »

- Il ne reçoit aucun événement, ne décide d’aucune autorisation.
- Il ne lit ni n’écrit aucun fichier.
- Il ne garde aucun secret.
- Il ne joint aucun hôte : rien ne part de ta machine à cause de lui.

## Ce qu’Escouade fait de ses couleurs

Escouade lit ses valeurs et filtre chacune avant de la poser sur la fenêtre :

- seuls les jetons de couleur d’Escouade sont pris (`--text`, `--line`, `--run`…) ; un autre est ignoré ;
- une valeur doit être une couleur (`#rrggbb`, `color-mix()`, `var()` d’un autre jeton…), 160 caractères au plus ;
- jamais de `url()`, d’autre fonction, de guillemets, d’antislash, de `;`, d’accolade ni de `!important`.

Un thème ne peut donc rien charger, ni écrire du CSS à lui, ni masquer une demande d’Escouade (une autorisation, une question). Ce qui ne passe pas est écarté, et Réglages › Général dit combien de valeurs l’ont été.

## Ce qu’il ne peut pas faire

- Changer la forme de l’interface : polices, rayons, largeurs restent ceux d’Escouade.
- Agir quand il n’est pas choisi : installé, il ne change rien tant que tu ne le choisis pas dans **Couleurs**.
- Toucher à tes projets, tes agents ou tes fichiers.
