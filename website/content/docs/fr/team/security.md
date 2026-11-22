# Le guide du RSSI

Pour le responsable de la sécurité qui évalue un serveur d’équipe : ce qu’il voit et ce qu’il ne voit pas, contre qui il protège et contre qui non, les clés, ce qu’une équipe peut imposer et comment c’est signé, et ce qui reste de confiance.

> [!WARNING]
> **L’audit externe de la cryptographie n’est pas fait.** Le schéma repose sur MLS (RFC 9420) et sur des primitives standard, mais leur assemblage par Escouade n’a été vérifié que par ses propres tests et son fuzzing. Un audit par des auditeurs extérieurs est prévu avant de dire le serveur stable ; d’ici là, le serveur d’équipe est un aperçu.

## En bref

- **Le serveur est aveugle.** Il garde, numérote et diffuse des octets chiffrés sur les postes. Il n’a aucune clé de contenu et ne lance rien : ni agent, ni commande, ni plugin.
- **Le chiffrement de bout en bout est le seul mode.** Il n’existe pas d’option où le serveur lit le contenu.
- **Il ne voit pas le contenu, il voit l’activité** : qui écrit, quand, combien, sur combien de projets. C’est détaillé plus bas, sans rien minimiser.
- **Une équipe ne peut que restreindre.** Rien de ce qui s’exécute n’est imposé par l’équipe : chaque commande attend la relecture et l’accord de chaque poste.
- **Les comptes des harness, les secrets, les conversations des agents et le code ne vont jamais sur le serveur.** Voir [Où vont mes données ?](team/data).

## Ce que voit le serveur

Le serveur est hébergé par l’équipe : elle choisit sa machine et qui l’administre. Voici ce que cet hébergeur, ou quiconque prend le serveur, peut lire.

| Visible du serveur | Caché au serveur |
|---|---|
| L’existence de chaque équipe, son nombre d’appareils, la clé publique de chacun, son rôle sur le serveur (administrateur ou membre), révoqué ou non | Les noms des membres et de l’équipe : le nom d’une équipe reste sur chaque appareil |
| Le nombre d’époques de l’équipe, c’est-à-dire de changements de ses membres et de ses clés | Qui est membre de quoi dans le registre de l’équipe, qui part dans des messages chiffrés |
| Chaque écriture : son flux (l’équipe ou un projet), sa sorte, l’appareil qui l’a écrite, son heure, sa taille | Ce qu’elle dit : les opérations, les réglages, les invitations, les enveloppes de clés |
| Le nombre de projets, sous des identifiants opaques, et leurs flux | Le nom des projets, leurs dépôts et leurs branches : la liste des projets de l’équipe est chiffrée |
| Les fichiers : un identifiant opaque, la taille, l’appareil qui l’a écrit, l’heure | Leur nom, leur type, leur contenu |
| L’empreinte SHA-256 des jetons d’inscription, leur expiration, un second usage | Le secret d’une invitation, qui reste après le `#` du lien |
| Les adresses IP des appareils, gardées en mémoire pour la limite de débit, et les heures de connexion | Le `.escouade.yml` de l’équipe et ce qu’il impose |

- **Ce que ça révèle** : l’activité de l’équipe (qui travaille quand, à quel rythme, sur combien de projets), la taille des objets, les changements de ses membres.
- **Le journal du serveur** ne garde que la méthode, le modèle de la route, le statut et la durée de chaque requête : ni identifiant, ni adresse IP, ni jeton. Le journal du proxy inverse devant lui, en revanche, garde les adresses IP.
- **Les métriques** n’ont aucun identifiant d’équipe, de projet ni d’appareil.
- **À venir** : la prise d’un ticket, quand le Kanban d’équipe sera là, sera en clair sur le serveur (quel appareil tient quel identifiant opaque, jusqu’à quand), pour qu’il garantisse que deux agents ne prennent jamais le même ticket. Le serveur ne saura pas quel ticket c’est.

Les tests d’Escouade cherchent les identifiants dans le journal et les métriques du serveur, et les titres, les noms et les adresses dans ce qu’il garde, et ne les trouvent pas.

## Le modèle de menace

