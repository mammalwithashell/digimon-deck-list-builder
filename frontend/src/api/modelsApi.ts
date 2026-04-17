/**
 * Desktop-side `/models` catalog client.
 *
 * The sidecar's `/models/manifest` endpoint already fuses bundled-local
 * state with whatever the hosted API advertises, so the frontend doesn't
 * need to talk to both. Always goes through `localClient` (even in
 * browser/hosted mode — the dev proxy forwards to the sidecar).
 */
import { localClient } from './client';

export type ModelArch = 'mlp' | 'lstm';

export interface ModelVersionEntry {
  version: string;
  size_bytes: number;
  sha256: string;
  url: string;
  min_engine_version: string | null;
  onnx_input_shapes: Record<string, unknown> | null;
  changelog: string;
  is_deprecated: boolean;
  released_at: string;
  is_cached: boolean;
  cache_path: string | null;
}

export interface ModelEntry {
  slug: string;
  name: string;
  arch: ModelArch;
  description: string;
  deck_pool: string | null;
  min_engine_version: string;
  is_deprecated: boolean;
  versions: ModelVersionEntry[];
  bundled_filename: string | null;
  bundled_path: string | null;
}

export interface ModelManifest {
  api_base: string;
  cache_dir: string | null;
  bundled_dir: string;
  models: ModelEntry[];
}

export async function fetchModelManifest(refresh = false): Promise<ModelManifest> {
  const { data } = await localClient.get<ModelManifest>('/models/manifest', {
    params: refresh ? { refresh: 'true' } : {},
  });
  return data;
}

export async function downloadModel(
  slug: string,
  version: string,
): Promise<{ slug: string; version: string; path: string }> {
  const { data } = await localClient.post('/models/download', { slug, version });
  return data;
}

export async function deleteCachedModel(
  slug: string,
  version: string,
): Promise<void> {
  await localClient.post('/models/delete', { slug, version });
}

/**
 * Pick the right opaque "model reference" to pass to `createGame`.
 * For bundled entries (no versions) we still send the filename; for
 * catalog entries we send the slug — the sidecar resolver handles both.
 */
export function modelReferenceForEntry(entry: ModelEntry): string | null {
  if (entry.bundled_filename) return entry.bundled_filename;
  const latest = entry.versions.find((v) => !v.is_deprecated) ?? entry.versions[0];
  return latest ? entry.slug : null;
}

export function isEntryAvailable(entry: ModelEntry): boolean {
  if (entry.bundled_filename) return true;
  const latest = entry.versions.find((v) => !v.is_deprecated) ?? entry.versions[0];
  return Boolean(latest?.is_cached);
}
