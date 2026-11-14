# escouade-policy : les stratégies du poste géré (TS-0b)

Ticket ESC-454, ligne TS-0b de `docs/TEAM-SERVER.md` §16 (conception au §6.3). La crate n'est branchée nulle part. TS-11 (le moteur des couches et le verrou) et TS-12 (la liste unique, l'ADMX, le profil MDM, la relecture à chaud) viendront après.

## Ce qu'elle lit

| Système | Où | Source | Règle |
| --- | --- | --- | --- |
| Windows | `HKLM\SOFTWARE\Policies\Escouade`, puis `HKCU\SOFTWARE\Policies\Escouade` (vue 64 bits) | `machine`, `user` | La machine gagne, clé par clé. Une clé que ce processus peut écrire sans être élevé est refusée et dite : son ACL a été relâchée. |
| macOS | domaine `dev.gagnaire.escouade`, par `CFPreferencesCopyAppValue` | `profile` | Seul ce que `CFPreferencesAppValueIsForced` dit forcé compte. Le reste (un `defaults write`, les préférences de l'app elle-même) est ignoré sans rien dire. |
| Linux | `/etc/escouade/policies/*.json` | `file` (son chemin) | Les fichiers se lisent par ordre de nom, et le dernier gagne clé par clé. Le dossier, chaque dossier au-dessus et chaque fichier doivent être à root, sans écriture pour le groupe ni pour les autres. Un lien symbolique est refusé. Un fichier qui ne respecte pas ces règles est refusé et dit. Pour un dossier, rien n'est lu. |

**Le modèle commun.** Une `Policy` porte une clé (`permissions.allowBypass`), une valeur JSON et une `Source`. `Policies` en garde une par clé, avec ses `Warning` :
- `Malformed` : la valeur est ignorée.
- `UnknownKey` : la valeur est gardée.
- `Refused` : la clé, le dossier ou le fichier n'est pas digne de confiance.
- `Unreadable` : la clé, le dossier ou le fichier n'a pas pu être lu.

Rien ne fait planter la lecture : `escouade_policy::read()` rend toujours un résultat. Les clés connues sont provisoires (`PROVISIONAL_KEYS`, tirées du §6.4) : la liste unique de TS-12 les remplacera.

**Les valeurs.**
- **Windows** :
  - `REG_DWORD`, `REG_QWORD` et `REG_DWORD_BIG_ENDIAN` donnent un nombre. Un booléen ADMX est un `REG_DWORD` à 0 ou 1 : c'est la liste de TS-12 qui dira quelles clés sont booléennes.
  - `REG_SZ` donne une chaîne. S'il commence par `{` ou `[`, il est lu comme du JSON : c'est la forme d'un réglage complexe, et un JSON cassé est ignoré et dit.
  - `REG_EXPAND_SZ` donne la chaîne telle qu'écrite, sans développer les `%VARIABLE%`.
  - `REG_MULTI_SZ` donne une liste de chaînes.
  - Une sous-clé à valeurs `1`, `2`, `10`… donne une liste, dans l'ordre des numéros : c'est une liste ADMX. Un nom qui n'est pas un numéro est ignoré et dit, comme une sous-clé plus profonde.
  - Les autres types (`REG_BINARY`, `REG_NONE`, `REG_LINK`…) sont ignorés et dits.
- **macOS** :
  - Booléens, nombres, chaînes, tableaux et dictionnaires passent tels quels. Une chaîne de premier niveau qui commence par `{` ou `[` est lue comme du JSON.
  - Les données et les dates sont ignorées et dites.
- **Linux** : chaque fichier est un objet JSON qui associe des clés à des valeurs. Un fichier qui n'est pas un objet, ou qui n'est pas du JSON valide, est ignoré et dit, et les autres fichiers sont lus quand même. Une valeur `null` est ignorée et dite. Un fichier de plus de 1 Mio est refusé.

**Les sources injectables, pour les tests sans droits d'administrateur.**
- **Windows** : `windows::from_nodes` lit des clés déjà en mémoire (`Node`). `windows::read_registry(machine, user, admin_only, known)` prend deux emplacements quelconques du registre.
- **macOS** : `macos::read_preferences` prend tout lecteur qui implémente `Preferences`. `macos::read_domain` prend n'importe quel domaine.
- **Linux** : `files::from_contents` lit des contenus déjà chargés. `files::read_folder(Folder { dir, owner, checked_up_to })` prend un dossier, le propriétaire attendu et la limite de la vérification des dossiers parents.

## Ce qui a été vérifié (le 2026-11-14)

**Windows 11 Pro 26200, compte non élevé : 21 tests (nextest), tous verts.**
- **Test réel** : une clé de test sous `HKCU\Software\EscouadePolicyTest-<pid>-…`, hors de `Policies`, est créée puis effacée par le test. Elle a été lue avec ses `REG_DWORD`, `REG_SZ`, son JSON, son `REG_MULTI_SZ` et une liste ADMX `1`, `2`, `10`. Un JSON cassé et un `REG_BINARY` ont été ignorés et dits. La machine est passée devant l'utilisateur.
- **Une clé relâchée est refusée** : la même clé de test, que l'utilisateur peut écrire, est refusée avec `admin_only`. C'est ce que deviendrait une clé `Policies` à l'ACL relâchée.
- **Les ACL**, lues par `Get-Acl` :
  - `HKLM\SOFTWARE\Policies` : Administrateurs et Système en contrôle total ; Utilisateurs authentifiés et paquets d'application en lecture seule.
  - `HKCU\SOFTWARE\Policies` : propriétaire Système ; Administrateurs et Système en contrôle total ; l'utilisateur lui-même en lecture seule (`ReadKey`), même dans sa propre ruche.
  - Un test le confirme : ce compte ne peut ouvrir aucune des deux clés en écriture (`KEY_SET_VALUE` refusé).
- `cargo run -p escouade-policy --example read` sur ce poste, sans stratégie : rien, sans avertissement.

**Linux (WSL 2, Ubuntu, ext4) : 24 tests, tous verts, dont 7 sur le vrai système de fichiers.**
- Les tests ont refusé :
  - un fichier en 666, un fichier en 664 et un lien symbolique ;
  - un fichier qui n'est pas à root ;
  - un dossier ouvert à tous, et un dossier sous un parent ouvert à tous ;
  - les fichiers et les valeurs mal formés.
- Ils ont vérifié que `/etc` est tenu pour sûr.
- **Essai réel en root** : un vrai `/etc/escouade/policies`, posé par `wsl -u root`, puis effacé, avec :
  - `10-base.json` et `20-team.json` (644, root) : lus, le second gagnant sur `telemetry.enabled` ;
  - `30-loose.json` (666) : refusé ;
  - `40-user.json` (à l'uid 1000) : refusé ;
  - `50-broken.json` : dit illisible.

  Le dossier passé en 775 n'a plus rien donné, avec un seul avertissement.

**macOS : compilé, pas exécuté.** `cargo clippy -p escouade-policy --all-targets --target x86_64-apple-darwin -D warnings` passe : le code CoreFoundation et son test sont typés et sans avertissement. Les tests sans CoreFoundation (le lecteur `Preferences` simulé) tournent sur Windows et Linux. Le test réel (`a_value_of_defaults_write_is_read_but_not_forced_so_no_policy`) écrit un domaine de test par `defaults write`. Il vérifie que CFPreferences voit bien les valeurs mais ne les dit pas forcées, donc qu'aucune n'est une stratégie, puis il efface le domaine. **Il n'a pas encore tourné sur un Mac** : la CI macOS de `release/2.3` le lancera (nextest de la crate).

## Ce qui n'a pas été vérifié, à faire dans les essais réels de TS-12

- **Un vrai GPO.** Une stratégie posée par `gpedit.msc` (ou par Intune) depuis l'ADMX de TS-12, dans `HKLM` puis dans `HKCU`. Il faut vérifier :
  - qu'elle arrive en `machine` et en `user`, et que la machine gagne ;
  - qu'une liste ADMX (`<list>` avec `explicitValue` absent) donne bien les valeurs `1`, `2`… ;
  - qu'un `<enum>` et un `<boolean>` donnent les `REG_DWORD` attendus.

  Il faut aussi relire l'ACL de `Policies\Escouade` telle que la crée la stratégie de groupe. Il faut un Windows Pro avec un compte administrateur, donc une élévation UAC.
- **Le cas `admin_only` quand l'app tourne élevée.** Escouade ne sait plus rien distinguer : tout est accepté. À dire dans la page des services informatiques.
- **Un vrai profil MDM installé.** Il faut un `.mobileconfig` du domaine `dev.gagnaire.escouade`, installé à la main dans Réglages Système > Profils, puis par un MDM si possible. Il faut vérifier :
  - que `CFPreferencesAppValueIsForced` dit vrai pour ses clés et faux pour un `defaults write` du même domaine ;
  - que la valeur forcée gagne sur le `defaults write` ;
  - que `CFPreferencesCopyKeyList` liste, ou non, les clés du profil. La crate lit aussi les clés des plists de `/Library/Managed Preferences`, au cas où.
  - que les tableaux et les dictionnaires du profil arrivent tels quels ;
  - le profil par utilisateur, dans `/Library/Managed Preferences/<user>/`.
- **Le premier lancement du test réel macOS**, en CI ou sur un Mac.
- **La relecture à chaud**, que la crate ne fait pas : surveiller le registre (`RegNotifyChangeKeyValue`), `/Library/Managed Preferences` et `/etc/escouade`.
- **La liste unique et les types.** Dire quelles clés sont des booléens, des listes ou des bornes. Convertir un `REG_DWORD` 0/1 en booléen. Remplacer `PROVISIONAL_KEYS`. Générer l'ADMX, les ADML, le manifeste et le schéma JSON.
- **Linux, au-delà d'Ubuntu.** Les ACL POSIX (`setfacl`) ne sont pas lues une à une. Mais quand un fichier a une ACL étendue, ses bits de groupe sont son masque, qui borne toute entrée nommée. Une écriture donnée à un autre utilisateur met donc `w` dans le masque, et le fichier est refusé comme ouvert au groupe. C'est la règle de POSIX.1e : elle n'a pas été essayée, car `setfacl` manque dans la distribution WSL. Un dossier `/etc` en lecture seule, comme sur NixOS ou dans un conteneur, n'a pas été essayé.

## Lancer

```sh
cd src-tauri
cargo nextest run -p escouade-policy
cargo run -p escouade-policy --example read   # les stratégies de ce poste, en JSON
```