| Menace | Ce qu’elle obtient |
|---|---|
| Le serveur pris, ou son administrateur système curieux | Les octets chiffrés et les métadonnées ci-dessus. Il ne peut ni lire, ni écrire au nom d’un membre, ni glisser sa propre clé dans l’équipe. Il peut refuser le service, retarder, ou montrer un état ancien, ce que les appareils voient. |
| Une sauvegarde du serveur qui fuit | Les mêmes octets chiffrés et métadonnées. |
| Un lien d’invitation qui fuit | L’entrée dans l’équipe, s’il sert avant l’invité : l’invité légitime voit alors que l’invitation a déjà servi, et l’administrateur en est prévenu. Le lien ne porte aucune clé. |
| Un membre qui part | Rien de ce qui s’écrit après son départ. Il garde ce qu’il a déjà lu. |
| Un appareil volé | Rien après sa révocation. Avant : ce que protègent le trousseau du système, la session verrouillée et le chiffrement du disque. |
| Le poste d’un membre compromis | Tout ce que ce membre lit. C’est hors du champ : ses agents tournent sur ce poste et lisent déjà le code et les docs. |

### Comment le serveur est tenu

- **Il ne peut pas glisser une clé.** L’équipe est un groupe MLS ; chaque appareil vérifie chaque changement des membres contre les règles de l’équipe (un administrateur ajoute et retire, un membre n’ajoute que ses propres appareils), et n’enveloppe une clé que pour un appareil de ce groupe vérifié.
- **Il ne peut pas forger une écriture.** Chaque opération est signée par l’appareil qui l’écrit, chaînée aux précédentes du même appareil, et chiffrée avec, en données associées, son projet et son époque : le serveur ne peut ni la déplacer, ni la rejouer, ni la réordonner, ni en retirer une sans que ça se voie.
- **Il ne peut pas revenir en arrière.** Chaque appareil garde, dans le trousseau, la plus haute époque vue et sa tête, et refuse un état plus ancien, du serveur comme d’une sauvegarde remise sur le disque.
- **Une fourche se voit.** S’il montre deux versions de l’équipe à deux membres, la première écriture qui passe d’une version à l’autre le révèle. L’app dit alors « Le serveur montre deux versions de l’équipe : contacte un administrateur. », cesse de lire l’équipe et de lui écrire, et montre les six mots de la tête de l’équipe à comparer de vive voix.

### Ce contre quoi il ne protège pas

- **Le poste compromis d’un membre**, ni ce qu’un membre a déjà lu avant de partir.
- **L’analyse de l’activité** : le serveur voit qui écrit, quand et combien. Les tailles ne sont pas arrondies.
- **Le déni de service** : le serveur, ou un administrateur, peut empêcher l’équipe de travailler ensemble. Chaque poste garde alors sa copie, lisible hors ligne.
- **Une partition durable** : un serveur qui montre pour toujours deux versions de l’équipe, sans que rien ne passe de l’une à l’autre, ne se voit qu’en comparant hors bande les six mots de la tête. De même, une version des réglages cachée à un appareil qui ne l’a jamais vue ne se voit pas de cet appareil seul.
- **Le détenteur de la clé de séquestre**, si l’équipe en a une : il lit tout, par construction (voir plus bas).
- **La copie locale** de chaque projet de l’équipe, sur chaque poste, n’est pas chiffrée, comme le Kanban local d’aujourd’hui : seules les clés sont dans l’état chiffré. Le chiffrement du disque du poste la protège.

## Les clés

- **Chaque appareil a ses clés, par équipe**, créées sur lui et jamais exportées : une clé Ed25519 pour se connecter au serveur (elle signe un défi : il n’y a aucun mot de passe), une clé Ed25519 de membre du groupe MLS (elle signe ses écritures), et la clé qui chiffre son état sur le disque. Deux clés de signature plutôt qu’une : le serveur ne peut pas relier un appareil à ses écritures.
- **Où elles vivent** : dans le trousseau du système, le gestionnaire d’identifiants de Windows, le trousseau de macOS. Sous Linux, où Escouade n’a pas de trousseau, elles vont dans un fichier lisible du seul propriétaire, et la carte de l’équipe le dit. Le point de contrôle contre le retour en arrière est alors sur le même disque : sa garde est affaiblie.
- **Le chiffrement** : MLS (RFC 9420) par openmls, en suite `MLS_128_DHKEMX25519_CHACHA20POLY1305_SHA256_Ed25519`. Les clés du contenu sont dérivées du groupe à chaque époque, puis par projet et par objet ; le contenu est chiffré en XChaCha20-Poly1305 et signé en Ed25519. Rien n’est inventé en cryptographie : des primitives standard, en Rust (RustCrypto).
- **L’historique se lit** : un nouvel appareil remonte les époques passées par une chaîne de liens. C’est voulu, car un Kanban et une spec durent ; l’envers est qu’un appareil compromis livre tout l’historique qu’il peut lire.
- **L’invitation** : le lien porte un secret après le `#`, que ni les navigateurs ni HTTP n’envoient au serveur. Le serveur ne garde que l’empreinte d’un jeton dérivé de ce secret, et ne le laisse servir qu’une fois ; il expire (72 h par défaut). L’invité vérifie les six mots de l’empreinte de l’équipe avec qui l’invite, et un administrateur en ligne le fait entrer. Le serveur ne peut ni substituer sa clé à celle de l’invité, ni lui montrer une fausse équipe.
- **Un autre appareil du même membre** entre par un QR code (un lien valable une heure) ou par un code court de 8 caractères, échangé par SPAKE2 : un seul essai, 10 minutes, et rien que le serveur puisse essayer hors ligne.

