# syntax=docker/dockerfile:1
# The image of escouade-server (https://escouade.dev/docs/server/): Escouade's core without a window, driven from afar.
#
# Built from the archives of a release (the Release workflow), or of `scripts/server-package.sh` for a test, put in the
# build context with the names the release gives them:
#
#   dist/escouade-server-<version>-linux-x64.tar.gz
#   dist/escouade-server-<version>-linux-arm64.tar.gz
#   docker buildx build --platform linux/amd64,linux/arm64 -f deploy/Dockerfile --build-arg VERSION=<version> dist/
#
# What it holds: Debian 13 (trixie) slim, the program and the seed of its plugins (/opt/escouade-server), git,
# openssh-client, curl, ca-certificates, tini (PID 1: it reaps the zombies the harnesses leave), Node.js and npm (the
# harnesses installed by `npm install -g`: Codex, Copilot CLI, Gemini CLI, OpenCode), a user without privileges
# (escouade, 10001: no root, no sudo). No harness, no account: they are installed and signed in from Escouade, into
# the volume.
#
# Everything it writes is in /data (the volume): HOME is /data, Escouade's data in /data/.escouade (its state, its
# secrets in a file readable by its owner alone, its log), the projects and their worktrees where you clone them
# (/data/projects by custom), the harnesses (~/.local/bin, ~/.npm-global) and their sign-ins (~/.claude…). It runs with
# a root filesystem read-only and /tmp in memory (`--read-only --tmpfs /tmp`).
#
# By default the API listens on 127.0.0.1 of the container alone: nothing is reachable from outside, even with a port
# published. ESCOUADE_SERVER_LISTEN=0.0.0.0 opens it to the container's network, for a port published on 127.0.0.1 of
# the host (an SSH tunnel) or a TLS proxy in front (deploy/docker-compose.yml, deploy/coolify/). The API is then in
# clear (plain HTTP and WebSocket) on that network: every other container on it (on Coolify, the other resources of the
# same network) reaches it and may try pairing codes and tokens. TLS is the proxy's: put one in front (Coolify's
# Traefik, Caddy) or reach it through an SSH tunnel, never a port published on a public address.
#
# The browser's client (BC-1) is off: ESCOUADE_SERVER_BROWSER=1 serves the interface on the same port, to a browser that
# reaches it through the SSH tunnel (http://127.0.0.1:<port>) or a TLS proxy declared by ESCOUADE_SERVER_ALLOW_HOSTS
# (https://<name>; http:// of a name is refused). It adds a page and the origin of that page, nothing else.
#
# An update is a new image (`docker compose pull && docker compose up -d`): `escouade-server update` refuses to run here.

# The bases pinned by their digest (the index of every architecture, SR1-12): the same bytes at each build, whatever
# the registry's tag points at later. Dependabot (docker, .github/dependabot.yml) proposes the new digest; the tag
# beside it says which one it is.
FROM docker.io/library/node:22-trixie-slim@sha256:f0960f8dc047f358e3fb428f36ccba21732194fa5f0cac849f401e63e62fb9e6 AS node

FROM docker.io/library/debian:trixie-slim@sha256:8aa5a7bf5a1955bddbe7d74fcdd0b91ab30e74362a8b73af672b0481b35b44a0
ARG TARGETARCH
ARG VERSION

LABEL org.opencontainers.image.title="escouade-server" \
      org.opencontainers.image.description="Escouade's core without a window: agents, worktrees, board and terminals, driven from Escouade on your computer" \
      org.opencontainers.image.source="https://github.com/guillaume-gagnaire/escouade" \
      org.opencontainers.image.url="https://escouade.dev/docs/server/" \
      org.opencontainers.image.licenses="GPL-3.0-or-later" \
      org.opencontainers.image.version="${VERSION}"

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl git openssh-client tini procps less \
    && rm -rf /var/lib/apt/lists/*

# Node.js and npm, from the official image of the same Debian.
COPY --from=node /usr/local/bin/node /usr/local/bin/node
COPY --from=node /usr/local/lib/node_modules/npm /usr/local/lib/node_modules/npm
RUN ln -s ../lib/node_modules/npm/bin/npm-cli.js /usr/local/bin/npm \
    && ln -s ../lib/node_modules/npm/bin/npx-cli.js /usr/local/bin/npx \
    && node --version && npm --version

# The program and the seed of its plugins, from the archive of this architecture.
RUN --mount=type=bind,target=/dist \
    set -eu; \
    case "${TARGETARCH:-amd64}" in amd64) arch=x64 ;; arm64) arch=arm64 ;; *) echo "no escouade-server for ${TARGETARCH}" >&2; exit 1 ;; esac; \
    mkdir -p /opt/escouade-server; \
    tar -xzf "/dist/escouade-server-${VERSION}-linux-${arch}.tar.gz" -C /opt/escouade-server --strip-components=1 --no-same-owner; \
    chmod -R a+rX,go-w /opt/escouade-server; \
    ln -s /opt/escouade-server/escouade-server /usr/local/bin/escouade-server; \
    escouade-server --version

RUN useradd --uid 10001 --user-group --home-dir /data --no-create-home --shell /bin/bash escouade \
    && install -d -o escouade -g escouade -m 0700 /data

ENV HOME=/data \
    ESCOUADE_SERVER_IMAGE=1 \
    ESCOUADE_SERVER_DATA=/data/.escouade \
    NPM_CONFIG_PREFIX=/data/.npm-global \
    XDG_CONFIG_HOME=/data/.config \
    XDG_CACHE_HOME=/data/.cache \
    XDG_DATA_HOME=/data/.local/share \
    XDG_STATE_HOME=/data/.local/state \
    PATH=/data/.local/bin:/data/.npm-global/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
    SHELL=/bin/bash \
    LANG=C.UTF-8

USER escouade
WORKDIR /data
VOLUME /data
STOPSIGNAL SIGTERM
# Fresh while the server writes its status (every 5 s) and its process lives; its start loads the plugins.
HEALTHCHECK --interval=30s --timeout=10s --start-period=120s --retries=3 CMD ["escouade-server", "health"]
ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["escouade-server", "run"]
