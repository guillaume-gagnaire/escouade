// @vitest-environment node
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { check, updaterKey } from './minisign-verify.mjs';

// ESC-515: how escouade-server is distributed. The image and its examples open nothing by default and never the API
// without TLS on the Internet; the Release publishes the server's archives signed with the updater's key and its image
// with an attestation, the image tagged only once the release is published; the key that checks the archives is the
// updater's everywhere. The image itself and install.sh are tried for real by scripts/server-image-test.sh and
// scripts/server-install-test.sh (the CI's « Server (Linux) »).
const ROOT = path.join(path.dirname(fileURLToPath(import.meta.url)), '..');
const read = (p: string) => fs.readFileSync(path.join(ROOT, p), 'utf8');
const IMAGE = 'ghcr.io/guillaume-gagnaire/escouade/escouade-server';

/** The text of a job of a workflow: from its line to the next job's. */
function job(file: string, name: string): string {
  const text = read(file);
  const start = text.indexOf(`\n  ${name}:\n`);
  expect(start, `${file} has a job ${name}`).toBeGreaterThan(-1);
  const rest = text.slice(start + 1);
  const next = rest.slice(1).search(/\n {2}[A-Za-z0-9_-]+:\n/);
  return next === -1 ? rest : rest.slice(0, next + 1);
}

