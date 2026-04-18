import client from './client';

export interface KnownIssue {
  id: string;
  title: string;
  description: string;
  created_at: string;
  updated_at: string;
}

export interface Release {
  id: string;
  version: string;
  release_date: string;
  title: string | null;
  added: string[];
  changed: string[];
  fixed: string[];
  created_at: string;
  updated_at: string;
}

export interface PatchNotesResponse {
  known_issues: KnownIssue[];
  releases: Release[];
}

export interface KnownIssueInput {
  title: string;
  description: string;
}

export interface ReleaseInput {
  version: string;
  release_date: string;
  title?: string | null;
  added: string[];
  changed: string[];
  fixed: string[];
}

export async function fetchPatchNotes(): Promise<PatchNotesResponse> {
  const { data } = await client.get<PatchNotesResponse>('/patch-notes');
  return data;
}

// ── Known Issues (admin) ──────────────────────────────────────────────

export async function createKnownIssue(payload: KnownIssueInput): Promise<KnownIssue> {
  const { data } = await client.post<KnownIssue>('/patch-notes/known-issues', payload);
  return data;
}

export async function updateKnownIssue(
  id: string,
  payload: Partial<KnownIssueInput>,
): Promise<KnownIssue> {
  const { data } = await client.patch<KnownIssue>(`/patch-notes/known-issues/${id}`, payload);
  return data;
}

export async function deleteKnownIssue(id: string): Promise<void> {
  await client.delete(`/patch-notes/known-issues/${id}`);
}

// ── Releases (admin) ──────────────────────────────────────────────────

export async function createRelease(payload: ReleaseInput): Promise<Release> {
  const { data } = await client.post<Release>('/patch-notes/releases', payload);
  return data;
}

export async function updateRelease(
  id: string,
  payload: Partial<ReleaseInput>,
): Promise<Release> {
  const { data } = await client.patch<Release>(`/patch-notes/releases/${id}`, payload);
  return data;
}

export async function deleteRelease(id: string): Promise<void> {
  await client.delete(`/patch-notes/releases/${id}`);
}
