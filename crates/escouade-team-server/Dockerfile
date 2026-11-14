# The team server's image. From the repository's root:
#   docker build -f crates/escouade-team-server/Dockerfile -t escouade-team-server .
# Nothing is published yet: the image and the binaries come with the 2.3.

FROM docker.io/library/rust:1-bookworm AS build
WORKDIR /src
COPY . .
# src-tauri/rust-toolchain.toml pins the toolchain; only this crate and its dependencies are built.
RUN cd src-tauri && cargo build --release --locked -p escouade-team-server \
    && cp target/release/escouade-team-server /escouade-team-server

FROM docker.io/library/debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --home-dir /data --shell /usr/sbin/nologin escouade-team \
    && mkdir -p /data && chown escouade-team /data
COPY --from=build /escouade-team-server /usr/local/bin/escouade-team-server
USER escouade-team
VOLUME /data
ENV ESCOUADE_TEAM_DATA=/data \
    ESCOUADE_TEAM_LISTEN=0.0.0.0:8443
EXPOSE 8443
ENTRYPOINT ["/usr/local/bin/escouade-team-server"]
CMD ["serve"]
