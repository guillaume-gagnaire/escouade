# L’app mobile

L’app mobile d’Escouade, pour Android et iPhone, montre ce qui t’attend sur ton poste et tes serveurs et te laisse y répondre. Cette page dit comment l’installer, l’appairer, y répondre et la révoquer.

## L’installer

L’app arrive d’abord en test fermé sur Google Play et dans TestFlight, puis sur les deux stores. Elle demande Escouade 2.3 ou plus récent sur le poste ou le serveur qu’elle joint, et un téléphone avec un verrou d’écran. L’iPad n’a pas de mise en page à lui.

## L’appairer

1. Sur ton poste : Réglages › Téléphone, allume « Accès depuis le téléphone », puis « Appairer un téléphone » montre un QR code. Sur un serveur : `escouade-server pair --qr`.
2. Dans l’app : « Scanner le QR code », ou tape le code à 8 caractères qui l’accompagne avec l’adresse de l’espace.
3. Donne un nom à ce téléphone, puis « Appairer ce téléphone ».

Le QR code vaut 10 minutes, pour un seul téléphone. Le téléphone joint ensuite la machine par un tunnel WireGuard intégré à Escouade : un seul port UDP ouvert dans la box, et l’API n’est servie que dans le tunnel. Les clés restent dans le Keystore d’Android ou le Trousseau d’iOS. Sans verrou d’écran, l’app refuse de s’appairer.

Chaque machine appairée est un [espace](spaces) : « Partout » réunit ce qui attend dans chacun, et une puce par espace filtre le fil.

## Les trois onglets

- **À toi** : les agents arrêtés (une autorisation, une question, un plan), le plus ancien d’abord, puis les demandes de la [boîte « À toi »](inbox). Une ligne se déplie sur place.
- **Agents** : ceux qui attendent ta réponse, puis ceux qui travaillent, avec ce qu’ils font. Un agent ouvre sa conversation, où lui écrire ou dicter un message, avec une photo.
- **Kanban** : le [Kanban](board) d’un projet, ses colonnes en segments. Un ticket « À tester » se valide ou se renvoie à son agent.

Le « + » en haut crée un agent, un ticket dicté ou un projet sur un serveur.

## Répondre sous le verrou

Une commande à autoriser se lit en entier avant le bouton. Chaque réponse qui fait agir un agent (autoriser une commande, approuver un plan, valider un ticket, donner un feu vert) passe par le verrou du téléphone : l’empreinte, le visage ou le code. Une annulation ne change rien : la demande reste ouverte.

« Toujours autoriser », les règles de permission et le mode Bypass ne se règlent jamais depuis le téléphone : ils se posent au poste, en voyant tout le contexte.

## Hors ligne

Quand un espace est coupé, sa puce le dit, une bande propose « Réessayer », et l’app garde ce qu’il a dit en dernier, daté. Répondre attend la connexion : une réponse ne part jamais d’un écran périmé. En arrière-plan, l’app ferme ses connexions et les rouvre en revenant.

## Les notifications

À l’appairage, l’app propose les notifications. Leur texte est scellé par ton poste avec la clé de l’appairage : le relais de l’éditeur, Google et Apple ne voient qu’un réveil. La toucher rouvre ce qu’elle annonce, ou dit que la demande a été réglée entre-temps. Sans elles, rien ne passe par l’éditeur.

## Révoquer un téléphone

Sur le poste, Réglages › Téléphone › « Les téléphones » : « Retirer » révoque à la fois son tunnel, son jeton et son inscription aux notifications. Le téléphone le dit à sa prochaine connexion (« Ce téléphone n’est plus appairé à… ») et efface ce qu’il savait de cet espace ; les autres ne bougent pas. Depuis l’app, « Tes espaces » dans la feuille du « + » retire un espace.

Ce que l’app envoie, et à qui : [la confidentialité de l’app mobile](https://escouade.dev/mobile/privacy/).
