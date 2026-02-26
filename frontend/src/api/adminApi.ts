import client from './client';

export interface IssueItem {
  id: string;
  card_id: string;
  description: string;
  source: 'player' | 'judge' | 'system';
  severity: 'low' | 'medium' | 'high' | 'critical';
  status: 'new' | 'approved_for_ai' | 'rejected' | 'resolved';
  created_by?: string | null;
  approved_by?: string | null;
  triage_notes: string;
  resolution_notes: string;
  created_at: string;
  updated_at: string;
  resolved_at?: string | null;
}

export interface ApproveSetIssuesResponse {
  set_id: string;
  matched_count: number;
  approved_count: number;
  skipped_count: number;
  issue_ids: string[];
}

export interface AITaskItem {
  id: string;
  task_type: 'review_batch' | 'qa_analysis' | 'engine_audit' | 'script_autofix' | 'llm_transpile' | 'transpiler_learn';
  payload: Record<string, unknown>;
  status: 'queued' | 'running' | 'completed' | 'failed';
  result?: Record<string, unknown> | null;
  sanitized_input: Record<string, unknown>;
  retrieval_refs: Array<Record<string, unknown>>;
  model_name?: string | null;
  cost_estimate: number;
  cost_actual: number;
  input_tokens: number;
  output_tokens: number;
  error_text?: string | null;
  attempts: number;
  max_attempts: number;
  created_by?: string | null;
  batch_id?: string | null;
  run_mode?: 'pr' | 'main' | null;
  scope_profile?: 'script' | 'script_engine' | 'script_engine_transpiler' | null;
  started_at?: string | null;
  completed_at?: string | null;
  created_at: string;
  updated_at: string;
}

export interface PromotionItem {
  id: string;
  card_id: string;
  set_id: string;
  module_name: string;
  generated_hash: string;
  frozen_hash: string;
  manifest_version: number;
  promoted_by?: string | null;
  ai_task_id?: string | null;
  batch_id?: string | null;
  notes: string;
  created_at: string;
}

export interface AIFixBatchItem {
  id: string;
  batch_id: string;
  issue_id?: string | null;
  card_id: string;
  task_id?: string | null;
  status: 'pending' | 'queued' | 'running' | 'completed' | 'failed' | 'applied' | 'canceled';
  error_text?: string | null;
  applied_at?: string | null;
  commit_sha?: string | null;
  created_at: string;
  updated_at: string;
}

export interface AIFixBatch {
  id: string;
  set_id: string;
  run_mode: 'pr' | 'main';
  scope_profile: 'script' | 'script_engine' | 'script_engine_transpiler';
  status: 'running' | 'completed' | 'failed' | 'failed_no_changes' | 'canceled';
  created_by?: string | null;
  model_name?: string | null;
  concurrency: number;
  max_total_cost_usd: number;
  failure_rate_stop: number;
  max_tasks: number;
  queued_count: number;
  running_count: number;
  completed_count: number;
  failed_count: number;
  applied_count: number;
  commit_count: number;
  stopped_reason?: string | null;
  pr_url?: string | null;
  created_at: string;
  updated_at: string;
}

export interface AIFixBatchCreateResponse {
  batch?: AIFixBatch | null;
  eligible_count: number;
  selected_count: number;
  queued_task_ids: string[];
  preview_only: boolean;
  cards: string[];
}

export interface AIFixBatchDetailResponse {
  batch: AIFixBatch;
  items: AIFixBatchItem[];
}

export interface AISetRunItem {
  id: string;
  set_run_id: string;
  card_id: string;
  set_id: string;
  module_name: string;
  review_faithful?: boolean | null;
  issue_id?: string | null;
  qa_task_id?: string | null;
  review_task_id?: string | null;
  fix_task_id?: string | null;
  fix_apply_status: 'pending' | 'applied' | 'failed' | 'skipped';
  commit_sha?: string | null;
  pr_url?: string | null;
  error_text?: string | null;
  created_at: string;
  updated_at: string;
}

