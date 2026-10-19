import { expect, it, vi } from 'vitest';
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
import { isCompatible, type EngineContract, type ManifestModel } from './desktopModelsApi';
const contract: EngineContract = { observation_profile: 'standard_lite_deck_v2', tensor_version: 2,
  feature_schema_version: 'new', tensor_size: 8850, tensor_layout_hash: 'sha256:current', action_space_size: 2192, action_schema_version: 1, engine_commit: null };
it('rejects same-shaped stale or incomplete models before download', () => {
  const model = { ...contract } as ManifestModel;
  expect(isCompatible(model, contract)).toBe(true);
  for (const patch of [{ observation_profile: undefined }, { tensor_version: 1 },
    { feature_schema_version: 'old' }, { tensor_layout_hash: 'sha256:old' },
    { tensor_size: 1375 }, { action_space_size: 1 }, { action_schema_version: 2 }, { action_schema_version: undefined }])
    expect(isCompatible({ ...model, ...patch }, contract)).toBe(false);
});
