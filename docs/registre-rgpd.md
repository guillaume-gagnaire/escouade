# Registre des activités de traitement (RGPD, article 30)

Tenu par GAGNAIRE DEVELOPPEMENT pour Escouade. À mettre à jour à chaque changement de ce qui est collecté, de l'hébergement ou des durées. Dernière mise à jour : 26 octobre 2026 (traitements 3 et 4 en projet, pour la mise en ligne de store.escouade.dev, `docs/STORE.md`).

## Responsable du traitement

GAGNAIRE DEVELOPPEMENT, SAS au capital de 1 000 €, 2 rue du Taillis, 33620 Cavignac, France. RCS Libourne 953 269 263. Représentée par Guillaume Gagnaire, président. Contact : guillaume@gagnaire.dev. Pas de délégué à la protection des données (non obligatoire).

## Traitement 1 : statistiques d'usage de l'application

| Rubrique | Contenu |
|---|---|
| Finalité | Mesurer l'usage d'Escouade : utilisateurs actifs sur 30 jours, versions et systèmes en service, fonctionnalités utilisées, pour décider quoi améliorer et quand cesser de prendre en charge une ancienne version. |
| Base légale | Consentement (RGPD art. 6.1.a ; loi Informatique et Libertés art. 82 pour l'identifiant gardé sur l'appareil). Demandé au premier lancement, retirable à tout moment dans Réglages › À propos. |
| Personnes concernées | Les utilisateurs d'Escouade qui ont accepté. |
| Données | Nom de l'événement (liste fermée de 23, dans `src-tauri/src/telemetry.rs`) et ses propriétés (langue de l'interface, nombre de comptes et de projets, oui/non, service d'intégration) ; date ; identifiant de session aléatoire ; identifiant d'installation aléatoire (UUID v4, `~/.escouade/telemetry-id`) ; version de l'app, système et version, langue du système, moteur web et version. Côté serveur, Aptabase déduit de l'adresse IP le pays et la région, et un identifiant quotidien haché avec un sel changé chaque jour ; l'adresse IP elle-même n'est pas gardée par Aptabase. Aucun nom, e-mail, chemin, code, contenu ni secret. |
| Destinataires | GAGNAIRE DEVELOPPEMENT seul. |
| Sous-traitant | Scaleway SAS, 8 rue de la Ville-l'Évêque, 75008 Paris (serveur dédié, centre de données de Paris). |
| Transferts hors UE | Aucun. |
| Durée de conservation | 25 mois (règle TTL des tables `events` et `error_events` de ClickHouse, appliquée par le conteneur `aptabase_retention` du service Aptabase dans Coolify). |
| Sécurité | HTTPS ; instance Aptabase auto-hébergée (aptabase.gagnaire.dev), inscriptions fermées (la route `/api/_auth/register` ne mène nulle part) ; mots de passe des bases générés par Coolify, bases non exposées ; aucune collecte en bac à sable (`ESCOUADE_DATA_DIR`). |
| Exercice des droits | Par e-mail à guillaume@gagnaire.dev ou par une issue GitHub, avec l'identifiant du fichier `telemetry-id` ; réponse sous un mois. Suppression : les événements portant cet identifiant (`installId` dans `string_props`) sont effacés de ClickHouse. |

## Traitement 2 : journaux du serveur

| Rubrique | Contenu |
|---|---|
| Finalité | Sécurité et bon fonctionnement du serveur (détection d'abus, diagnostic). |
| Base légale | Intérêt légitime (RGPD art. 6.1.f). |
| Personnes concernées | Les visiteurs d'escouade.dev et les installations qui envoient des statistiques. |
| Données | Adresse IP, date, adresse demandée, navigateur (User-Agent), page d'origine (journal d'accès du proxy Traefik géré par Coolify). Le serveur web du site (nginx) ne tient pas de journal d'accès. |
| Destinataires | GAGNAIRE DEVELOPPEMENT seul. |
| Sous-traitant | Scaleway SAS (hébergement). |
| Transferts hors UE | Aucun. |
| Durée de conservation | Un an au plus : le conteneur `coolify-proxy-logrotate` (configuration du proxy dans Coolify) fait tourner le journal chaque jour et au-delà de 20 Mo, et supprime les fichiers tournés après 364 jours (variable `TRAEFIK_ACCESS_LOG_MAX_DAYS`). Mis en place le 14 octobre 2026, les lignes de plus d'un an des journaux existants alors purgées. |
| Sécurité | Fichiers sur le serveur, accessibles au seul administrateur. |

## Traitement 3 : comptes des auteurs du store

Mis en œuvre le 29 octobre 2026 avec les comptes de store.escouade.dev (ESC-229, STORE-2). Les plugins, les versions et les signalements y entrent avec la soumission et la revue (STORE-3 à STORE-8).

