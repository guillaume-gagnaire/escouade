# syntax=docker/dockerfile:1
# The team server's image (https://escouade.dev/docs/team/install), two ways to get its program:
#
# - from the sources (by default), with the repository as the context:
#     docker build -f crates/escouade-team-server/Dockerfile -t escouade-team-server .
# - from the archive of a release (the Release workflow, scripts/server-release.sh), put in the context with the name
#   the release gives it (dist/escouade-team-server-<version>-linux-<x64|arm64>.tar.gz):
#     docker buildx build --platform linux/amd64,linux/arm64 -f crates/escouade-team-server/Dockerfile \
#       --build-arg PROGRAM=archive --build-arg VERSION=<version> dist/
#
# Published as ghcr.io/guillaume-gagnaire/escouade/escouade-team-server:<version> (and `latest` for a stable version
# alone, `beta` for a beta). Everything it writes is in /data (the volume): the base and the files.

ARG PROGRAM=source

FROM docker.io/library/rust:1-bookworm AS source
WORKDIR /src
COPY . .
# src-tauri/rust-toolchain.toml pins the toolchain; only this crate and its dependencies are built.
RUN cd src-tauri && cargo build --release --locked -p escouade-team-server \
    && install -D target/release/escouade-team-server /out/escouade-team-server

FROM docker.io/library/debian:bookworm-slim AS archive
ARG TARGETARCH
ARG VERSION
RUN --mount=type=bind,target=/dist \
    set -eu; \
    case "${TARGETARCH:-amd64}" in amd64) arch=x64 ;; arm64) arch=arm64 ;; *) echo "no escouade-team-server for ${TARGETARCH}" >&2; exit 1 ;; esac; \
    mkdir -p /out; \
    tar -xzf "/dist/escouade-team-server-${VERSION}-linux-${arch}.tar.gz" -C /out --strip-components=1 --no-same-owner \
      escouade-team-server/escouade-team-server

FROM ${PROGRAM} AS program

FROM docker.io/library/debian:bookworm-slim
ARG VERSION
LABEL org.opencontainers.image.title="escouade-team-server" \
      org.opencontainers.image.description="Escouade's team server: blind, it keeps only encrypted bytes and runs nothing" \
      org.opencontainers.image.source="https://github.com/guillaume-gagnaire/escouade" \
      org.opencontainers.image.url="https://escouade.dev/docs/team/install" \
      org.opencontainers.image.licenses="GPL-3.0-or-later" \
      org.opencontainers.image.version="${VERSION}"
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --home-dir /data --shell /usr/sbin/nologin escouade-team \
    && mkdir -p /data && chown escouade-team /data
COPY --from=program /out/escouade-team-server /usr/local/bin/escouade-team-server
RUN escouade-team-server --version
USER escouade-team
VOLUME /data
ENV ESCOUADE_TEAM_DATA=/data \
    ESCOUADE_TEAM_LISTEN=0.0.0.0:8443
EXPOSE 8443
STOPSIGNAL SIGTERM
ENTRYPOINT ["/usr/local/bin/escouade-team-server"]
CMD ["serve"]
