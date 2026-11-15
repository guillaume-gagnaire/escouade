# The site, as Coolify deploys it on escouade.dev: generated and tested, then served by nginx.
# Built from the root of the repo (the site reads the app's version in src-tauri/tauri.conf.json, and its tests
# the roadmap in docs/): `docker build -f website/Dockerfile .`. What the build sees is in Dockerfile.dockerignore.

FROM node:24-alpine AS build
WORKDIR /repo
COPY . .
WORKDIR /repo/website
# A failing test fails the deployment: the site online stays the last one that passed.
RUN npm ci && npm run generate && npm test

FROM nginx:1.29-alpine
COPY website/nginx.conf /etc/nginx/conf.d/default.conf
COPY website/security-headers.conf /etc/nginx/snippets/security-headers.conf
COPY --from=build /repo/website/.output/public /usr/share/nginx/html
EXPOSE 80
