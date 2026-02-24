import { useEffect, useMemo, useState } from 'react';
import {
  applyAITaskFix,
  createAITask,
  getAIFixBatch,
  getAITask,
  getAITasks,
  getAppliedCards,
  promoteTaskCard,
  retryAITask,
  type AIFixBatchDetailResponse,
  type AITaskItem,
  type AppliedCardItem,
} from '@/api/adminApi';
import { StatusBadge } from '@/components/admin/StatusBadge';
import { CommitLink, PrLink } from '@/components/admin/GitLinks';

const DEFAULT_PAYLOAD = JSON.stringify(
  {
    cards: [
      { card_id: 'BT24-001', set_id: 'bt24', module_name: 'bt24_001' },
      { card_id: 'BT24-002', set_id: 'bt24', module_name: 'bt24_002' },
      { card_id: 'BT24-003', set_id: 'bt24', module_name: 'bt24_003' },
      { card_id: 'BT24-004', set_id: 'bt24', module_name: 'bt24_004' },
      { card_id: 'BT24-005', set_id: 'bt24', module_name: 'bt24_005' },
    ],
  },
  null,
  2,
);

type TaskTypeFilter = 'all' | 'review_batch' | 'qa_analysis' | 'engine_audit' | 'script_autofix';
type TaskStatusFilter = 'all' | 'queued' | 'running' | 'completed' | 'failed';
type RunModeFilter = 'all' | 'pr' | 'main' | 'n/a';
type ScopeFilter = 'all' | 'script' | 'script_engine' | 'script_engine_transpiler' | 'n/a';
type ActiveTab = 'tasks' | 'applied';
type AppliedStatusFilter = 'all' | 'applied' | 'failed';

