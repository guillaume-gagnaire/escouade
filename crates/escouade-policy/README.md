# escouade-policy : les stratégies du poste géré (TS-0b, TS-12)

Tickets ESC-454 (TS-0b : la lecture) et ESC-465 (TS-12 : la liste unique, les fichiers générés, la relecture à chaud), lignes de `docs/TEAM-SERVER.md` §16 (conception aux §6.3 et §6.4). Le cœur la lit comme rang 6 des couches de réglages (TS-11, ESC-455, `src-tauri/src/locks.rs`) : au démarrage, puis à chaque changement.

## La liste unique (TS-12)

`src/list.rs` est la seule liste des stratégies (`POLICIES`, 30 clés) : pour chacune son **type** (booléen, liste, chaîne, nombre, JSON), sa **sorte** (restreindre, valeur, sensible), sa **forme** (interrupteur, choix, plafond, borne, liste d'interdits, texte, JSON), les lignes des Réglages qu'elle verrouille (`SETTINGS_ROWS`), et ses textes en français et en anglais. Elle remplace `PROVISIONAL_KEYS`, et c'est la table du cœur (`locks::KEYS` est elle).

- **Les types** : `read()` lit puis convertit chaque clé de la liste (`list::convert`, `Policies::typed`). Un `REG_DWORD` 0 ou 1 devient un booléen, une chaîne seule une liste d'un élément, une chaîne JSON un objet ; ce qui ne va pas est retiré et dit (`Warning::Malformed`). Une clé hors de la liste reste telle quelle, dite inconnue.
- **Les fichiers générés** (`src/generate.rs`) : `policies/windows/escouade.admx` et ses ADML `fr-FR`, `en-US` ; `policies/macos/` : le manifeste ProfileManifests, le schéma de Jamf, un `.mobileconfig` d'exemple ; `policies/linux/` : le schéma JSON de `/etc/escouade/policies` et un exemple ; la page du site `website/content/docs/{fr,en}/managed-policies.md`. `cargo run -p escouade-policy --example generate` les écrit ; le test `the_files_of_the_repository_are_what_the_list_generates` échoue tant qu'ils divergent. D'autres tests relisent l'ADMX et les ADML (chaque chaîne et chaque présentation nommées existent), le profil et le manifeste (par la crate `plist`), et les schémas.

## La relecture à chaud (TS-12)

`src/watch.rs` : un fil attend un signe de changement (`Changes`), relit, et rappelle quand les stratégies diffèrent des dernières.

- **Windows** : `RegNotifyChangeKeyValue` sur `HKLM\SOFTWARE\Policies` et `HKCU\SOFTWARE\Policies`, sous-arbre compris (la clé `Escouade` peut ne pas exister encore), réarmé après chaque signe.
- **macOS** : les fichiers de `/Library/Managed Preferences` ; **Linux** : ceux de `/etc/escouade`. Noms, tailles et dates, regardés chaque seconde (`Poll`) : pas de dépendance, et un dossier qui apparaît plus tard est vu.
- Partout, tout est relu toutes les 30 s en plus (le cœur), au cas où un changement ne donnerait aucun signe.
- **Injectable** : `watch::spawn(lecture, signe, dernières, période, rappel)` ; les tests donnent un canal (`Channel`) et leur propre lecture. L'e2e passe par la source d'essai du cœur (`ESCOUADE_TEST_POLICIES`, build de développement seulement), surveillée par `Poll`.
- `cargo run -p escouade-policy --example read -- --watch` imprime les stratégies à chaque changement : c'est l'outil des essais réels.

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

Rien ne fait planter la lecture : `escouade_policy::read()` rend toujours un résultat. Les clés connues sont celles de la liste unique (`known_keys()`).

**Les valeurs.**
- **Windows** :
  - `REG_DWORD`, `REG_QWORD` et `REG_DWORD_BIG_ENDIAN` donnent un nombre. Un booléen ADMX est un `REG_DWORD` à 0 ou 1 : la liste dit quelles clés sont booléennes, et `read()` les convertit.
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

## Les essais de TS-12 (le 2026-11-15)

**Windows 11 Pro 26200, compte non élevé (niveau d'intégrité moyen).**
- **La stratégie locale (gpedit) n'a pas pu être essayée** : poser l'ADMX dans `C:\Windows\PolicyDefinitions` demande l'élévation UAC, que l'agent n'a pas. `scripts/windows-trial.ps1`, à lancer en administrateur, pose l'ADMX et ses ADML, écrit quelques stratégies dans `HKLM` et `HKCU` comme le ferait la stratégie de groupe, montre leur ACL et ce que lit la crate, puis dit quoi vérifier à la main dans gpedit (`-Cleanup` retire tout). **À faire par le chef avec l'utilisateur.**
- **Écrire `HKCU\Software\Policies\Escouade` avec les droits de ce compte : refusé** (« Accès refusé », par `New-Item` comme par `reg add`). C'est ce que TS-0b avait lu dans l'ACL : l'utilisateur n'a que la lecture sur sa propre clé `Policies`. Rien ne peut donc être posé là sans administrateur.
- **La relecture à chaud, sur le vrai registre** : `RegNotifyChangeKeyValue` sur une vraie clé de test de `HKCU\Software\` (hors de `Policies`) a dit une valeur écrite, une sous-clé de liste créée et un élément de liste ajouté, puis plus rien (`the_registry_tells_a_value_written_and_a_list_subkey_created`). Une stratégie écrite dans cette clé pendant que la surveillance tourne a été relue et convertie : le `REG_DWORD` 0 de `updates.automatic` est arrivé en `false` (`a_policy_written_in_the_registry_while_it_runs_is_read_again_and_typed`).
- **Dans l'app** : la source d'essai (build de développement) changée pendant que l'app tourne grise le réglage, puis le libère (`tests/e2e/policies-live.spec.ts`) ; un agent dont le harness devient interdit est arrêté entre deux tours (`locks::policies_tests`).
- Les fichiers générés sont relus par les tests : l'ADMX et ses ADML (XML bien formé, chaque chaîne et chaque présentation nommées existent, une liste écrite en sous-clé `<clé>` avec `valuePrefix=""`, donc des valeurs `1`, `2`…), le profil et le manifeste (par la crate `plist`), les schémas.

**Linux (WSL 2, Ubuntu) : 51 tests de la crate, tous verts, et un essai réel en root.** `read --watch` lancé, puis :
1. `/etc/escouade/policies/10-company.json` posé (644, root) avec `{"updates.automatic": 0, "permissions.modes": "plan"}` : vu en moins de 3 s, `false` et `["plan"]` ;
2. le même fichier changé : vu, `telemetry.enabled` à `false` ;
3. `20-loose.json` posé en 666 : vu, refusé et dit, les autres valeurs gardées ;
4. le dossier retiré : vu, plus aucune stratégie.

**macOS : rien n'a pu être essayé.** Pas de Mac ici. Restent :
- **un vrai profil MDM** : installer `policies/macos/escouade-example.mobileconfig` (réduit à quelques clés) à la main dans Réglages Système › Profils, puis par un MDM (Jamf, Intune, Kandji). Il faut vérifier que `CFPreferencesAppValueIsForced` dit vrai pour ses clés, que les tableaux et le dictionnaire de `harnesses.allowedModels` arrivent tels quels, et le profil par utilisateur dans `/Library/Managed Preferences/<user>/` ;
- **la relecture à chaud** : qu'un profil installé ou retiré change bien un fichier de `/Library/Managed Preferences` (ce que regarde `Poll`), et que `CFPreferencesAppSynchronize` rend la nouvelle valeur sans redémarrer ; sinon la relecture des 30 s ne suffirait pas non plus, à cause du cache de CFPreferences ;
- que Jamf Pro accepte `dev.gagnaire.escouade.json` comme schéma personnalisé, et ProfileCreator le manifeste ;
- le premier lancement du test réel macOS de TS-0b, en CI ou sur un Mac.

## Ce qui reste à essayer

- **Un vrai GPO** (`scripts/windows-trial.ps1`, en administrateur), dans `HKLM` puis dans `HKCU` : qu'une stratégie arrive en `machine` et en `user` et que la machine gagne ; qu'une liste ADMX donne bien `1`, `2`… ; qu'un `<enum>` et un booléen donnent les valeurs attendues ; l'ACL de `Policies\Escouade` telle que la crée la stratégie de groupe ; qu'un changement dans gpedit suivi de `gpupdate` est vu par `read --watch`. Puis l'import dans Intune (ADMX importés).
- **Le cas `admin_only` quand l'app tourne élevée** : Escouade ne sait plus rien distinguer et lit tout. La page du site le dit.
- **macOS** : ci-dessus.
- **Linux, au-delà d'Ubuntu** : les ACL POSIX (`setfacl`) et un `/etc` en lecture seule (NixOS, conteneur), comme le notait TS-0b.
- **Ce que la liste garde sans l'appliquer encore** : `projects.deniedFiles` (avec la clôture des fichiers d'un projet), `servers.personalSpace`, `servers.aggregateView`, `servers.contentlessNotifications` (avec les espaces et le serveur d'équipe), et l'écran d'entrée dans l'équipe qui lira l'enrôlement (TS-4).

## Lancer

```sh
cd src-tauri
cargo nextest run -p escouade-policy
cargo run -p escouade-policy --example read   # les stratégies de ce poste, en JSON
cargo run -p escouade-policy --example read -- --watch   # et à chaque changement
cargo run -p escouade-policy --example generate   # l'ADMX, les profils, les schémas et la page du site
```

L'essai en administrateur sous Windows : `powershell -ExecutionPolicy Bypass -File crates/escouade-policy/scripts/windows-trial.ps1` (puis `-Cleanup`).