### La clé de secours et la clé de séquestre

- **La clé de secours**, obligatoire : 24 mots, montrés une fois au propriétaire à la création de l’équipe, qui en retape trois. Elle compte comme un administrateur hors ligne. Si tous les administrateurs perdent leurs appareils, l’hébergeur donne un jeton de récupération et les 24 mots ramènent l’équipe sur un nouvel appareil ; une nouvelle clé de secours remplace alors l’ancienne, qui n’ouvre plus rien. Garde-la imprimée, ou dans le gestionnaire de mots de passe de l’organisation, avec les six mots de l’équipe que la feuille de secours porte aussi : la récupération demande les deux. Elle ne suit qu’une enveloppe signée par un administrateur de l’équipe, d’une clé tirée des 24 mots que seuls les appareils administrateurs tiennent : ni un membre ni le serveur ne peuvent la faire entrer dans un autre groupe. Après le départ ou la rétrogradation d’un administrateur, la carte de l’équipe dit d’en faire une nouvelle.
- **La clé de séquestre**, en option, à la création : une paire tenue par l’organisation, à qui tout le contenu est aussi chiffré. L’organisation peut alors toujours le lire, ce que certaines entreprises exigent. **Chaque membre le voit**, sur la carte de l’équipe : « Cette équipe a une clé de séquestre : l’organisation peut lire son contenu. Détenteur : … ». Elle lit, elle n’écrit rien, et seulement ce qu’un administrateur de l’équipe a scellé et signé.
- **Un membre qui perd ses appareils ne perd rien** : un administrateur l’invite de nouveau.
- **Un seul administrateur** est un risque : l’app le dit tant qu’il n’y en a qu’un, et propose d’en nommer un second.

## Le départ d’un membre et la rotation

- Retirer un membre fait d’abord couper ses appareils par le serveur (ses sessions et ses connexions en direct se ferment), puis ouvre une nouvelle époque sans eux : ce qui s’écrit ensuite lui échappe. Révoquer un appareil volé fait de même pour cet appareil.
- L’ancien contenu n’est pas rechiffré : le membre parti l’a déjà eu, et le rechiffrer ne lui retirerait rien. Le serveur ne le lui sert plus.
- Dans l’app, l’écran des membres et des appareils de l’équipe, qui retire un membre, révoque un appareil ou change les clés à la demande, est à venir. Aujourd’hui, un appareil se retire lui-même d’une équipe (« Retirer de cet appareil ») ; il reste dans le groupe jusqu’à ce qu’un administrateur le retire.

## Ce qu’une équipe peut imposer

