# Les stratégies du poste géré

Ce qu'un service informatique déploie pour imposer des réglages d'Escouade (docs/TEAM-SERVER.md §6.3). Tout ce dossier, sauf ce fichier, est **généré** depuis la liste unique des stratégies, `crates/escouade-policy/src/list.rs` : ne le modifie pas à la main.

```sh
cd src-tauri && cargo run -p escouade-policy --example generate
```

Le test `generate::tests::the_files_of_the_repository_are_what_the_list_generates` échoue tant que ces fichiers ne sont pas ce que la liste génère.

| Fichier | Pour |
| --- | --- |
| `windows/escouade.admx`, `windows/fr-FR/escouade.adml`, `windows/en-US/escouade.adml` | Stratégies de groupe (`C:\Windows\PolicyDefinitions` ou le magasin central), Intune (ADMX importés) |
| `macos/dev.gagnaire.escouade.plist` | Le manifeste des clés, format ProfileManifests (ProfileCreator, iMazing Profile Editor) |
| `macos/dev.gagnaire.escouade.json` | Le schéma personnalisé de Jamf Pro |
| `macos/escouade-example.mobileconfig` | Un profil d'exemple, chaque clé à sa valeur d'exemple |
| `linux/escouade-policies.schema.json`, `linux/example.json` | `/etc/escouade/policies/*.json` |

La page du site qui dit comment les déployer, `website/content/docs/{fr,en}/managed-policies.md`, est générée de la même liste.
