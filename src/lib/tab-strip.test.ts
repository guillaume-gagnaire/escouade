import { describe, expect, it } from 'vitest';
import { alertColor, mostPressing } from './tab-strip';

describe('alertColor', () => {
  it('colors a blink by what the agent did: red when it failed, amber when it asks, green when it finished', () => {
    expect(alertColor('error')).toBe('var(--del)');
    expect(alertColor('waiting')).toBe('var(--wait)');
    expect(alertColor('done')).toBe('var(--run)');
  });
});

describe('mostPressing', () => {
  it('puts an error before a question, and a question before an end', () => {
    expect(mostPressing(['var(--run)', 'var(--del)', 'var(--wait)'])).toBe('var(--del)');
    expect(mostPressing([null, 'var(--run)', 'var(--wait)'])).toBe('var(--wait)');
    expect(mostPressing(['var(--run)', null])).toBe('var(--run)');
    expect(mostPressing([null, null])).toBeNull();
  });
});
