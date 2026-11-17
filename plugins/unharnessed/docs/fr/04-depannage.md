# Dépannage

Les messages ci-dessous sont ceux qu’Escouade et le plugin ACP affichent ; `{…}` est ce qui change d’une fois à l’autre.

## L’agent ne démarre pas

**« Unharnessed ne tourne qu’en Bypass (il ne demande jamais avant d’agir) : active « Autoriser le mode Bypass » dans Réglages › Agents, ou choisis un autre harness. »**
Unharnessed n’a que Bypass. Allume le réglage, après avoir lu l’onglet Permissions.

**« Unharnessed lit et écrit les fichiers lui-même, sans les faire passer par Escouade (son manifeste le dit) : la clôture des projets privés ne tiendrait pas, l’agent ne démarre pas dans ce projet privé. »**
Un de tes projets est privé : chaque agent a alors une clôture, qu’Unharnessed ne peut pas tenir. Le message vaut pour tout projet tant qu’un projet est privé.

**« Unharnessed n’a pas de mode qui ne change rien (son manifeste n’en dit aucun) : il ne tient pas une discussion en lecture seule. »**
Une discussion de l’onglet Docs ne tourne pas sur Unharnessed. Prends un autre harness pour elle.

**« Unharnessed introuvable. Installe-le ou indique son chemin dans les réglages. »**
Escouade n’a pas trouvé `unharnessed` dans le `PATH`. Le programme de l’extension VS Code est dans un dossier qui change à chaque version : donne son chemin dans Réglages › Comptes et harness (voir Installer).

**« Unharnessed est plus ancien que la version 0.9.8, la plus ancienne que son plugin tient. »**
Construis une version plus récente, ou mets l’extension à jour.

**« Aucun compte Unharnessed : ajoute-en un dans Réglages › Comptes et harness. »**
Ajoute le compte de la machine ou un nouveau compte.

**« Unharnessed n’a pas ouvert la session de l’agent : {raison}. S’il demande une connexion, connecte-le dans un terminal. »**
Le plus souvent, il n’a pas de modèle : aucun fournisseur configuré, ou un modèle local qui ne répond pas. Vérifie `unharnessed model list` dans un terminal, avec le même dossier de compte.

## Pendant un tour

**« La session Unharnessed n’a pas pu être reprise : nouvelle session (contexte vidé) »**
Sa session est perdue (son fichier n’est plus dans le dossier `sessions` du compte), ou Escouade est en 2.2.2, qui ne la reprend pas par `--resume` (voir Utiliser). L’agent continue sans le contexte d’avant ; ses consignes lui sont redonnées avec son prochain message.

**« Unharnessed : {erreur} »**
Unharnessed a fini le tour en erreur, avec ses propres mots (un fournisseur qui refuse, un modèle introuvable). Lis-les.

**Le premier tour est long**
À son premier lancement, Unharnessed construit son noyau Python avec `uv`. Sans `uv` dans le `PATH`, il ne le peut pas.

**Une réponse qui commence par des mots d’avant**
Ce sont ceux d’un tour qu’Unharnessed a lancé lui-même, quand une commande a fini après sa cellule (voir Utiliser).

## Le plugin ACP

Les messages du plugin qui fait tourner Unharnessed (« Le plugin d’Unharnessed a redémarré. », « … s’est arrêté après {n} plantages… ») sont dans la documentation du plugin ACP.
