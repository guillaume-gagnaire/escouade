# Contributing

## What runs in a fork or a copy

The workflows of `.github/workflows/` do not all run outside `guillaume-gagnaire/escouade`:

- **Runs everywhere** (a fork, a copy, a pull request): `ci.yml`, the whole CI, on each push to `main` and on each pull request, and the `Advisories` checks on a push and a pull request. A contributor can run the CI on a public fork as is.
- **Only in `guillaume-gagnaire/escouade`**: the Release (a `v*` tag), the revocation list copy (every 15 minutes), the SDK's publication on npm, the Discord posts, and the nightly run of `Advisories`. A fork or a copy has not the secrets they need, and they would commit or publish where they should not. Their jobs check `github.repository`, and `scripts/workflows-origin.test.ts` fails if one loses it.
- **By hand only**: the macOS paste probe (`workflow_dispatch`).

A public repository's runners are free. **A private copy pays its minutes**, and a macOS minute counts ten times: the CI's macOS and Windows jobs are long. In a private copy, turn GitHub Actions off (Settings › Actions › General), or keep it to the pull requests you want checked.
