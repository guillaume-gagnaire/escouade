# The store, as Coolify deploys it on store.escouade.dev: built and tested, then served by Nitro on port 3000.
# Built from the root of the repo (the store reads the site's tokens in website/app/assets/main.css, and later the
# manifest's schema in schemas/): `docker build -f store/Dockerfile .`. What the build sees is in
# Dockerfile.dockerignore.
#
# Coolify hands the application's variables to the build as well (it cannot be told not to): nothing here declares
# or needs one. The tests run without the database's variables, so that they never reach the store's base; and the
# final stage runs no command, so that no variable lands in the image's history.

FROM node:24-alpine AS build
WORKDIR /repo
COPY . .
WORKDIR /repo/store
# A failing test fails the deployment: the store online stays the last one that passed.
RUN npm ci && npm run build && env -u DATABASE_URL -u TEST_DATABASE_URL npm test && mkdir -p /out/archives

FROM node:24-alpine
WORKDIR /app
ENV NODE_ENV=production HOST=0.0.0.0 PORT=3000 ARCHIVE_DIR=/data/archives MIGRATIONS_DIR=/app/migrations
COPY --from=build /repo/store/.output ./.output
# The migrations are played at start, before /api/health says ok (docs/STORE.md, §1.3).
COPY --from=build /repo/store/server/db/migrations ./migrations
# The archives' volume (Coolify mounts it here and backs it up every night): the server writes in it.
COPY --from=build --chown=node:node /out/archives /data/archives
USER node
VOLUME /data/archives
EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=5s --start-period=60s CMD wget -qO /dev/null http://127.0.0.1:3000/api/health || exit 1
CMD ["node", ".output/server/index.mjs"]
