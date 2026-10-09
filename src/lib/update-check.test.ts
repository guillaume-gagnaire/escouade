import { describe, expect, it } from 'vitest';
import { fakeBackend } from '../test/ipc';
import { check } from './update-check';

describe('check', () => {
  it('finds an update through the backend, which downloads or frees it', async () => {
    const backend = fakeBackend({
      update_check: () => ({ id: 3, version: '1.6.0', notes: 'Notes 1.6.0' }),
      update_download: () => true,
    });
    const update = await check();
    expect(update).toMatchObject({ version: '1.6.0', body: 'Notes 1.6.0' });
    expect(await update!.download()).toBe(true);
    expect(backend.called('update_download').map((c) => c.args)).toEqual([{ id: 3 }]);
    await update!.close();
    expect(backend.called('update_close').map((c) => c.args)).toEqual([{ id: 3 }]);
  });

  it('finds nothing when the app is up to date, and lets a failure through', async () => {
    fakeBackend({ update_check: () => null });
    expect(await check()).toBeNull();
    fakeBackend({ update_check: () => Promise.reject('Escouade injoignable') });
    await expect(check()).rejects.toBe('Escouade injoignable');
  });
});
