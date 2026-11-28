# Deploying escouade-server

Escouade's server mode: its core without a window, on a machine with no desktop, driven from Escouade on your computer.
The user's guide is the site's page, [Server mode](https://escouade.dev/docs/server/) (in French at `/docs/server/`).

| File | What it is |
|---|---|
| `Dockerfile` | The image `ghcr.io/guillaume-gagnaire/escouade/escouade-server` (linux/amd64, linux/arm64), built from the archives of a release |
| `docker-compose.yml` | An example: the port on `127.0.0.1` of the host alone, read-only, no capability, the advised limits |
| `coolify/docker-compose.yml` | The same as Coolify takes it; a domain behind its proxy, in TLS, commented out |
| `install.sh` | systemd without Docker: the archive checked (SHA-256, signature), a service of the user or of the system, idempotent, `--uninstall [--purge]` |

What none of them does: publish the API without TLS on the Internet. By default the API listens on `127.0.0.1` (of the
container, in Docker); the examples publish it on `127.0.0.1` of the host, for an SSH tunnel:

```sh
ssh -N -L 46480:127.0.0.1:46480 <the server>
```

## Building the image here

From the archive of `scripts/server-package.sh` (or a release's), in a folder of its own. The program carries the page of
the browser's client, built before it (`server-package.sh` refuses a program without it):

```sh
npm ci && npm run build:web
(cd src-tauri && ESCOUADE_WEB_DIST=dist-web cargo build --release --package escouade --bin escouade-server --no-default-features)
sh scripts/server-package.sh src-tauri/target/release/escouade-server 2.3.0 x64 dist <seed folder>
docker build -f deploy/Dockerfile --build-arg VERSION=2.3.0 -t escouade-server:2.3.0 dist
sh scripts/server-image-test.sh escouade-server:2.3.0 --harness
```

`scripts/server-release.sh` is the Release's part for one architecture (archive, signature checked, image pushed by
its digest), and runs the same way as a dry run with a key of your own and without `PUSH=1`.
`scripts/server-install-test.sh` tries `install.sh` on a machine with systemd (the CI's runner, or a container of
`debian:trixie` with systemd).

The browser's client is off in the examples: `ESCOUADE_SERVER_BROWSER=1` (or `escouade-server --browser`) serves the
interface on the API's port, through the SSH tunnel (`http://127.0.0.1:46480`) or behind a TLS proxy named by
`ESCOUADE_SERVER_ALLOW_HOSTS` (`https://<name>` alone).
