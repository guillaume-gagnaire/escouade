import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

// The two profiles of the store's container on Coolify (ESC-356, docs/STORE.md §3.3): Docker's own, plus what bubblewrap
// asks and nothing else. What the kernel does with them is proved by scripts/image-test.mjs; what is here keeps them
// narrow when one of them is edited.

const deploy = join(dirname(fileURLToPath(import.meta.url)), '..', '..', 'deploy');

interface Rule {
  names: string[];
  action: string;
  args?: { index: number; value: number; valueTwo?: number; op: string }[];
  includes?: { caps?: string[]; arches?: string[]; minKernel?: string };
  excludes?: { caps?: string[]; arches?: string[] };
  comment?: string;
  errnoRet?: number;
}

const seccomp = JSON.parse(readFileSync(join(deploy, 'seccomp-bubblewrap.json'), 'utf8')) as {
  defaultAction: string;
  syscalls: Rule[];
};
const apparmor = readFileSync(join(deploy, 'apparmor-bubblewrap'), 'utf8');

/** moby's default.json at docker-v29.9.3 (vendor/github.com/moby/profiles/seccomp v0.2.4), parsed and written back. */
const MOBY = '4dc1298afa7676147c1320afa42d72691fc55379ae762b392b56853574a7a38a';
const OURS = 4;
const ALL_NAMESPACES = 0x7e020000;
const NEWUSER = 0x10000000;
const NEWNET = 0x40000000;

/** Whether a process without capabilities may make this system call under the profile. */
const allowed = (name: string) =>
  seccomp.syscalls.some((r) => r.names.includes(name) && r.action === 'SCMP_ACT_ALLOW' && !r.includes?.caps);

describe('the seccomp profile', () => {
  const ours = seccomp.syscalls.slice(-OURS);

  it('is moby’s, whole, with four rules at its end', () => {
    const base = { ...seccomp, syscalls: seccomp.syscalls.slice(0, -OURS) };
    expect(createHash('sha256').update(JSON.stringify(base)).digest('hex')).toBe(MOBY);
    expect(seccomp.defaultAction).toBe('SCMP_ACT_ERRNO');
  });

  it('adds what bubblewrap calls, and no other system call', () => {
    expect([...new Set(ours.flatMap((r) => r.names))].sort()).toEqual(['clone', 'mount', 'pivot_root', 'umount2', 'unshare']);
    for (const r of ours) expect(r.action).toBe('SCMP_ACT_ALLOW');
    for (const r of ours) expect(r.comment).toMatch(/^ESC-356: /);
  });

  it('allows the two sets of namespaces of bwrapArgs, exactly', () => {
    const clones = ours.filter((r) => r.names.includes('clone'));
    expect(clones.map((r) => r.args)).toEqual([
      [{ index: 0, value: ALL_NAMESPACES, valueTwo: ALL_NAMESPACES, op: 'SCMP_CMP_MASKED_EQ' }],
      [{ index: 0, value: ALL_NAMESPACES, valueTwo: ALL_NAMESPACES & ~NEWNET, op: 'SCMP_CMP_MASKED_EQ' }],
    ]);
  });

  it('allows unshare for a second user namespace alone: `unshare -rn` stays refused', () => {
    expect(ours.find((r) => r.names.includes('unshare'))?.args).toEqual([{ index: 0, value: NEWUSER, op: 'SCMP_CMP_EQ' }]);
  });

  it('leaves refused what Docker refuses and bubblewrap does not ask', () => {
    for (const name of ['keyctl', 'add_key', 'request_key', 'bpf', 'setns', 'perf_event_open', 'init_module']) {
      expect(allowed(name), name).toBe(false);
    }
    expect(seccomp.syscalls.find((r) => r.names.includes('clone3') && !r.includes)?.errnoRet).toBe(38);
  });
});

describe('the AppArmor profile', () => {
  const rules = apparmor.split('\n').map((l) => l.trim());
  const mounts = rules.filter((l) => l.startsWith('mount'));

  it('is what Docker’s docker-default holds, less `deny mount`', () => {
    expect(rules).not.toContain('deny mount,');
    for (const rule of [
      'network,',
      'deny network alg,',
      'deny network vsock,',
      'capability,',
      'file,',
      'umount,',
      'deny @{PROC}/* w,   # deny write for all files directly in /proc (not in a subdir)',
      'deny @{PROC}/sysrq-trigger rwklx,',
      'deny @{PROC}/kcore rwklx,',
      'deny /sys/firmware/** rwklx,',
      'deny /sys/devices/virtual/powercap/** rwklx,',
      'deny /sys/kernel/security/** rwklx,',
    ]) {
      expect(rules, rule).toContain(rule);
    }
  });

  it('has no rule that mounts anything anywhere', () => {
    expect(rules).not.toContain('mount,');
    expect(rules).not.toContain('pivot_root,');
    expect(mounts.length).toBeGreaterThan(0);
    for (const m of mounts) expect(m, m).toMatch(/ -> \/\S*,$/);
    // Without an fstype, a `mount` rule takes any filesystem: its flags must say a bind or a change of propagation, or
    // the remount of a bind in its sandbox, by its exact flags (AppArmor 3.0 reads no other).
    const remounts = /^mount options=\((ro|rw), nosuid, nodev, remount, bind, silent, (relatime|noatime)\) -> \/newroot\/\*\*,$/;
    for (const m of mounts.filter((r) => !r.includes('fstype='))) {
      if (remounts.test(m)) continue;
      expect(m, m).toMatch(/^mount options=\(rw, (rbind|silent, (rslave|rprivate))\) /);
    }
    expect(mounts.filter((m) => remounts.test(m))).toHaveLength(3);
    expect(rules.filter((l) => l.startsWith('remount')).map((l) => l.split(' ').at(-1))).toEqual(['/newroot/**,']);
  });

  it('mounts tmpfs and devpts alone, and binds the sandbox’s tree, never the store’s folders', () => {
    const types = mounts.flatMap((m) => [...m.matchAll(/fstype=(\S+)/g)].map((f) => f[1]));
    expect([...new Set(types)].sort()).toEqual(['devpts', 'tmpfs']);
    const sources = mounts.filter((m) => m.includes('/oldroot/') && !m.startsWith('mount options=(rw, silent')).join('\n');
    expect(sources).not.toMatch(/\/oldroot\/(app|data|proc|sys|home)\b/);
    expect(sources).not.toContain('/oldroot/**');
  });
});