Une équipe recommande et impose des réglages par son `.escouade.yml` d’équipe : un fichier signé par ses administrateurs, chiffré par la clé de l’équipe, qui vit sur le serveur et jamais dans un dépôt. Ses clés sont celles des [stratégies du poste](managed-policies#les-strategies) et du [.escouade.yml](escouade-yml) d’un dépôt.

- `defaults` : ce que l’équipe recommande ; le dépôt et le poste le changent.
- `enforced` : ce qu’elle impose. Le réglage est grisé, avec un cadenas et qui l’impose, et c’est le cœur d’Escouade qui refuse de le changer, par la fenêtre, par le serveur MCP comme par le `.escouade.yml` du dépôt.
- `proposed` : ce qui s’exécute, proposé seulement (voir [le piège du hook](#le-piege-du-hook)).
- `reasons` : la raison de chaque réglage, montrée au membre.

Ce qu’impose une équipe ne vaut que pour ses projets. Les réglages de l’app elle-même (le réseau, la télémétrie, la voix, les mises à jour, les plugins) valent pour l’app tant que le poste est dans l’équipe : la place de l’équipe comme espace à part, qui les bornera à elle, est à venir.

### Les sortes de clés

Chaque clé a sa sorte, fixée par l’app, pas par l’équipe :

| Sorte | Exemples | Ce qu’elle devient |
|---|---|---|
| Restreindre | les modes de permission, les harness et les modèles permis, la télémétrie éteinte, le Remote Control interdit | Appliquée tout de suite. Le pire qu’un pirate en tire est un déni de service. |
| Valeur | la branche cible, un préfixe de branche, ce que fait la validation | Appliquée et montrée. |
| Sensible | l’adresse d’un proxy, ses exclusions, un fichier d’autorités de certification | En attente de l’accord de chaque membre, sur son poste, avec ce qu’elle permet dit en clair (« Ce certificat permet de lire le trafic TLS d’Escouade. »). Une valeur changée redemande l’accord. |
| Exécutable | les commandes des worktrees, `launch`, la commande de test, les hooks, un serveur MCP, les variables d’environnement des agents, un plugin à installer | Jamais imposée. Seulement proposée, relue et acceptée sur chaque poste à son empreinte. |

### Comment c’est signé

- **Chaque version** du fichier porte son numéro, l’empreinte de la précédente, le nom que son auteur donne et l’heure. Elle est signée par la clé de l’appareil d’un administrateur. Un appareil refuse une version non signée, forgée par le serveur, signée par qui n’est pas administrateur ou ne l’est plus, rejouée, ou qui recule ; il ne défait jamais la chaîne qu’il a choisie.
- **Le quorum** : une version qui change une clé sensible, la liste des signataires (`signers`) ou le quorum lui-même attend la signature d’un second administrateur. Deux appareils d’un même administrateur comptent pour un. Le quorum est allumé d’office dès que l’équipe a deux administrateurs ; avec un seul, il ne s’applique pas, et l’écran le dit.
- **Chaque changement est montré** : une bande sur toute la largeur de la fenêtre dit « L’équipe Acme a changé 2 réglages · Claire Martin, il y a 5 min · Voir », et l’historique de chaque version, avec son changement clé par clé, se lit dans les réglages de l’équipe.

## Le piège du hook

Le scénario : quelqu’un prend le serveur, ou le poste d’un administrateur, et pousse dans le fichier de l’équipe une commande de préparation des worktrees (`curl https://… | sh`), un hook, un plugin à installer d’office, une adresse d’API qui détourne les requêtes des agents, ou un certificat racine qui intercepte tout le TLS. À la synchronisation suivante, chaque poste l’exécuterait : la prise de toute la flotte en une minute. Un serveur central de configuration est la cible rêvée.

Les défenses, en couches :

1. **Le serveur ne peut rien forger** : une version non signée par un administrateur de l’équipe est refusée.
2. **Chaque changement est montré** à chaque membre, avec son auteur et son diff.
3. **L’équipe ne peut que restreindre** : une clé sensible attend l’accord de chaque membre, une clé qui s’exécute n’est jamais imposée. Imposer une commande ne peut vouloir dire qu’une chose : « seule celle-ci est permise », jamais « lance-la ».
4. **Le quorum** : un administrateur pris seul ne fait passer aucune clé sensible.
5. **Ce qui s’exécute ne s’impose que par le poste géré** : un MDM installe déjà ce qu’il veut sur la machine. Le serveur d’équipe n’ajoute aucun pouvoir d’exécution à ceux qu’a déjà le service informatique.

**Ce qui n’entre jamais, même signé.** Une liste écrite dans l’app, que l’équipe ne change pas. Une version qui en contient une ligne est refusée entière, à la lecture, et l’écran de l’administrateur refuse de la signer :

- `bypassPermissions`, nommé n’importe où, ou permis ;
- une commande qui tournerait sans accord : une clé qui s’exécute hors de `proposed`, ou une proposition marquée acceptée d’avance ;
- un plugin installé sans le geste du membre ;
- un plugin `dev.escouade.*` signé par l’équipe ;
- un projet rendu public par le fichier.

Ces règles prolongent celles du [.escouade.yml](escouade-yml) d’un dépôt, dont les commandes ne tournent déjà qu’une fois approuvées sur chaque machine.

## Le store de plugins privés

Une équipe publie ses propres plugins sur son serveur, signés par sa clé, installables par ses seuls membres. Le serveur ne voit que des fichiers chiffrés : les contrôles tournent sur les postes.

- **La clé de l’équipe** est une paire minisign à part, avec sa clé de secours, créée sur le poste d’un administrateur (la fenêtre des équipes, « Plugins »), jamais sur le serveur. Elle reste scellée par un mot de passe sur ce poste ; la clé de secours va dans le dossier qu’il choisit, à garder hors ligne. Ses clés publiques et **son espace de noms** (`fr.acme.*`) sont déclarés à l’équipe par une entrée signée de l’administrateur.
- **Les refus en dur de l’app** : une signature de l’équipe hors de son espace ; `dev.escouade.*`, jamais ; un espace qui contient un plugin installé sur le poste d’une autre origine. Un plugin de l’équipe ne remplace jamais un plugin du même id venu d’ailleurs : sa page dit « Acme · privé ».
- **La publication** : les contrôles du store public (le manifeste, la version égale à l’étiquette et plus grande que la dernière, l’archive saine, les permissions, aucun binaire, la compilation deux fois dans un dossier neuf) tournent sur le poste de qui propose, puis à nouveau sur celui de l’administrateur qui signe ; il ne signe que si les deux empreintes sont les mêmes. `escouade-plugin team-check` les joue sans l’app.
- **La liste de révocation de l’équipe**, signée par sa clé, au format de celle du store, son numéro ne reculant jamais : une version retirée est désactivée sur chaque poste à sa lecture suivante.
- **La politique d’installation** : les clés `plugins.allowedOrigins` (`team-store`), `plugins.allowed` et `plugins.denied` du fichier de l’équipe et des [stratégies du poste](managed-policies). Installer reste le geste du membre, depuis la vue Plugins, après la fenêtre des permissions ; un agent ne publie ni ne signe rien.

## Les stratégies du poste

Le service informatique impose aussi des réglages par ses propres outils, sans aucun serveur d’équipe : stratégies de groupe ou Intune sous Windows, profil de configuration sous macOS, fichiers sous `/etc/escouade/policies` sous Linux. Voir [les stratégies du poste](managed-policies).

- **Le poste géré passe devant l’équipe.** Une stratégie du poste vaut pour toute l’app ; celle de l’équipe, pour ses projets.
- **Pour une même clé** : une borne (les harness permis, les modes permis) se croise, il reste ce que les deux permettent ; une valeur (l’adresse du proxy) du poste géré gagne.
- **Il enrôle et borne** : `team.server` et `team.fingerprint` posent le serveur et l’empreinte de l’équipe de l’entreprise, `team.onlyThisServer` refuse tout autre serveur d’équipe, `servers.allowed` borne les serveurs joints. Le préremplissage de l’écran d’entrée dans l’équipe par ces valeurs est à venir ; le refus est en place.

## Ce qui reste de confiance

- **Le code d’Escouade sur chaque poste**, et ses mises à jour : c’est lui qui vérifie les signatures, les chaînes et les règles. Le code est ouvert.
- **openmls et les bibliothèques de cryptographie**, et leur assemblage, que l’audit externe n’a pas encore lu.
- **Les administrateurs** : ils peuvent restreindre, poser des valeurs et proposer des commandes, et un administrateur en ligne fait entrer les invités. Le quorum borne ce qu’un seul peut faire.
- **Les membres** et leurs postes, avec leurs agents.
- **Le trousseau du système** de chaque poste.
- **Le détenteur de la clé de séquestre**, si l’équipe en a une, et celui de la clé de secours.
- **L’hébergeur du serveur**, pour la disponibilité et les métadonnées, pas pour le contenu.
- **L’hébergeur de code** (GitHub, GitLab…), qui garde le code : le serveur d’équipe n’y donne aucun accès.

## Ce qui n’est pas encore là

- L’audit externe de la cryptographie (à venir, avant de dire le serveur stable).
- Le Kanban d’équipe en direct, et la prise d’un ticket par un seul agent (à venir).
- L’écran des membres et des appareils, la place de l’équipe comme espace à part, les rôles propriétaire et lecteur (à venir).
- L’entrée par l’annuaire de l’entreprise (OIDC) : aujourd’hui, on n’entre que par invitation.
- Les droits par projet sur le serveur : il sert un projet à tout appareil de l’équipe ; seul le chiffrement protège un projet gardé à quelques membres.