export interface AISetRun {
  id: string;
  set_id: string;
  status: 'running' | 'completed' | 'failed' | 'canceled';
  stage: 'qa' | 'review' | 'fix' | 'completed' | 'canceled';
  run_mode: 'pr' | 'main';
  scope_profile: 'script' | 'script_engine' | 'script_engine_transpiler';
  model_name?: string | null;
  created_by?: string | null;
  max_total_cost_usd: number;
  failure_rate_stop: number;
  max_fix_tasks: number;
  qa_total: number;
  qa_completed: number;
  qa_failed: number;
  review_total: number;
  review_completed: number;
  review_failed: number;
  fix_total: number;
  fix_completed: number;
  fix_failed: number;
  fix_applied: number;
  stopped_reason?: string | null;
  created_at: string;
  updated_at: string;
  completed_at?: string | null;
}

export interface AISetRunDetailResponse {
  run: AISetRun;
  items: AISetRunItem[];
  pr_queue: AISetRunItem[];
}

export interface AppliedCardItem {
  id: string;
  ai_task_id?: string | null;
  batch_id?: string | null;
  card_id: string;
  scope_profile: string;
  run_mode: string;
  applied_files: string[];
  commit_sha?: string | null;
  pr_url?: string | null;
  status: 'applied' | 'failed';
  error_text?: string | null;
  created_by?: string | null;
  created_at: string;
}

export interface AITaskQueryParams {
  status?: 'queued' | 'running' | 'completed' | 'failed';
  task_type?: 'review_batch' | 'qa_analysis' | 'engine_audit' | 'script_autofix' | 'llm_transpile' | 'transpiler_learn';
  run_mode?: 'pr' | 'main';
  scope_profile?: 'script' | 'script_engine' | 'script_engine_transpiler';
  batch_id?: string;
  limit?: number;
}

export async function getIssues(): Promise<IssueItem[]> {
  const { data } = await client.get<IssueItem[]>('/issues');
  return data;
}

export async function updateIssue(
  issueId: string,
  updates: Partial<Pick<IssueItem, 'status' | 'severity' | 'triage_notes' | 'resolution_notes'>>,
): Promise<IssueItem> {
  const { data } = await client.patch<IssueItem>(`/issues/${issueId}`, updates);
  return data;
}

export async function approveSetIssues(payload: {
  set_id: string;
  status_filter?: 'new' | 'approved_for_ai' | 'rejected' | 'resolved';
}): Promise<ApproveSetIssuesResponse> {
  const { data } = await client.post<ApproveSetIssuesResponse>('/admin/issues/approve-set', payload);
  return data;
}

export async function getAITasks(params?: AITaskQueryParams): Promise<AITaskItem[]> {
  const { data } = await client.get<AITaskItem[]>('/admin/ai-tasks', { params });
  return data;
}

export async function getAITask(taskId: string): Promise<AITaskItem> {
  const { data } = await client.get<AITaskItem>(`/admin/ai-tasks/${taskId}`);
  return data;
}

export async function createAITask(payload: {
  task_type: 'review_batch' | 'qa_analysis' | 'engine_audit' | 'script_autofix' | 'llm_transpile' | 'transpiler_learn';
  payload: Record<string, unknown>;
  model_name?: string;
  cost_estimate?: number;
  max_attempts?: number;
  run_mode?: 'pr' | 'main';
  scope_profile?: 'script' | 'script_engine' | 'script_engine_transpiler';
  batch_id?: string;
}): Promise<AITaskItem> {
  const { data } = await client.post<AITaskItem>('/admin/ai-tasks', payload);
  return data;
}

export async function retryAITask(taskId: string): Promise<{ task_id: string; status: string }> {
  const { data } = await client.post<{ task_id: string; status: string }>(`/admin/ai-tasks/${taskId}/retry`);
  return data;
}

export async function applyAITaskFix(
  taskId: string,
): Promise<{ task_id: string; status: string; applied_files: string[]; commit_sha?: string | null; pr_url?: string | null }> {
  const { data } = await client.post<{
    task_id: string;
    status: string;
    applied_files: string[];
    commit_sha?: string | null;
    pr_url?: string | null;
  }>(`/admin/ai-tasks/${taskId}/apply-fix`);
  return data;
}

export async function promoteTaskCard(
  taskId: string,
  payload: { card_id: string; notes?: string },
): Promise<PromotionItem> {
  const { data } = await client.post<PromotionItem>(`/admin/ai-tasks/${taskId}/promote`, payload);
  return data;
}