describe('the Release of escouade-server', () => {
  const release = '.github/workflows/release.yml';

  it('builds, signs and uploads the archives of both architectures, and the release waits for them', () => {
    const server = job(release, 'server');
    expect(server).toContain("if: github.repository == 'guillaume-gagnaire/escouade'");
    expect(server).toContain('runner: ubuntu-22.04-arm');
    expect(server).toContain('runner: ubuntu-22.04');
    expect(server).toContain('--no-default-features');
    expect(server).toContain('sh scripts/server-release.sh');
    expect(server).toContain('TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}');
    expect(server).toContain("PUSH: '1'");
    for (const f of ['*.tar.gz', '*.tar.gz.sha256', '*.tar.gz.sig', 'install.sh']) expect(server).toContain(`dist/${f}`);
    expect(job(release, 'publish')).toContain('needs: [create, build, server]');
  });

  // ESC-586: the team server is published as escouade-server is, by the same job and the same scripts.
  it("builds the browser's page before the server that embeds it, and packs no server without it (BC-1)", () => {
    const server = job(release, 'server');
    const page = server.indexOf('npm run build:web');
    expect(page).toBeGreaterThan(-1);
    expect(page).toBeLessThan(server.indexOf('--bin escouade-server --no-default-features'));
    expect(server).toMatch(/ESCOUADE_WEB_DIST: dist-web\n\s+run: cargo build --release --locked -p escouade --bin escouade-server/);
    const pack = read('scripts/server-package.sh');
    expect(pack).toContain('grep -aq "escouade-web-page:"');
    // The CI's archives of a debug build say they carry none.
    expect(read('.github/workflows/ci.yml')).toContain(
      'ESCOUADE_NO_WEB_PAGE=1 sh scripts/server-package.sh "$RUNNER_TEMP/escouade-server"',
    );
  });

  it('builds, tries in a bare container, signs and pushes the team server too, by the same script', () => {
    const server = job(release, 'server');
    expect(server).toContain('cargo build --release --locked -p escouade-team-server');
    // The trial before anything is signed or goes up.
    const trial = server.indexOf('bash /srv/scripts/team-server-trial.sh /srv/bin/escouade-team-server "${TAG#v}"');
    expect(trial).toBeGreaterThan(-1);
    expect(trial).toBeLessThan(server.indexOf('sh scripts/server-release.sh'));
    expect(server).toContain(
      'IMAGE=$REGISTRY/escouade-team-server sh scripts/server-release.sh src-tauri/target/release/escouade-team-server',
    );
    expect(server).toContain('IMAGE=$REGISTRY/escouade-server sh scripts/server-release.sh src-tauri/target/release/escouade-server');
    expect(server).toContain('path: dist/*-${{ matrix.arch }}.digest');
    // One job of images per program, each with its digests.
    const image = job(release, 'server-image');
    expect(image).toContain('program: [escouade-server, escouade-team-server]');
    expect(image).toContain('IMAGE: ghcr.io/${{ github.repository }}/${{ matrix.program }}');
    expect(image).toContain('digests/$PROGRAM-x64.digest');
    expect(image).toContain('digests/$PROGRAM-arm64.digest');
    // The team server's version is the tag's (scripts/version.mjs sets it).
    expect(read('scripts/version.mjs')).toContain("'escouade-team-server', 'Cargo.toml'");
  });

  it('the team server’s image builds from its archive, and its compose and Coolify examples pull the published image', () => {
    const file = read('crates/escouade-team-server/Dockerfile');
    expect(file).toMatch(/^FROM \$\{PROGRAM\} AS program$/m);
    expect(file).toContain('escouade-team-server-${VERSION}-linux-${arch}.tar.gz');
    expect(file).toMatch(/^USER escouade-team$/m);
    expect(read('scripts/server-release.sh')).toContain('--build-arg PROGRAM=archive');
    for (const compose of ['crates/escouade-team-server/deploy/docker-compose.yml', 'crates/escouade-team-server/deploy/coolify.yml']) {
      const active = read(compose)
        .split('\n')
        .filter((l) => !l.trim().startsWith('#'))
        .join('\n');
      expect(active, compose).toContain('image: ghcr.io/guillaume-gagnaire/escouade/escouade-team-server:${ESCOUADE_TEAM_TAG:-latest}');
      // Never the team server's own port on the host: a proxy in TLS reaches it.
      expect(active, compose).not.toMatch(/- ["']?\d+:8443/);
    }
  });

  it('tags the image only once the release is published, `latest` never for a beta, and attests it', () => {
    const image = job(release, 'server-image');
    expect(image).toContain('needs: [publish, server]');
    expect(image).toContain("if: github.repository == 'guillaume-gagnaire/escouade'");
    expect(image).toContain('docker buildx imagetools create');
    expect(image).toContain('node scripts/image-tags.mjs "$IMAGE" "$TAG"');
    expect(image).toContain('actions/attest-build-provenance@');
    expect(image).toContain('push-to-registry: true');
    expect(image).toContain('gh attestation verify "oci://');
    expect(image).not.toContain('TAURI_SIGNING');
    // Nothing else tags an image: the architectures' jobs push by digest alone.
    const others = read(release).replace(image, '');
    expect(others).not.toContain('imagetools create');
    const script = read('scripts/server-release.sh');
    expect(script).toContain('push-by-digest=true');
    expect(script).not.toMatch(/-t "\$IMAGE/);
  });

  it('signs with the updater’s key and checks the signature before anything goes up', () => {
    const script = read('scripts/server-release.sh');
    expect(script.indexOf('signer sign')).toBeLessThan(script.indexOf('minisign-verify.mjs'));
    expect(script.indexOf('minisign-verify.mjs')).toBeLessThan(script.indexOf('buildx build'));
  });

  it('checks an archive with the updater’s key everywhere: install.sh, escouade-server update, the Release', () => {
    const key = updaterKey();
    expect(read('deploy/install.sh')).toContain(`UPDATER_PUBKEY=${key}`);
    expect(read('src-tauri/src/server/update.rs')).toContain(`"${key}"`);
  });

  it('a signature holds for its file and its key alone', () => {
    const dir = 'tests/fixtures/server-update';
    const archive = fs.readFileSync(path.join(ROOT, dir, 'escouade-server-9.9.9-linux-x64.tar.gz'));
    const sig = read(`${dir}/escouade-server-9.9.9-linux-x64.tar.gz.sig`);
    const key = read(`${dir}/test-key.pub`);
    expect(() => check(archive, sig, key)).not.toThrow();
    const changed = Buffer.from(archive);
    changed[changed.length - 1] ^= 1;
    expect(() => check(changed, sig, key)).toThrow(/does not hold/);
    expect(() => check(archive, sig, updaterKey())).toThrow(/another key/);
  });
});

describe('the image and its examples open nothing by default', () => {
  it('the image runs as its own user, keeps its data in /data, has its HEALTHCHECK and exposes no port', () => {
    const file = read('deploy/Dockerfile');
    expect(file).toMatch(/^USER escouade$/m);
    expect(file).toMatch(/^VOLUME \/data$/m);
    expect(file).toMatch(/^HEALTHCHECK .*escouade-server", "health"/m);
    expect(file).toMatch(/^ENTRYPOINT \["\/usr\/bin\/tini", "--"\]$/m);
    expect(file).toContain('ESCOUADE_SERVER_IMAGE=1');
    expect(file).not.toMatch(/^EXPOSE/m);
    const instructions = file
      .split('\n')
      .filter((l) => !l.trim().startsWith('#'))
      .join('\n');
    expect(instructions).not.toMatch(/sudo/);
    // The browser's client off in the image unless ESCOUADE_SERVER_BROWSER=1 is given (BC-1).
    expect(instructions).not.toContain('ESCOUADE_SERVER_BROWSER');
    expect(file).toContain('ESCOUADE_SERVER_BROWSER=1');
    // Without ESCOUADE_SERVER_LISTEN, the API listens on 127.0.0.1 of the container alone.
    expect(file).not.toMatch(/ENV[^\n]*ESCOUADE_SERVER_LISTEN/);
    // SR1-12: every base pinned by the digest of its index, and what 0.0.0.0 exposes said.
    const bases = [...file.matchAll(/^FROM\s+(\S+)/gm)].map((m) => m[1]);
    expect(bases.length).toBeGreaterThanOrEqual(2);
    for (const base of bases) expect(base, base).toMatch(/^docker\.io\/library\/[a-z]+:[\w.-]+@sha256:[0-9a-f]{64}$/);
    expect(file).not.toMatch(/^ARG\s+\w*IMAGE/m);
    expect(file).toMatch(/in\s+(#\s+)?clear/);
    expect(read('.github/dependabot.yml')).toMatch(/package-ecosystem: docker\n\s+directory: \/deploy/);
  });

  for (const compose of ['deploy/docker-compose.yml', 'deploy/coolify/docker-compose.yml']) {
    it(`${compose}: the port on 127.0.0.1 of the host alone, read-only, no capability`, () => {
      const text = read(compose);
      const active = text
        .split('\n')
        .filter((l) => !l.trim().startsWith('#'))
        .join('\n');
      // On a minor version, never `latest` (SR1-12); the tag the Release puts on a stable version (image-tags.mjs).
      expect(active).toMatch(new RegExp(`image: ${IMAGE.replace(/[.]/g, '\\.')}:\\d+\\.\\d+\\n`));
      expect(active).not.toContain(':latest');
      // 0.0.0.0 said: in clear on the container's network.
      expect(text).toMatch(/ESCOUADE_SERVER_LISTEN[^\n]*0\.0\.0\.0/);
      expect(text).toMatch(/in clear/);
      const ports = [...active.matchAll(/^\s+- ["']?([^"'\s]+)["']?$/gm)].map((m) => m[1]).filter((p) => /^[\d.]*:?\d+:\d+$/.test(p));
      expect(ports).toEqual(['127.0.0.1:46480:46480']);
      expect(active).toContain('read_only: true');
      expect(active).toMatch(/cap_drop:\n\s+- ALL/);
      expect(active).toContain('no-new-privileges:true');
      expect(active).toMatch(/mem_limit: \d+[mg]/);
      expect(active).toMatch(/pids_limit: \d+/);
      // A domain only through Coolify's proxy, which holds TLS; nothing else listens on the Internet.
      expect(active).not.toContain('SERVICE_FQDN');
      // The browser's client (BC-1) left off, and said so in a comment.
      expect(active).not.toContain('ESCOUADE_SERVER_BROWSER');
      expect(text).toMatch(/#\s*-?\s*ESCOUADE_SERVER_BROWSER/);
      expect(text).toMatch(/browser's client stays off/);
    });
  }

  it('the image’s name is the one the server tells a client to pull', () => {
    expect(read('src-tauri/src/remoteapi/proto.rs')).toContain(`pub const SERVER_IMAGE: &str = "${IMAGE}";`);
  });

  it('no example of the docs publishes the API but on 127.0.0.1', () => {
    const docs = [
      'README.md',
      'README.en.md',
      'website/content/docs/fr/server.md',
      'website/content/docs/en/server.md',
      'deploy/README.md',
    ].filter((p) => fs.existsSync(path.join(ROOT, p)));
    expect(docs.length).toBeGreaterThanOrEqual(4);
    for (const p of docs) {
      const text = read(p);
      for (const m of text.matchAll(/(?:-p|--publish)\s+(\S+)/g)) expect(m[1], `${p}: ${m[0]}`).toMatch(/^127\.0\.0\.1:/);
      for (const m of text.matchAll(/["'](\d+:\d+(?::\d+)?)["']/g)) expect(m[1], `${p}: ${m[0]}`).toMatch(/^127\.0\.0\.1:/);
      expect(text, p).not.toMatch(/ws:\/\/(?!127\.0\.0\.1|localhost)[a-z]/);
    }
  });
});

// The scripts of the server mode are run by `sh`, which is bash under macOS and Git Bash, and dash under Debian: each
// shell must read them. An apostrophe in a `"${VAR:?message}"` passes dash and bash in its POSIX mode, and breaks the
// whole script for bash (ESC-515's test return).
describe('the scripts of the server mode', () => {
  const SCRIPTS = [
    'deploy/install.sh',
    'scripts/server-package.sh',
    'scripts/server-release.sh',
    'scripts/server-image-test.sh',
    'scripts/server-install-test.sh',
    'scripts/server-trial.sh',
    'scripts/server-libs.sh',
    'scripts/team-server-trial.sh',
    'scripts/team-server-image-test.sh',
  ];
  const has = (shell: string) => spawnSync(shell, ['-c', 'exit 0']).status === 0;

  // One shell started per script: over a second each on a loaded Windows (Git Bash), hence the time given.
  for (const shell of ['bash', 'dash']) {
    it.skipIf(!has(shell))(
      `${shell} reads each of them`,
      () => {
        for (const s of SCRIPTS) {
          const r = spawnSync(shell, ['-n', s], { cwd: ROOT, encoding: 'utf8' });
          expect(r.status, `${shell} -n ${s}: ${r.stderr}`).toBe(0);
        }
      },
      60_000,
    );
  }

  it('none puts an apostrophe in the message of a parameter’s expansion', () => {
    for (const s of SCRIPTS) {
      expect(read(s), s).not.toMatch(/\$\{[A-Za-z_0-9]+:[?=+-][^}]*'/);
    }
  });
});
