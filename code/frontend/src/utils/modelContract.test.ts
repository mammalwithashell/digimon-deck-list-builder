import { describe, expect, it } from 'vitest';
import { parseModelContract } from './modelContract';
const artifact = { observation_profile: 'standard_lite_v2', tensor_version: 2,
  feature_schema_version: 'old-but-authentic', tensor_size: 8410,
  tensor_layout_hash: `sha256:${'a'.repeat(64)}`, action_space_size: 2192, action_schema_version: 1 };
describe('artifact contract declaration', () => {
  it('preserves the artifact tuple instead of stamping current semantics', () => {
    expect(parseModelContract(JSON.stringify({ ...artifact, export_type: 'mlp' }))).toEqual(artifact);
  });
  it('rejects every missing contract field', () => {
    for (const key of Object.keys(artifact)) {
      const row: Record<string, unknown> = { ...artifact }; delete row[key];
      expect(() => parseModelContract(JSON.stringify(row))).toThrow();
    }
  });
  it('rejects malformed fields and nonobjects', () => {
    for (const patch of [{ tensor_size: '8410' }, { tensor_version: 1.5 }, { action_space_size: 0 }, { action_schema_version: 0 }, { action_schema_version: true }, { tensor_layout_hash: 'old' }])
      expect(() => parseModelContract(JSON.stringify({ ...artifact, ...patch }))).toThrow();
    for (const value of ['null', '[]', '1']) expect(() => parseModelContract(value)).toThrow();
  });
});
