# La clé de fraîcheur de la liste de révocation

La clé publique (`freshness.pub`) de la clé de fraîcheur (audit SEC-15) : chaque lundi, le workflow `.github/workflows/revocations.yml` signe avec elle « la liste nº N, d'empreinte X, vaut encore au &lt;date&gt; » (`store/scripts/attest-revocations.ts`), servi à côté de la copie de la liste sur escouade.dev (`revocations-fresh.json`). L'app la compare à `FRESHNESS_KEYS` (`crates/escouade-plugins/src/store_keys.rs`, qu'un test compare à ce dossier) et avertit quand la liste qu'elle tient a plus de 7 jours.

**Elle ne signe jamais une liste** : volée, elle ne peut que prolonger une liste signée par l'utilisateur, jamais retirer ni rétablir un plugin. Elle n'est donc pas dans `store/keys/`, dont les clés signent des sceaux ou des listes.

**La clé secrète n'est jamais ici** : l'utilisateur la crée sur son poste par `cd store && npx tsx scripts/create-freshness-key.ts` (`~/.escouade-signing/freshness.key`, jamais affichée), la donne au workflow par `gh secret set REVOCATIONS_FRESHNESS_KEY < ~/.escouade-signing/freshness.key`, puis commite `freshness.pub` et sa ligne dans `FRESHNESS_KEYS`. Tant que ce dossier n'a pas de `.pub`, le workflow republie la liste telle qu'elle a été signée et le dit dans son résumé, et l'app prend une liste sans attestation, comme avant.
