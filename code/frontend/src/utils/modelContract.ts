/** The artifact's training contract, never inferred from its shape. */
export interface ModelContract {
  observation_profile: string;
  tensor_version: number;
  feature_schema_version: string;
  tensor_size: number;
  tensor_layout_hash: string;
  action_space_size: number;
  action_schema_version: number;
}

export function parseModelContract(text: string): ModelContract {
  const value: unknown = JSON.parse(text);
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('Invalid model contract');
  const row = value as Record<string, unknown>;
  for (const key of ['observation_profile', 'feature_schema_version', 'tensor_layout_hash']) {
    if (typeof row[key] !== 'string' || !(row[key] as string).trim()) throw new Error(`Missing or invalid ${key}`);
  }
  for (const key of ['tensor_version', 'tensor_size', 'action_space_size', 'action_schema_version']) {
    if (typeof row[key] !== 'number' || !Number.isSafeInteger(row[key]) || (row[key] as number) <= 0) throw new Error(`Missing or invalid ${key}`);
  }
  if (!/^sha256:[a-f0-9]{64}$/.test(row.tensor_layout_hash as string)) throw new Error('Invalid tensor_layout_hash');
  return {
    observation_profile: row.observation_profile as string,
    tensor_version: row.tensor_version as number,
    feature_schema_version: row.feature_schema_version as string,
    tensor_size: row.tensor_size as number,
    tensor_layout_hash: row.tensor_layout_hash as string,
    action_space_size: row.action_space_size as number,
    action_schema_version: row.action_schema_version as number,
  };
}
