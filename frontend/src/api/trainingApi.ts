import client from './client';

// ── Types ──────────────────────────────────────────────────────────

export interface AgentItem {
  id: string;
  name: string;
  archetype: string;
  algorithm: 'mlp' | 'lstm';
  weights_path: string;
  deck_source?: string | null;
  total_games: number;
  total_wins: number;
  total_losses: number;
  total_draws: number;
  win_rate: number;
  total_timesteps_trained: number;
  owner_id?: string | null;
  created_at: string;
  updated_at: string;
}

export interface TrainingJobItem {
  id: string;
  job_type: 'train_vs_greedy' | 'train_vs_random' | 'train_vs_agent' | 'evaluate';
  status: 'queued' | 'running' | 'completed' | 'failed' | 'canceled';
  agent_id?: string | null;
  opponent_agent_id?: string | null;
  opponent_type?: string | null;
  gauntlet_id?: string | null;
  gauntlet_stage?: number | null;
  total_games: number;
  total_timesteps: number;
  completed_games: number;
  wins: number;
  losses: number;
  draws: number;
  win_rate?: number | null;
  config: Record<string, unknown>;
  result?: Record<string, unknown> | null;
  error_text?: string | null;
  worker_id?: string | null;
  device?: string | null;
  created_by?: string | null;
  started_at?: string | null;
  completed_at?: string | null;
  created_at: string;
  updated_at: string;
}

export interface GauntletParticipantItem {
  id: string;
  slot: number;
  role: 'core' | 'exploiter';
  agent_id?: string | null;
  archetype_name: string;
  deck_source?: string | null;
  meta_share: number;
  threat_index: number;
  tournament_win_rate?: number | null;
  expected_meta_win_rate?: number | null;
}

export interface GauntletItem {
  id: string;
  name: string;
  status: 'configuring' | 'stage_1' | 'stage_2' | 'stage_3' | 'completed' | 'failed' | 'canceled';
  current_stage: number;
  stage1_games: number;
  stage2_games: number;
  stage3_games_per_matchup: number;
  participants: GauntletParticipantItem[];
  error_text?: string | null;
  created_by?: string | null;
  started_at?: string | null;
  completed_at?: string | null;
  created_at: string;
  updated_at: string;
}

export interface GauntletDetailItem {
  gauntlet: GauntletItem;
  jobs: TrainingJobItem[];
  matchup_matrix?: Record<string, Record<string, number>> | null;
  tournament_rankings?: Array<Record<string, unknown>> | null;
}

export interface MatchupResultItem {
  id: string;
  gauntlet_id: string;
  agent_a_id?: string | null;
  agent_b_id?: string | null;
  games_played: number;
  a_wins: number;
  b_wins: number;
  draws: number;
  a_win_rate: number;
}

export interface ArchetypeInfo {
  name: string;
  display_card_id?: string | null;
  primary_color?: string | null;
  meta_share: number;
  threat_index: number;
  decklists_count: number;
  sample_decklist: string[];
}

// ── Agents ─────────────────────────────────────────────────────────

export async function getAgents(params?: { archetype?: string; limit?: number }): Promise<AgentItem[]> {
  const { data } = await client.get<AgentItem[]>('/admin/training/agents', { params });
  return data;
}

export async function createAgent(payload: {
  name: string;
  archetype: string;
  algorithm: 'mlp' | 'lstm';
  deck: string[];
  deck_source?: string;
}): Promise<AgentItem> {
  const { data } = await client.post<AgentItem>('/admin/training/agents', payload);
  return data;
}

export async function getAgent(agentId: string): Promise<AgentItem> {
  const { data } = await client.get<AgentItem>(`/admin/training/agents/${agentId}`);
  return data;
}

export async function updateAgent(
  agentId: string,
  updates: { name?: string; archetype?: string },
): Promise<AgentItem> {
  const { data } = await client.patch<AgentItem>(`/admin/training/agents/${agentId}`, updates);
  return data;
}

export async function deleteAgent(agentId: string): Promise<{ id: string; deleted: boolean }> {
  const { data } = await client.delete<{ id: string; deleted: boolean }>(`/admin/training/agents/${agentId}`);
  return data;
}

// ── Training Jobs ──────────────────────────────────────────────────

export async function createJob(payload: {
  job_type: 'train_vs_greedy' | 'train_vs_random' | 'train_vs_agent' | 'evaluate';
  agent_id: string;
  opponent_agent_id?: string;
  total_timesteps?: number;
  total_games?: number;
  config?: Record<string, unknown>;
}): Promise<TrainingJobItem> {
  const { data } = await client.post<TrainingJobItem>('/admin/training/jobs', payload);
  return data;
}

export async function getJobs(params?: {
  status?: string;
  agent_id?: string;
  gauntlet_id?: string;
  limit?: number;
}): Promise<TrainingJobItem[]> {
  const { data } = await client.get<TrainingJobItem[]>('/admin/training/jobs', { params });
  return data;
}

export async function getJob(jobId: string): Promise<TrainingJobItem> {
  const { data } = await client.get<TrainingJobItem>(`/admin/training/jobs/${jobId}`);
  return data;
}

export async function cancelJob(jobId: string): Promise<{ job_id: string; status: string }> {
  const { data } = await client.post<{ job_id: string; status: string }>(`/admin/training/jobs/${jobId}/cancel`);
  return data;
}

// ── Gauntlets ──────────────────────────────────────────────────────

export async function createGauntlet(payload: {
  name: string;
  participants: Array<{
    archetype_name: string;
    deck: string[];
    deck_source?: string;
    role: 'core' | 'exploiter';
    meta_share: number;
    threat_index: number;
  }>;
  stage1_games?: number;
  stage2_games?: number;
  stage3_games_per_matchup?: number;
  config?: Record<string, unknown>;
}): Promise<GauntletItem> {
  const { data } = await client.post<GauntletItem>('/admin/training/gauntlets', payload);
  return data;
}

export async function getGauntlets(): Promise<GauntletItem[]> {
  const { data } = await client.get<GauntletItem[]>('/admin/training/gauntlets');
  return data;
}

export async function getGauntletDetail(gauntletId: string): Promise<GauntletDetailItem> {
  const { data } = await client.get<GauntletDetailItem>(`/admin/training/gauntlets/${gauntletId}`);
  return data;
}

export async function startGauntlet(gauntletId: string): Promise<GauntletItem> {
  const { data } = await client.post<GauntletItem>(`/admin/training/gauntlets/${gauntletId}/start`);
  return data;
}

export async function cancelGauntlet(gauntletId: string): Promise<{ gauntlet_id: string; status: string }> {
  const { data } = await client.post<{ gauntlet_id: string; status: string }>(`/admin/training/gauntlets/${gauntletId}/cancel`);
  return data;
}

export async function getMatchups(gauntletId: string): Promise<MatchupResultItem[]> {
  const { data } = await client.get<MatchupResultItem[]>(`/admin/training/gauntlets/${gauntletId}/matchups`);
  return data;
}

// ── Deck Library ───────────────────────────────────────────────────

export async function getDeckLibraryArchetypes(): Promise<ArchetypeInfo[]> {
  const { data } = await client.get<ArchetypeInfo[]>('/admin/training/deck-library/archetypes');
  return data;
}