export function AdminTasksPage() {
  const [tasks, setTasks] = useState<AITaskItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [detailLoadingTaskId, setDetailLoadingTaskId] = useState<string | null>(null);
  const [expandedTaskId, setExpandedTaskId] = useState<string | null>(null);
  const [taskDetails, setTaskDetails] = useState<Record<string, AITaskItem>>({});
  const [batchDetails, setBatchDetails] = useState<Record<string, AIFixBatchDetailResponse>>({});
  const [loadingBatchIds, setLoadingBatchIds] = useState<Record<string, boolean>>({});
  const [promotingKeys, setPromotingKeys] = useState<Record<string, boolean>>({});
  const [promotedKeys, setPromotedKeys] = useState<Record<string, boolean>>({});
  const [applyingTaskIds, setApplyingTaskIds] = useState<Record<string, boolean>>({});
  const [promotionNotes, setPromotionNotes] = useState<Record<string, string>>({});
  const [error, setError] = useState<string | null>(null);

  const [taskType, setTaskType] = useState<'review_batch' | 'qa_analysis' | 'engine_audit' | 'script_autofix'>('review_batch');
  const [payloadText, setPayloadText] = useState(DEFAULT_PAYLOAD);
  const [modelName, setModelName] = useState('');
  const [costEstimate, setCostEstimate] = useState('0.5');

  const [statusFilter, setStatusFilter] = useState<TaskStatusFilter>('all');
  const [taskTypeFilter, setTaskTypeFilter] = useState<TaskTypeFilter>('all');
  const [runModeFilter, setRunModeFilter] = useState<RunModeFilter>('all');
  const [scopeFilter, setScopeFilter] = useState<ScopeFilter>('all');
  const [batchOnlyFilter, setBatchOnlyFilter] = useState(false);
  const [batchIdFilter, setBatchIdFilter] = useState('');
  const [limitFilter, setLimitFilter] = useState('250');

  // Applied Cards tab state
  const [activeTab, setActiveTab] = useState<ActiveTab>('tasks');
  const [appliedCards, setAppliedCards] = useState<AppliedCardItem[]>([]);
  const [appliedLoading, setAppliedLoading] = useState(false);
  const [appliedStatusFilter, setAppliedStatusFilter] = useState<AppliedStatusFilter>('all');
  const [appliedSetFilter, setAppliedSetFilter] = useState('');

  // Store apply-fix results for showing git links
  const [applyResults, setApplyResults] = useState<Record<string, { commit_sha?: string | null; pr_url?: string | null }>>({});

  const refresh = async () => {
    setLoading(true);
    setError(null);
    try {
      const limit = Math.min(500, Math.max(1, Number(limitFilter) || 250));
      const params: {
        status?: 'queued' | 'running' | 'completed' | 'failed';
        task_type?: 'review_batch' | 'qa_analysis' | 'engine_audit' | 'script_autofix';
        run_mode?: 'pr' | 'main';
        scope_profile?: 'script' | 'script_engine' | 'script_engine_transpiler';
        limit: number;
      } = { limit };
      if (statusFilter !== 'all') {
        params.status = statusFilter;
      }
      if (taskTypeFilter !== 'all') {
        params.task_type = taskTypeFilter;
      }
      if (runModeFilter !== 'all' && runModeFilter !== 'n/a') {
        params.run_mode = runModeFilter;
      }
      if (scopeFilter !== 'all' && scopeFilter !== 'n/a') {
        params.scope_profile = scopeFilter;
      }
      const data = await getAITasks(params);
      setTasks(data);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load AI tasks';
      setError(message);
    } finally {
      setLoading(false);
    }
  };

  const refreshApplied = async () => {
    setAppliedLoading(true);
    setError(null);
    try {
      const params: { status?: 'applied' | 'failed'; set_id?: string; limit: number } = { limit: 200 };
      if (appliedStatusFilter !== 'all') {
        params.status = appliedStatusFilter;
      }
      if (appliedSetFilter.trim()) {
        params.set_id = appliedSetFilter.trim();
      }
      const data = await getAppliedCards(params);
      setAppliedCards(data);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load applied cards';
      setError(message);
    } finally {
      setAppliedLoading(false);
    }
  };

  useEffect(() => {
    void refresh();
    // only initial load
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const totalEstimated = useMemo(
    () => tasks.reduce((acc, t) => acc + Number(t.cost_estimate || 0), 0),
    [tasks],
  );

  const filteredTasks = useMemo(() => {
    const normalizedBatchQuery = batchIdFilter.trim().toLowerCase();
    return tasks.filter((task) => {
      if (statusFilter !== 'all' && task.status !== statusFilter) {
        return false;
      }
      if (taskTypeFilter !== 'all' && task.task_type !== taskTypeFilter) {
        return false;
      }
      if (runModeFilter === 'n/a' && task.run_mode) {
        return false;
      }
      if (runModeFilter === 'pr' || runModeFilter === 'main') {
        if (task.run_mode !== runModeFilter) {
          return false;
        }
      }
      if (scopeFilter === 'n/a' && task.scope_profile) {
        return false;
      }
      if (scopeFilter !== 'all' && scopeFilter !== 'n/a') {
        if (task.scope_profile !== scopeFilter) {
          return false;
        }
      }
      if (batchOnlyFilter && !task.batch_id) {
        return false;
      }
      if (normalizedBatchQuery) {
        const value = String(task.batch_id ?? '').toLowerCase();
        if (!value.includes(normalizedBatchQuery)) {
          return false;
        }
      }
      return true;
    });
  }, [tasks, statusFilter, taskTypeFilter, runModeFilter, scopeFilter, batchOnlyFilter, batchIdFilter]);

  // Group applied cards by set_id
  const groupedApplied = useMemo(() => {
    const groups: Record<string, AppliedCardItem[]> = {};
    for (const card of appliedCards) {
      const setId = card.card_id.split('-')[0] ?? 'unknown';
      if (!groups[setId]) {
        groups[setId] = [];
      }
      groups[setId].push(card);
    }
    return Object.entries(groups).sort(([a], [b]) => a.localeCompare(b));
  }, [appliedCards]);

  const onCreateTask = async () => {
    try {
      const payload = JSON.parse(payloadText) as Record<string, unknown>;
      await createAITask({
        task_type: taskType,
        payload,
        model_name: modelName || undefined,
        cost_estimate: Number(costEstimate) || 0,
      });
      await refresh();
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to create AI task';
      setError(message);
    }
  };

  const onRetryTask = async (taskId: string) => {
    try {
      await retryAITask(taskId);
      await refresh();
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to retry task';
      setError(message);
    }
  };

  const onApplyFix = async (taskId: string) => {
    setApplyingTaskIds((prev) => ({ ...prev, [taskId]: true }));
    try {
      const result = await applyAITaskFix(taskId);
      setApplyResults((prev) => ({
        ...prev,
        [taskId]: { commit_sha: result.commit_sha, pr_url: result.pr_url },
      }));
      await loadTaskDetail(taskId);
      await refresh();
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to apply fix';
      setError(message);
    } finally {
      setApplyingTaskIds((prev) => ({ ...prev, [taskId]: false }));
    }
  };

  const loadTaskDetail = async (taskId: string) => {
    setDetailLoadingTaskId(taskId);
    try {
      const detail = await getAITask(taskId);
      setTaskDetails((prev) => ({ ...prev, [taskId]: detail }));
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load task detail';
      setError(message);
    } finally {
      setDetailLoadingTaskId(null);
    }
  };

  const loadBatchDetail = async (batchId: string) => {
    if (batchDetails[batchId] || loadingBatchIds[batchId]) {
      return;
    }
    setLoadingBatchIds((prev) => ({ ...prev, [batchId]: true }));
    try {
      const detail = await getAIFixBatch(batchId);
      setBatchDetails((prev) => ({ ...prev, [batchId]: detail }));
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load batch detail';
      setError(message);
    } finally {
      setLoadingBatchIds((prev) => ({ ...prev, [batchId]: false }));
    }
  };

  const onToggleReview = async (task: AITaskItem) => {
    if (expandedTaskId === task.id) {
      setExpandedTaskId(null);
      return;
    }
    setExpandedTaskId(task.id);
    if (!taskDetails[task.id]) {
      await loadTaskDetail(task.id);
    }
    if (task.batch_id) {
      await loadBatchDetail(task.batch_id);
    }
  };

  const getTaskCards = (task: AITaskItem): Array<{ card_id: string; set_id?: string; module_name?: string }> => {
    const fromSanitized = (task.sanitized_input as { cards?: unknown })?.cards;
    if (Array.isArray(fromSanitized)) {
      const cards = fromSanitized.filter((entry): entry is Record<string, unknown> => typeof entry === 'object' && entry !== null);
      if (cards.length > 0) {
        return cards.map((entry) => ({
          card_id: String(entry.card_id ?? ''),
          set_id: entry.set_id ? String(entry.set_id) : undefined,
          module_name: entry.module_name ? String(entry.module_name) : undefined,
        }));
      }
    }
    const fromPayload = (task.payload as { cards?: unknown })?.cards;
    if (!Array.isArray(fromPayload)) {
      return [];
    }
    return fromPayload
      .filter((entry): entry is Record<string, unknown> => typeof entry === 'object' && entry !== null)
      .map((entry) => ({
        card_id: String(entry.card_id ?? ''),
        set_id: entry.set_id ? String(entry.set_id) : undefined,
        module_name: entry.module_name ? String(entry.module_name) : undefined,
      }))
      .filter((entry) => entry.card_id);
  };

  const onPromoteFromTask = async (taskId: string, cardId: string) => {
    const key = `${taskId}:${cardId}`;
    setPromotingKeys((prev) => ({ ...prev, [key]: true }));
    try {
      await promoteTaskCard(taskId, {
        card_id: cardId,
        notes: promotionNotes[taskId] ?? '',
      });
      setPromotedKeys((prev) => ({ ...prev, [key]: true }));
      await loadTaskDetail(taskId);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to promote card';
      setError(message);
    } finally {
      setPromotingKeys((prev) => ({ ...prev, [key]: false }));
    }
  };

  return (
    <div className="max-w-6xl mx-auto p-6 space-y-6">
      <h1 className="text-2xl font-semibold text-white">Admin AI Tasks</h1>
      {error ? <div className="text-red-400 text-sm">{error}</div> : null}

      {/* Tab switcher */}
      <div className="flex gap-1 border-b border-gray-700 pb-0">
        <button
          onClick={() => setActiveTab('tasks')}
          className={`px-4 py-2 text-sm rounded-t border border-b-0 ${
            activeTab === 'tasks'
              ? 'bg-gray-800 text-white border-gray-700'
              : 'text-gray-400 hover:text-white border-transparent'
          }`}
        >
          Tasks
        </button>
        <button
          onClick={() => {
            setActiveTab('applied');
            void refreshApplied();
          }}
          className={`px-4 py-2 text-sm rounded-t border border-b-0 ${
            activeTab === 'applied'
              ? 'bg-gray-800 text-white border-gray-700'
              : 'text-gray-400 hover:text-white border-transparent'
          }`}
        >
          Applied Cards
        </button>
      </div>

      {/* ── Tasks Tab ─────────────────────────────────────────────── */}
      {activeTab === 'tasks' && (
        <>
          <div className="border border-gray-700 rounded p-4 bg-gray-900 space-y-3">
            <div className="text-sm text-gray-300 font-medium">Create Task</div>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
              <select
                value={taskType}
                onChange={(e) => setTaskType(e.target.value as typeof taskType)}
                className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
              >
                <option value="review_batch">review_batch</option>
                <option value="qa_analysis">qa_analysis</option>
                <option value="engine_audit">engine_audit</option>
                <option value="script_autofix">script_autofix</option>
              </select>
              <input
                value={modelName}
                onChange={(e) => setModelName(e.target.value)}
                placeholder="model override (optional)"
                className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
              />
              <input
                value={costEstimate}
                onChange={(e) => setCostEstimate(e.target.value)}
                placeholder="cost estimate USD"
                className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
              />
            </div>
            <textarea
              value={payloadText}
              onChange={(e) => setPayloadText(e.target.value)}
              rows={10}
              className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white font-mono"
            />
            <button
              onClick={() => void onCreateTask()}
              className="px-3 py-1.5 rounded bg-blue-600 hover:bg-blue-500 text-white text-sm"
            >
              Queue Task
            </button>
          </div>

          <div className="border border-gray-700 rounded p-4 bg-gray-900 space-y-3">
            <div className="text-sm text-gray-300 font-medium">Task Filters</div>
            <div className="grid grid-cols-1 md:grid-cols-4 gap-3">
              <select
                value={statusFilter}
                onChange={(e) => setStatusFilter(e.target.value as TaskStatusFilter)}
                className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
              >
                <option value="all">All statuses</option>
                <option value="queued">queued</option>
                <option value="running">running</option>
                <option value="completed">completed</option>
                <option value="failed">failed</option>
              </select>
              <select
                value={taskTypeFilter}
                onChange={(e) => setTaskTypeFilter(e.target.value as TaskTypeFilter)}
                className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
              >
                <option value="all">All task types</option>
                <option value="review_batch">review_batch</option>
                <option value="qa_analysis">qa_analysis</option>
                <option value="engine_audit">engine_audit</option>
                <option value="script_autofix">script_autofix</option>
              </select>
              <select
                value={runModeFilter}
                onChange={(e) => setRunModeFilter(e.target.value as RunModeFilter)}
                className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
              >
                <option value="all">All run modes</option>
                <option value="pr">pr</option>
                <option value="main">main</option>
                <option value="n/a">n/a</option>
              </select>
              <select
                value={scopeFilter}
                onChange={(e) => setScopeFilter(e.target.value as ScopeFilter)}
                className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
              >
                <option value="all">All scopes</option>
                <option value="script">script</option>
                <option value="script_engine">script_engine</option>
                <option value="script_engine_transpiler">script_engine_transpiler</option>
                <option value="n/a">n/a</option>
              </select>
              <input
                value={batchIdFilter}
                onChange={(e) => setBatchIdFilter(e.target.value)}
                placeholder="batch id contains..."
                className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
              />
              <input
                value={limitFilter}
                onChange={(e) => setLimitFilter(e.target.value)}
                placeholder="fetch limit (1-500)"
                className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
              />
              <label className="text-sm text-gray-300 flex items-center gap-2">
                <input type="checkbox" checked={batchOnlyFilter} onChange={(e) => setBatchOnlyFilter(e.target.checked)} />
                batch tasks only
              </label>
              <div className="text-sm text-gray-400 flex items-center">
                showing {filteredTasks.length} of {tasks.length}
              </div>
            </div>
            <div className="flex gap-2">
              <button
                onClick={() => void refresh()}
                className="px-3 py-1.5 rounded bg-blue-600 hover:bg-blue-500 text-white text-sm"
              >
                Apply / Refresh
              </button>
            </div>
          </div>

          <div className="text-xs text-gray-400">Total estimated spend: ${totalEstimated.toFixed(4)}</div>

          <div className="space-y-3">
            {loading ? <div className="text-gray-300">Loading...</div> : null}
            {filteredTasks.map((task) => {
              const detail = taskDetails[task.id];
              const batchDetail = task.batch_id ? batchDetails[task.batch_id] : null;
              const loadingBatch = task.batch_id ? loadingBatchIds[task.batch_id] === true : false;
              const workedItems = batchDetail?.items.filter((item) => item.status === 'applied') ?? [];
              const failedItems = batchDetail?.items.filter((item) => item.status === 'failed') ?? [];
              const applyResult = applyResults[task.id];

              return (
                <div key={task.id} className="border border-gray-700 rounded p-4 bg-gray-900">
                  <div className="flex justify-between items-center text-sm">
                    <div className="text-white">
                      {task.task_type} <span className="text-gray-400">({task.id})</span>
                    </div>
                    <StatusBadge status={task.status} />
                  </div>
                  <div className="text-xs text-gray-500 mt-1">
                    scope={task.scope_profile ?? 'n/a'} run_mode={task.run_mode ?? 'n/a'} batch={task.batch_id ?? 'n/a'}
                  </div>
                  <div className="text-xs text-gray-500 mt-1">
                    est=${Number(task.cost_estimate || 0).toFixed(4)} actual=${Number(task.cost_actual || 0).toFixed(4)}{' '}
                    in={task.input_tokens} out={task.output_tokens}
                  </div>
                  {batchDetail ? (
                    <div className="text-xs text-sky-300 mt-1">
                      batch_status={batchDetail.batch.status} worked={workedItems.length} failed={failedItems.length} commits=
                      {batchDetail.batch.commit_count}
                    </div>
                  ) : null}
                  {applyResult?.commit_sha || applyResult?.pr_url ? (
                    <div className="flex items-center gap-3 text-xs mt-1">
                      <span className="text-gray-400">Applied:</span>
                      <CommitLink sha={applyResult.commit_sha} />
                      <PrLink url={applyResult.pr_url} />
                    </div>
                  ) : null}
                  {task.error_text ? <div className="text-xs text-red-400 mt-2">{task.error_text}</div> : null}
                  <div className="flex gap-2 mt-3">
                    <button
                      onClick={() => void onRetryTask(task.id)}
                      className="px-2 py-1 rounded bg-slate-700 hover:bg-slate-600 text-xs text-white"
                    >
                      Retry
                    </button>
                    <button
                      onClick={() => void onToggleReview(task)}
                      className="px-2 py-1 rounded bg-indigo-700 hover:bg-indigo-600 text-xs text-white"
                    >
                      {expandedTaskId === task.id ? 'Hide Review' : 'Review Result'}
                    </button>
                    {task.task_type === 'script_autofix' ? (
                      <button
                        onClick={() => void onApplyFix(task.id)}
                        disabled={applyingTaskIds[task.id] === true}
                        className="px-2 py-1 rounded bg-emerald-700 hover:bg-emerald-600 disabled:bg-gray-700 text-xs text-white"
                      >
                        {applyingTaskIds[task.id] === true ? 'Applying...' : 'Apply Fix'}
                      </button>
                    ) : null}
                  </div>

                  {expandedTaskId === task.id ? (
                    <div className="mt-4 border border-gray-700 rounded p-3 bg-gray-950 space-y-3">
                      {detailLoadingTaskId === task.id && !detail ? (
                        <div className="text-xs text-gray-400">Loading task detail...</div>
                      ) : null}
                      {detail ? (
                        <>
                          <div className="text-xs text-gray-400">Stored Result JSON</div>
                          <pre className="text-xs text-gray-200 bg-gray-900 border border-gray-800 rounded p-3 overflow-auto max-h-56">
                            {JSON.stringify(detail.result ?? {}, null, 2)}
                          </pre>

                          {task.batch_id ? (
                            <div className="border border-gray-800 rounded p-3 bg-gray-900 space-y-2">
                              <div className="text-xs text-gray-300">Batch Execution Detail</div>
                              {loadingBatch ? <div className="text-xs text-gray-400">Loading batch detail...</div> : null}
                              {batchDetail ? (
                                <>
                                  <div className="text-[11px] text-gray-300">
                                    batch={batchDetail.batch.id} set={batchDetail.batch.set_id} status={batchDetail.batch.status}{' '}
                                    run_mode={batchDetail.batch.run_mode} scope={batchDetail.batch.scope_profile}
                                  </div>
                                  <div className="text-[11px] text-gray-400">
                                    applied={batchDetail.batch.applied_count} failed={batchDetail.batch.failed_count} commits=
                                    {batchDetail.batch.commit_count} stop_reason={batchDetail.batch.stopped_reason ?? 'n/a'}
                                  </div>
                                  <div className="text-[11px] text-gray-400 flex items-center gap-2">
                                    pr=<PrLink url={batchDetail.batch.pr_url} />
                                    {!batchDetail.batch.pr_url ? <span>n/a</span> : null}
                                  </div>
                                  <div className="text-[11px] text-emerald-300">
                                    worked_cards ({workedItems.length}):{' '}
                                    {workedItems.length > 0
                                      ? workedItems.map((item) => (
                                          <span key={item.id} className="inline-flex items-center gap-1 mr-2">
                                            {item.card_id}
                                            <CommitLink sha={item.commit_sha} />
                                          </span>
                                        ))
                                      : 'none'}
                                  </div>
                                  <div className="space-y-1">
                                    <div className="text-[11px] text-red-300">failed_cards ({failedItems.length})</div>
                                    {failedItems.length === 0 ? (
                                      <div className="text-[11px] text-gray-500">none</div>
                                    ) : (
                                      failedItems.map((item) => (
                                        <div key={item.id} className="text-[11px] text-red-200">
                                          {item.card_id}: {item.error_text ?? 'no error text'}
                                        </div>
                                      ))
                                    )}
                                  </div>
                                </>
                              ) : null}
                            </div>
                          ) : null}

                          {detail.task_type === 'review_batch' ? (
                            <>
                              <div className="text-xs text-gray-400">Promotion Note (optional)</div>
                              <input
                                value={promotionNotes[task.id] ?? ''}
                                onChange={(e) => setPromotionNotes((prev) => ({ ...prev, [task.id]: e.target.value }))}
                                placeholder="e.g. faithful + engine-supported; approved"
                                className="w-full bg-gray-900 border border-gray-700 rounded px-2 py-1.5 text-xs text-white"
                              />
                              <div className="space-y-2">
                                {getTaskCards(detail).map((card) => {
                                  const resultCards = (detail.result as { cards?: Record<string, Record<string, unknown>> } | null)?.cards ?? {};
                                  const review = resultCards[card.card_id] ?? {};
                                  const faithful = review.faithful_to_card_text === true;
                                  const engineSupported = review.engine_supported === true;
                                  const issues = Array.isArray(review.issues) ? review.issues.map((issue) => String(issue)) : [];
                                  const key = `${task.id}:${card.card_id}`;
                                  const promoting = promotingKeys[key] === true;
                                  const promoted = promotedKeys[key] === true;

                                  return (
                                    <div key={key} className="border border-gray-800 rounded p-2 bg-gray-900">
                                      <div className="flex items-center justify-between gap-2">
                                        <div className="text-xs text-white">
                                          {card.card_id}
                                          <span className="text-gray-400"> ({card.module_name ?? 'unknown module'})</span>
                                        </div>
                                        <button
                                          onClick={() => void onPromoteFromTask(task.id, card.card_id)}
                                          disabled={promoting || promoted}
                                          className="px-2 py-1 rounded bg-emerald-700 hover:bg-emerald-600 disabled:bg-gray-700 text-xs text-white"
                                        >
                                          {promoted ? 'Promoted' : promoting ? 'Promoting...' : 'Promote'}
                                        </button>
                                      </div>
                                      <div className="text-[11px] text-gray-400 mt-1">
                                        faithful={String(faithful)} engine_supported={String(engineSupported)}
                                      </div>
                                      {issues.length > 0 ? (
                                        <div className="text-[11px] text-amber-300 mt-1">issues: {issues.join(' | ')}</div>
                                      ) : null}
                                    </div>
                                  );
                                })}
                              </div>
                            </>
                          ) : null}
                        </>
                      ) : null}
                    </div>
                  ) : null}
                </div>
              );
            })}
          </div>
        </>
      )}

      {/* ── Applied Cards Tab ─────────────────────────────────────── */}
      {activeTab === 'applied' && (
        <>
          <div className="border border-gray-700 rounded p-4 bg-gray-900 space-y-3">
            <div className="text-sm text-gray-300 font-medium">Applied Cards Filters</div>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
              <select
                value={appliedStatusFilter}
                onChange={(e) => setAppliedStatusFilter(e.target.value as AppliedStatusFilter)}
                className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
              >
                <option value="all">All statuses</option>
                <option value="applied">applied</option>
                <option value="failed">failed</option>
              </select>
              <input
                value={appliedSetFilter}
                onChange={(e) => setAppliedSetFilter(e.target.value)}
                placeholder="set_id (e.g. BT24)"
                className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
              />
              <button
                onClick={() => void refreshApplied()}
                className="px-3 py-1.5 rounded bg-blue-600 hover:bg-blue-500 text-white text-sm"
              >
                Refresh
              </button>
            </div>
            <div className="text-xs text-gray-400">
              {appliedCards.length} card{appliedCards.length !== 1 ? 's' : ''} found
            </div>
          </div>

          {appliedLoading ? <div className="text-gray-300">Loading...</div> : null}

          {groupedApplied.map(([setId, cards]) => (
            <div key={setId} className="space-y-2">
              <div className="text-sm text-gray-300 font-medium border-b border-gray-700 pb-1">
                {setId} <span className="text-gray-500">({cards.length})</span>
              </div>
              <div className="overflow-x-auto">
                <table className="w-full text-xs">
                  <thead>
                    <tr className="text-gray-400 text-left">
                      <th className="px-2 py-1">Card ID</th>
                      <th className="px-2 py-1">Status</th>
                      <th className="px-2 py-1">Applied At</th>
                      <th className="px-2 py-1">Commit</th>
                      <th className="px-2 py-1">PR</th>
                      <th className="px-2 py-1">Source</th>
                      <th className="px-2 py-1">Error</th>
                    </tr>
                  </thead>
                  <tbody>
                    {cards.map((card) => (
                      <tr key={card.id} className="border-t border-gray-800 text-gray-200">
                        <td className="px-2 py-1.5 font-mono">{card.card_id}</td>
                        <td className="px-2 py-1.5">
                          <StatusBadge status={card.status} />
                        </td>
                        <td className="px-2 py-1.5 text-gray-400">
                          {new Date(card.created_at).toLocaleString()}
                        </td>
                        <td className="px-2 py-1.5">
                          <CommitLink sha={card.commit_sha} />
                        </td>
                        <td className="px-2 py-1.5">
                          <PrLink url={card.pr_url} />
                        </td>
                        <td className="px-2 py-1.5">
                          <span
                            className={`text-[10px] px-1.5 py-0.5 rounded ${
                              card.batch_id ? 'bg-blue-900/50 text-blue-300' : 'bg-purple-900/50 text-purple-300'
                            }`}
                          >
                            {card.batch_id ? 'batch' : 'manual'}
                          </span>
                        </td>
                        <td className="px-2 py-1.5 text-red-400 max-w-xs truncate">{card.error_text ?? ''}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          ))}
        </>
      )}
    </div>
  );
}