export async function getPromotions(): Promise<PromotionItem[]> {
  const { data } = await client.get<PromotionItem[]>('/admin/promotions');
  return data;
}

export async function previewAIFixBatch(payload: {
  set_id: string;
  max_tasks?: number;
}): Promise<AIFixBatchCreateResponse> {
  const { data } = await client.get<AIFixBatchCreateResponse>('/admin/ai-batches/preview', {
    params: payload,
  });
  return data;
}

export async function createAIFixBatch(payload: {
  set_id: string;
  run_mode: 'pr' | 'main';
  scope_profile: 'script' | 'script_engine' | 'script_engine_transpiler';
  model_name?: string;
  concurrency: number;
  max_total_cost_usd: number;
  failure_rate_stop: number;
  max_tasks: number;
  dry_run: boolean;
}): Promise<AIFixBatchCreateResponse> {
  const { data } = await client.post<AIFixBatchCreateResponse>('/admin/ai-batches', payload);
  return data;
}

export async function getAIFixBatches(): Promise<AIFixBatch[]> {
  const { data } = await client.get<AIFixBatch[]>('/admin/ai-batches');
  return data;
}

export async function getAIFixBatch(batchId: string): Promise<AIFixBatchDetailResponse> {
  const { data } = await client.get<AIFixBatchDetailResponse>(`/admin/ai-batches/${batchId}`);
  return data;
}

export async function cancelAIFixBatch(batchId: string): Promise<{ batch_id: string; status: string }> {
  const { data } = await client.post<{ batch_id: string; status: string }>(`/admin/ai-batches/${batchId}/cancel`);
  return data;
}

export async function createSetRun(payload: {
  set_id: string;
  run_mode: 'pr' | 'main';
  scope_profile: 'script' | 'script_engine' | 'script_engine_transpiler';
  model_name?: string;
  max_total_cost_usd?: number;
  failure_rate_stop?: number;
  max_fix_tasks?: number;
}): Promise<AISetRun> {
  const { data } = await client.post<AISetRun>('/admin/set-runs', payload);
  return data;
}

export async function getSetRuns(params?: {
  set_id?: string;
  status?: 'running' | 'completed' | 'failed' | 'canceled';
  limit?: number;
}): Promise<AISetRun[]> {
  const { data } = await client.get<AISetRun[]>('/admin/set-runs', { params });
  return data;
}

export async function getSetRun(setRunId: string): Promise<AISetRunDetailResponse> {
  const { data } = await client.get<AISetRunDetailResponse>(`/admin/set-runs/${setRunId}`);
  return data;
}

export async function cancelSetRun(setRunId: string): Promise<{ set_run_id: string; status: string }> {
  const { data } = await client.post<{ set_run_id: string; status: string }>(`/admin/set-runs/${setRunId}/cancel`);
  return data;
}

export async function getAppliedCards(params?: {
  status?: 'applied' | 'failed';
  set_id?: string;
  limit?: number;
}): Promise<AppliedCardItem[]> {
  const { data } = await client.get<AppliedCardItem[]>('/admin/applied-cards', { params });
  return data;
}

export async function queueIssueFix(issueId: string): Promise<AITaskItem> {
  const { data } = await client.post<AITaskItem>(`/admin/issues/${issueId}/queue-fix`);
  return data;
}

export async function createPromotion(payload: {
  card_id: string;
  set_id: string;
  module_name: string;
  expected_generated_hash: string;
  notes?: string;
  ai_task_id?: string;
}): Promise<PromotionItem> {
  const { data } = await client.post<PromotionItem>('/admin/promotions', payload);
  return data;
}

export async function createTranspilerLearnRun(sourceSetRunId: string, minClusterSize = 3) {
  const resp = await client.post('/admin/transpiler-learn', {
    source_set_run_id: sourceSetRunId,
    min_cluster_size: minClusterSize,
  });
  return resp.data;
}

export async function getTranspilerLearnRun(learnRunId: string) {
  const resp = await client.get(`/admin/transpiler-learn/${learnRunId}`);
  return resp.data;
}
