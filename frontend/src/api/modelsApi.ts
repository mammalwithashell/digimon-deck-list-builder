import client from './client';

export type ModelType = 'mlp' | 'lstm';
export type ModelState = 'pending' | 'uploaded' | 'failed';

export interface AIModel {
  id: string;
  name: string;
  model_type: ModelType;
  tensor_size: number | null;
  action_space_size: number | null;
  engine_commit: string | null;
  trained_at: string | null;
  file_sha256: string | null;
  file_size_bytes: number | null;
  spaces_key: string;
  deck_id: string | null;
  uploaded_by: string | null;
  published: boolean;
  state: ModelState;
  notes: string | null;
  created_at: string;
  updated_at: string;
  uploader_username: string | null;
  deck_name: string | null;
}

export interface CreateModelInput {
  name: string;
  model_type: ModelType;
  engine_commit?: string | null;
  trained_at?: string | null;
  deck_id?: string | null;
  notes?: string | null;
}

export interface CreateModelResponse {
  id: string;
  upload_url: string;
  spaces_key: string;
  expires_in: number;
}

export interface ConfirmModelResponse {
  id: string;
  state: ModelState;
  file_sha256: string;
  file_size_bytes: number;
  tensor_size: number;
  action_space_size: number;
}

export interface UpdateModelInput {
  name?: string;
  deck_id?: string | null;
  published?: boolean;
  notes?: string | null;
  engine_commit?: string | null;
  trained_at?: string | null;
}

export interface ListModelsFilters {
  state?: ModelState;
  published?: boolean;
  model_type?: ModelType;
  deck_id?: string;
}

export async function listModels(filters?: ListModelsFilters): Promise<AIModel[]> {
  const params: Record<string, string> = {};
  if (filters?.state !== undefined) params.state = filters.state;
  if (filters?.published !== undefined) params.published = String(filters.published);
  if (filters?.model_type !== undefined) params.model_type = filters.model_type;
  if (filters?.deck_id !== undefined) params.deck_id = filters.deck_id;
  const { data } = await client.get<{ models: AIModel[] }>('/admin/models', { params });
  return data.models;
}

export async function createModel(input: CreateModelInput): Promise<CreateModelResponse> {
  const { data } = await client.post<CreateModelResponse>('/admin/models', input);
  return data;
}

export async function confirmModel(id: string): Promise<ConfirmModelResponse> {
  const { data } = await client.post<ConfirmModelResponse>(`/admin/models/${id}/confirm`);
  return data;
}

export async function updateModel(id: string, input: UpdateModelInput): Promise<AIModel> {
  const { data } = await client.patch<AIModel>(`/admin/models/${id}`, input);
  return data;
}

export async function deleteModel(id: string): Promise<void> {
  await client.delete(`/admin/models/${id}`);
}

/**
 * Upload a file to a presigned PUT URL using XMLHttpRequest so we can track
 * upload progress. The presigned URL is already signed — do NOT send auth
 * headers or it will break the Spaces signature.
 */
export function uploadWithProgress(
  url: string,
  file: File,
  onProgress: (pct: number) => void,
): Promise<void> {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    xhr.open('PUT', url);
    xhr.setRequestHeader('Content-Type', 'application/octet-stream');

    xhr.upload.onprogress = (event) => {
      if (event.lengthComputable) {
        onProgress(Math.round((event.loaded / event.total) * 100));
      }
    };

    xhr.onload = () => {
      if (xhr.status >= 200 && xhr.status < 300) {
        onProgress(100);
        resolve();
      } else {
        reject(new Error(`Upload failed: HTTP ${xhr.status} ${xhr.statusText}`));
      }
    };

    xhr.onerror = () => {
      reject(new Error('Upload failed: network error'));
    };

    xhr.send(file);
  });
}