| Rubrique | Contenu |
|---|---|
| Finalité | Permettre aux auteurs de plugins de publier et de gérer leurs plugins sur store.escouade.dev (soumission, versions, transfert, retrait, statistiques), de recevoir les avis sur leurs soumissions, et de signaler un plugin ; tenir un journal des actions pour la sécurité et les litiges. |
| Base légale | Exécution des conditions des auteurs acceptées à la première soumission (RGPD art. 6.1.b) ; intérêt légitime pour les signalements et le journal des actions (art. 6.1.f). |
| Personnes concernées | Les auteurs qui se connectent au store avec GitHub, et les comptes qui signalent un plugin. |
| Données | Identifiant numérique GitHub, login, nom affiché et adresse de l'avatar (fournis par GitHub à la connexion) ; e-mail facultatif, saisi et vérifié par l'auteur ; jeton de session aléatoire ; jeton d'accès GitHub de la session, chiffré ; plugins, versions, résultats des contrôles, résumés de revue ; texte des signalements et compte qui signale ; journal des actions (compte, action, date). Aucune donnée des utilisateurs de l'app. |
| Destinataires | GAGNAIRE DEVELOPPEMENT seul. Le login, le nom affiché et l'avatar d'un auteur sont publics sur la page de ses plugins ; le compte qui signale ne l'est jamais. |
| Sous-traitants | Scaleway SAS (hébergement, et stockage objet des sauvegardes à Paris). GitHub, Inc. est le fournisseur d'identité choisi par l'auteur (connexion par une GitHub App en lecture seule). |
| Transferts hors UE | Aucun depuis le store. L'identité arrive de GitHub, Inc. (États-Unis, cadre de protection des données UE–États-Unis), à la demande de l'auteur qui se connecte. |
| Durée de conservation | Compte, e-mail et plugins : jusqu'à la suppression du compte, puis 30 jours dans les sauvegardes. Sessions : 30 jours. Jeton GitHub : la durée de la session, 8 h au plus. Signalements : 1 an après leur clôture. Journal des actions : 1 an. Signatures et révocations publiées : sans limite (elles doivent rester vérifiables ; elles ne portent pas de donnée personnelle). |
| Sécurité | HTTPS et HSTS ; un seul cookie, de session, `__Host-store_session`, `HttpOnly`, `Secure`, `SameSite=Lax` ; la base ne garde que l'empreinte (SHA-256) des jetons de session et des liens de vérification d'e-mail, et le jeton GitHub chiffré (AES-256-GCM, clé tirée de `NUXT_SESSION_SECRET`), révoqué chez GitHub à la déconnexion et à la suppression ; formulaires refusés s'ils ne viennent pas du store (en-tête `Origin`) ; webhooks de GitHub refusés sans leur signature (HMAC-SHA256, comparée en temps constant) ; politique de sécurité du contenu stricte, sans script en ligne hors de son nonce ; aucune image chargée chez un tiers ; base PostgreSQL non exposée, mot de passe généré par Coolify ; secrets de la GitHub App dans les variables de Coolify ; la clé de signature des plugins n'est jamais sur le serveur. Purge chaque nuit de ce qui a passé sa durée. |
| Exercice des droits | Dans le compte (`/account`) : export des données (JSON), suppression du compte (le journal garde la suppression un an, par identifiant GitHub). Sinon par e-mail à guillaume@gagnaire.dev ; réponse sous un mois. Rectification du nom et de l'avatar chez GitHub (relus à chaque connexion). |

## Traitement 4 : compteurs d'installations des plugins (en projet)

| Rubrique | Contenu |
|---|---|
| Finalité | Montrer à l'auteur d'un plugin et dans le store combien de fois il est installé et mis à jour. |
| Base légale | Aucune donnée personnelle n'est gardée. L'app n'envoie le compte que si l'utilisateur a accepté les statistiques d'usage (traitement 1), par cohérence avec son choix. |
| Données | Identifiant du plugin, version, événement (installation ou mise à jour), jour, nombre. Aucun identifiant d'installation ni de session. L'adresse IP de la requête n'est vue que par le proxy (traitement 2). |
| Destinataires | Publics, agrégés par jour et par version. |
| Durée de conservation | Sans limite (des compteurs). |

## Le site

escouade.dev ne dépose aucun cookie et n'a aucun outil de mesure d'audience. Le choix de langue est gardé dans le stockage local du navigateur (clé `escouade-lang`), à la demande du visiteur : exempté de consentement.

store.escouade.dev dépose un seul cookie, celui de la session d'un auteur connecté, strictement nécessaire : exempté de consentement. Aucune mesure d'audience. Les avatars des auteurs sont leurs initiales, dessinées par le store, et les images des README seront copiées sur le store à la soumission : un visiteur ne charge rien chez un tiers. Le traitement est décrit aux visiteurs dans la section « Le store » de la politique de confidentialité d'escouade.dev, et les conditions des auteurs sont sur store.escouade.dev/terms.
