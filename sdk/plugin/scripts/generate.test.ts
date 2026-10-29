// @vitest-environment node
import { describe, expect, it } from 'vitest';
import { manifestTs, protocolTs, stale, typeOf } from './generate.mjs';

// The SDK's generated types and the schemas they come from (ESC-181, PLG-9): one changed without the other fails here.

describe('the generated types of @escouade-dev/plugin', () => {
  it('are those the schemas give (run node sdk/plugin/scripts/generate.mjs)', async () => {
    expect(await stale()).toEqual([]);
  }, 30_000);

  it('follow a change of the schema', () => {
    const schema = {
      'x-protocolVersion': 1,
      'x-defaultTimeoutMs': 5000,
      'x-errors': {},
      'x-methods': {
        'a/b': { direction: 'toPlugin', params: { $ref: '#/definitions/FactName' }, result: { type: 'null' }, timeoutMs: 7 },
      },
      definitions: { FactName: { enum: ['x.y'] }, Thing: { type: 'object', properties: { n: { type: 'integer' } }, required: [] } },
    };
    const ts = protocolTs(schema);
    expect(ts).toContain("export type FactName = 'x.y';");
    expect(ts).toContain('n?: number;');
    expect(ts).toContain('"a/b": { params: FactName; result: null };');
    expect(ts).toContain('"a/b": 7,');
  });

  it('write each construct of the schemas', () => {
    expect(typeOf({ type: ['string', 'null'] })).toBe('string | null');
    expect(typeOf({ oneOf: [{ type: 'string' }, { type: 'object', additionalProperties: { type: 'string' } }] })).toBe(
      'string | Record<string, string>',
    );
    expect(typeOf({ type: 'object', additionalProperties: { type: 'string' }, anyOf: [{ required: ['en'] }, { required: ['fr'] }] })).toBe(
      'Record<string, string>',
    );
    expect(typeOf({ type: 'array', items: { type: 'object' } })).toBe('Array<Record<string, unknown>>');
    expect(manifestTs({ $id: 'x', type: 'object', properties: { id: { type: 'string' } }, required: ['id'] })).toContain('id: string;');
  });
});
