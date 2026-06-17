// Tauri-invoke wrappers for the desktop model manager (see src-tauri/src/models.rs).
//
// Only meaningful in desktop builds (VITE_BUILD_TARGET=desktop). On the web
// build these commands don't exist — callers must gate on IS_DESKTOP before
// importing or calling anything here.
//
// Kept separate from `modelsApi.ts` (which is the admin hosted-API surface
// for uploading/publishing models) to avoid conflating the two.

import { invoke } from '@tauri-apps/api/core';

export type ModelArchitecture = 'mlp' | 'lstm';

/// Mirrors `src-tauri/src/models.rs::ManifestModel` and
/// `digimon_gym/db/schemas.py::ManifestModel` — keep the three in sync.
export interface ManifestModel {
  id: string;
  name: string;
  model_type: ModelArchitecture;
  tensor_size: number;
  action_space_size: number;
  engine_commit: string | null;
  trained_at: string | null;
  file_sha256: string;
  file_size_bytes: number;
  url: string;
  deck_id: string | null;
  deck_name: string | null;
  notes: string | null;
}

export interface LocalModelMeta extends ManifestModel {
  downloaded_at: number;
}

export interface EngineContract {
  tensor_size: number;
  action_space_size: number;
  engine_commit: string | null;
}

export async function engineContract(): Promise<EngineContract> {
  return invoke<EngineContract>('models_engine_contract');
}

export async function fetchManifest(baseUrl: string): Promise<ManifestModel[]> {
  return invoke<ManifestModel[]>('models_fetch_manifest', { baseUrl });
}

export async function listLocal(): Promise<LocalModelMeta[]> {
  return invoke<LocalModelMeta[]>('models_list_local');
}

export async function downloadModel(model: ManifestModel): Promise<LocalModelMeta> {
  return invoke<LocalModelMeta>('models_download', { model });
}

export async function deleteModel(modelId: string): Promise<boolean> {
  return invoke<boolean>('models_delete', { modelId });
}

export async function loadCached(modelId: string): Promise<LocalModelMeta> {
  return invoke<LocalModelMeta>('models_load_cached', { modelId });
}

/// True if a manifest entry's shape contract matches the running engine.
/// The backend rejects mismatches at download time too, but checking client-
/// side lets the UI grey out incompatible rows before the user clicks.
export function isCompatible(model: ManifestModel, contract: EngineContract): boolean {
  return (
    model.tensor_size === contract.tensor_size &&
    model.action_space_size === contract.action_space_size
  );
}

/// Resolve + load the released "starter AI" model flagged in the hosted
/// manifest (`starter_ai_model_id`). Returns the loaded model id, or `null`
/// when no model is published / it's incompatible / the fetch fails — callers
/// fall back to the greedy CPU.
export async function resolveStarterModel(baseUrl: string): Promise<string | null> {
  return invoke<string | null>('models_resolve_starter', { baseUrl });
}
