import { useEffect, useMemo, useState } from 'react';
import axios from 'axios';
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
import { parseDeck } from '@/api/deckApi';

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

type TaskTypeFilter = 'all' | 'review_batch' | 'qa_analysis' | 'engine_audit' | 'script_autofix' | 'llm_transpile' | 'transpiler_learn';
type TaskStatusFilter = 'all' | 'queued' | 'running' | 'completed' | 'failed';
type RunModeFilter = 'all' | 'pr' | 'main' | 'n/a';
type ScopeFilter = 'all' | 'script' | 'script_engine' | 'script_engine_transpiler' | 'n/a';
type ActiveTab = 'tasks' | 'applied';
type ApplyStatusFilter = 'all' | 'applicable' | 'stale' | 'invalid';
type AppliedStatusFilter = 'all' | 'applied' | 'failed';
type TaskRunMode = 'pr' | 'main';
type TaskScopeProfile = 'script' | 'script_engine' | 'script_engine_transpiler';

type GroupStatus = 'queued' | 'running' | 'completed' | 'failed';

type TaskGroup = {
  workId: string;
  workType: AITaskItem['work_type'];
  setLabel: string;
  taskCount: number;
  cardCount: number;
  aggregateStatus: GroupStatus;
  tasks: AITaskItem[];
};

const STATUS_PRIORITY: Record<GroupStatus, number> = {
  failed: 4,
  running: 3,
  queued: 2,
  completed: 1,
};

function setLabelForGroup(tasks: AITaskItem[]): string {
  const values = new Set<string>();
  for (const task of tasks) {
    const value = task.set_id?.trim().toLowerCase();
    if (value) values.add(value);
  }
  if (values.size === 0) return 'n/a';
  if (values.size === 1) return Array.from(values)[0]!;
  return 'mixed';
}

function aggregateStatusForGroup(tasks: AITaskItem[]): GroupStatus {
  let selected: GroupStatus = 'completed';
  for (const task of tasks) {
    if (STATUS_PRIORITY[task.status] > STATUS_PRIORITY[selected]) {
      selected = task.status;
    }
  }
  return selected;
}

type ScriptAutofixCardPayload = {
  card_id: string;
  set_id: string;
  module_name: string;
};

type StaleApplyConflict = {
  code: string;
  message: string;
  path: string;
  expected_hash: string;
  current_hash: string;
  can_force_apply: boolean;
};

function parseErrorMessage(err: unknown): string {
  if (axios.isAxiosError(err)) {
    const detail = err.response?.data?.detail;
    if (typeof detail === 'string' && detail.trim()) {
      return detail;
    }
    if (detail && typeof detail === 'object') {
      const message = (detail as { message?: unknown }).message;
      if (typeof message === 'string' && message.trim()) {
        return message;
      }
    }
    if (typeof err.message === 'string' && err.message.trim()) {
      return err.message;
    }
  }
  if (err instanceof Error) {
    return err.message;
  }
  return 'Request failed';
}

function extractStaleApplyConflict(err: unknown): StaleApplyConflict | null {
  if (!axios.isAxiosError(err)) {
    return null;
  }
  const detail = err.response?.data?.detail;
  if (!detail || typeof detail !== 'object') {
    return null;
  }
  const typed = detail as Partial<StaleApplyConflict>;
  if (
    typed.code !== 'stale_expected_hash' ||
    typeof typed.message !== 'string' ||
    typeof typed.path !== 'string' ||
    typeof typed.expected_hash !== 'string' ||
    typeof typed.current_hash !== 'string'
  ) {
    return null;
  }
  return {
    code: typed.code,
    message: typed.message,
    path: typed.path,
    expected_hash: typed.expected_hash,
    current_hash: typed.current_hash,
    can_force_apply: typed.can_force_apply === true,
  };
}

function toScriptAutofixCardPayload(cardId: string): ScriptAutofixCardPayload | null {
  const normalized = cardId.trim().toUpperCase();
  if (!normalized) {
    return null;
  }
  const splitIndex = normalized.indexOf('-');
  if (splitIndex <= 0 || splitIndex >= normalized.length - 1) {
    return null;
  }

  const setId = normalized.slice(0, splitIndex).toLowerCase();
  const rawSuffix = normalized.slice(splitIndex + 1).trim();
  const suffix = rawSuffix.replace(/[^A-Z0-9]+/gi, '_').replace(/^_+|_+$/g, '').toLowerCase();
  if (!setId || !suffix) {
    return null;
  }
  return {
    card_id: normalized,
    set_id: setId,
    module_name: `${setId}_${suffix}`,
  };
}

export function AdminTasksPage() {
  const [tasks, setTasks] = useState<AITaskItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [detailLoadingTaskId, setDetailLoadingTaskId] = useState<string | null>(null);
  const [expandedTaskId, setExpandedTaskId] = useState<string | null>(null);
  const [taskDetails, setTaskDetails] = useState<Record<string, AITaskItem>>({});
  const [batchDetails, setBatchDetails] = useState<Record<string, AIFixBatchDetailResponse>>({});
  const [loadingBatchIds, setLoadingBatchIds] = useState<Record<string, boolean>>({});
  const [collapsedGroups, setCollapsedGroups] = useState<Record<string, boolean>>({});
  const [promotingKeys, setPromotingKeys] = useState<Record<string, boolean>>({});
  const [promotedKeys, setPromotedKeys] = useState<Record<string, boolean>>({});
  const [applyingTaskIds, setApplyingTaskIds] = useState<Record<string, boolean>>({});
  const [promotionNotes, setPromotionNotes] = useState<Record<string, string>>({});
  const [error, setError] = useState<string | null>(null);

  const [taskType, setTaskType] = useState<'review_batch' | 'qa_analysis' | 'engine_audit' | 'script_autofix' | 'llm_transpile' | 'transpiler_learn'>('review_batch');
  const [payloadText, setPayloadText] = useState(DEFAULT_PAYLOAD);
  const [modelName, setModelName] = useState('');
  const [costEstimate, setCostEstimate] = useState('0.5');

  const [statusFilter, setStatusFilter] = useState<TaskStatusFilter>('all');
  const [taskTypeFilter, setTaskTypeFilter] = useState<TaskTypeFilter>('all');
  const [runModeFilter, setRunModeFilter] = useState<RunModeFilter>('all');
  const [scopeFilter, setScopeFilter] = useState<ScopeFilter>('all');
  const [applyStatusFilter, setApplyStatusFilter] = useState<ApplyStatusFilter>('all');
  const [setIdFilter, setSetIdFilter] = useState('');
  const [workIdFilter, setWorkIdFilter] = useState('');
  const [limitFilter, setLimitFilter] = useState('250');
  const [deckListText, setDeckListText] = useState('');
  const [deckRunMode, setDeckRunMode] = useState<TaskRunMode>('pr');
  const [deckScopeProfile, setDeckScopeProfile] = useState<TaskScopeProfile>('script');
  const [deckQueueLimit, setDeckQueueLimit] = useState('60');
  const [deckQueueing, setDeckQueueing] = useState(false);
  const [deckQueueSummary, setDeckQueueSummary] = useState<string | null>(null);
  const [deckParseWarnings, setDeckParseWarnings] = useState<string[]>([]);
  const [createTaskAccordionOpen, setCreateTaskAccordionOpen] = useState(true);
  const [taskFiltersAccordionOpen, setTaskFiltersAccordionOpen] = useState(true);
  const [deckQueueAccordionOpen, setDeckQueueAccordionOpen] = useState(false);

  // Applied Cards tab state
  const [activeTab, setActiveTab] = useState<ActiveTab>('tasks');
  const [appliedCards, setAppliedCards] = useState<AppliedCardItem[]>([]);
  const [appliedLoading, setAppliedLoading] = useState(false);
  const [appliedStatusFilter, setAppliedStatusFilter] = useState<AppliedStatusFilter>('all');
  const [appliedSetFilter, setAppliedSetFilter] = useState('');

  // Store apply-fix results for showing git links
  const [applyResults, setApplyResults] = useState<Record<string, { commit_sha?: string | null; pr_url?: string | null }>>({});
  const [staleApplyConflicts, setStaleApplyConflicts] = useState<Record<string, StaleApplyConflict>>({});

  const refresh = async () => {
    setLoading(true);
    setError(null);
    try {
      const limit = Math.min(500, Math.max(1, Number(limitFilter) || 250));
      const params: {
        status?: 'queued' | 'running' | 'completed' | 'failed';
        task_type?: 'review_batch' | 'qa_analysis' | 'engine_audit' | 'script_autofix' | 'llm_transpile' | 'transpiler_learn';
        run_mode?: 'pr' | 'main';
        scope_profile?: 'script' | 'script_engine' | 'script_engine_transpiler';
        set_id?: string;
        work_id?: string;
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
      if (setIdFilter.trim()) {
        params.set_id = setIdFilter.trim();
      }
      if (workIdFilter.trim()) {
        params.work_id = workIdFilter.trim();
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
    const normalizedSetId = setIdFilter.trim().toLowerCase();
    const normalizedWorkId = workIdFilter.trim().toLowerCase();
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
      if (scopeFilter !== 'all' && scopeFilter !== 'n/a' && task.scope_profile !== scopeFilter) {
        return false;
      }
      if (normalizedSetId) {
        const value = task.set_id?.toLowerCase();
        if (!value || value !== normalizedSetId) {
          return false;
        }
      }
      if (normalizedWorkId) {
        const value = String(task.work_id ?? '').toLowerCase();
        if (!value.includes(normalizedWorkId)) {
          return false;
        }
      }
      if (applyStatusFilter !== 'all') {
        const taskApplyStatus = task.apply_status ?? null;
        if (taskApplyStatus !== applyStatusFilter) {
          return false;
        }
      }
      return true;
    });
  }, [tasks, statusFilter, taskTypeFilter, runModeFilter, scopeFilter, setIdFilter, workIdFilter, applyStatusFilter]);

  const groupedTasks = useMemo(() => {
    const groups: Record<string, AITaskItem[]> = {};
    for (const task of filteredTasks) {
      const workId = task.work_id || task.id;
      if (!groups[workId]) {
        groups[workId] = [];
      }
      groups[workId].push(task);
    }
    const result: TaskGroup[] = Object.entries(groups).map(([workId, groupItems]) => {
      const cards = new Set<string>();
      for (const task of groupItems) {
        for (const card of getTaskCards(task)) {
          cards.add(card.card_id);
        }
      }
      return {
        workId,
        workType: groupItems[0]?.work_type ?? 'task',
        setLabel: setLabelForGroup(groupItems),
        taskCount: groupItems.length,
        cardCount: cards.size,
        aggregateStatus: aggregateStatusForGroup(groupItems),
        tasks: groupItems,
      };
    });
    result.sort((a, b) => {
      const left = Math.max(...a.tasks.map((t) => new Date(t.created_at).getTime()));
      const right = Math.max(...b.tasks.map((t) => new Date(t.created_at).getTime()));
      return right - left;
    });
    return result;
  }, [filteredTasks]);

  useEffect(() => {
    setCollapsedGroups((prev) => {
      const next = { ...prev };
      for (const group of groupedTasks) {
        if (!(group.workId in next)) {
          next[group.workId] = false;
        }
      }
      return next;
    });
  }, [groupedTasks]);

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
      setStaleApplyConflicts((prev) => {
        const next = { ...prev };
        delete next[taskId];
        return next;
      });
      await refresh();
    } catch (err) {
      const message = parseErrorMessage(err);
      setError(message);
    }
  };

  const onApplyFix = async (taskId: string, force = false) => {
    setApplyingTaskIds((prev) => ({ ...prev, [taskId]: true }));
    try {
      const result = await applyAITaskFix(taskId, { force });
      setApplyResults((prev) => ({
        ...prev,
        [taskId]: { commit_sha: result.commit_sha, pr_url: result.pr_url },
      }));
      setStaleApplyConflicts((prev) => {
        const next = { ...prev };
        delete next[taskId];
        return next;
      });
      await loadTaskDetail(taskId);
      await refresh();
    } catch (err) {
      const stale = extractStaleApplyConflict(err);
      if (stale) {
        setStaleApplyConflicts((prev) => ({ ...prev, [taskId]: stale }));
      }
      const message = stale?.message ?? parseErrorMessage(err);
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

  function getTaskCards(task: AITaskItem): Array<{ card_id: string; set_id?: string; module_name?: string }> {
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
      const fallbackCardId = (task.payload as { card_id?: unknown }).card_id;
      if (typeof fallbackCardId !== 'string' || !fallbackCardId.trim()) {
        return [];
      }
      return [
        {
          card_id: fallbackCardId.trim().toUpperCase(),
          set_id: task.set_id ?? undefined,
          module_name:
            typeof (task.payload as { module_name?: unknown }).module_name === 'string'
              ? String((task.payload as { module_name?: unknown }).module_name)
              : undefined,
        },
      ];
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
  }

  const renderTaskCard = (task: AITaskItem) => {
    const detail = taskDetails[task.id];
    const batchDetail = task.batch_id ? batchDetails[task.batch_id] : null;
    const loadingBatch = task.batch_id ? loadingBatchIds[task.batch_id] === true : false;
    const workedItems = batchDetail?.items.filter((item) => item.status === 'applied') ?? [];
    const failedItems = batchDetail?.items.filter((item) => item.status === 'failed') ?? [];
    const applyResult = applyResults[task.id];
    const staleConflict = staleApplyConflicts[task.id];

    return (
      <div key={task.id} className="border border-gray-700 rounded p-4 bg-gray-900">
        <div className="flex justify-between items-center text-sm">
          <div className="text-white">
            {task.task_type} <span className="text-gray-400">({task.id})</span>
          </div>
          <StatusBadge status={task.status} />
        </div>
        <div className="text-xs text-gray-500 mt-1">
          scope={task.scope_profile ?? 'n/a'} run_mode={task.run_mode ?? 'n/a'} batch={task.batch_id ?? 'n/a'} set_run=
          {task.set_run_id ?? 'n/a'} set={task.set_id ?? 'n/a'}
        </div>
        <div className="text-xs text-gray-500 mt-1">
          est=${Number(task.cost_estimate || 0).toFixed(4)} actual=${Number(task.cost_actual || 0).toFixed(4)} in=
          {task.input_tokens} out={task.output_tokens}
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
          {task.task_type === 'script_autofix' && task.apply_status === 'stale' ? (
            <span className="px-2 py-1 rounded bg-amber-800/60 text-amber-300 text-xs font-medium">Stale</span>
          ) : null}
          {task.task_type === 'script_autofix' && task.apply_status === 'invalid' ? (
            <span className="px-2 py-1 rounded bg-red-800/60 text-red-300 text-xs font-medium">Invalid</span>
          ) : null}
          {task.task_type === 'script_autofix' && task.apply_status !== 'invalid' ? (
            <button
              onClick={() => void onApplyFix(task.id)}
              disabled={applyingTaskIds[task.id] === true}
              className="px-2 py-1 rounded bg-emerald-700 hover:bg-emerald-600 disabled:bg-gray-700 text-xs text-white"
            >
              {applyingTaskIds[task.id] === true ? 'Applying...' : 'Apply Fix'}
            </button>
          ) : null}
          {task.task_type === 'script_autofix' && (task.apply_status === 'stale' || staleConflict?.can_force_apply) ? (
            <button
              onClick={() => void onApplyFix(task.id, true)}
              disabled={applyingTaskIds[task.id] === true}
              className="px-2 py-1 rounded bg-amber-700 hover:bg-amber-600 disabled:bg-gray-700 text-xs text-white"
            >
              {applyingTaskIds[task.id] === true ? 'Applying...' : 'Force Apply'}
            </button>
          ) : null}
        </div>
        {staleConflict ? (
          <div className="mt-2 border border-amber-700/60 rounded p-2 bg-amber-950/20 text-[11px] text-amber-200 space-y-1">
            <div>{staleConflict.message}</div>
            <div>path={staleConflict.path}</div>
            <div>expected={staleConflict.expected_hash}</div>
            <div>current={staleConflict.current_hash}</div>
            {!staleConflict.can_force_apply ? <div>Force apply is unavailable for this mismatch.</div> : null}
          </div>
        ) : null}

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
  };

  const onQueueDeckImplementationTasks = async () => {
    if (!deckListText.trim()) {
      setError('Paste a deck list first.');
      return;
    }

    setDeckQueueing(true);
    setDeckQueueSummary(null);
    setDeckParseWarnings([]);
    setError(null);

    try {
      const parsed = await parseDeck(deckListText);
      setDeckParseWarnings(parsed.warnings ?? []);

      const uniqueCards = Array.from(
        new Set(
          [...parsed.main_deck, ...parsed.egg_deck]
            .map((cardId) => cardId.trim().toUpperCase())
            .filter((cardId) => cardId.length > 0),
        ),
      );

      const limit = Math.min(500, Math.max(1, Number(deckQueueLimit) || 60));
      const cardsToQueue = uniqueCards.slice(0, limit);
      const queueable: Array<{ cardId: string; payload: ScriptAutofixCardPayload }> = [];
      const skipped: string[] = [];
      for (const cardId of cardsToQueue) {
        const payload = toScriptAutofixCardPayload(cardId);
        if (!payload) {
          skipped.push(cardId);
          continue;
        }
        queueable.push({ cardId, payload });
      }

      if (queueable.length === 0) {
        throw new Error('No recognizable card IDs found in the parsed deck list.');
      }

      const estimate = Number(costEstimate) || 0;
      let created = 0;
      const failed: string[] = [];
      let firstError: string | null = null;
      for (const entry of queueable) {
        try {
          await createAITask({
            task_type: 'script_autofix',
            payload: {
              ...entry.payload,
              scope_profile: deckScopeProfile,
              issue_description: `Implement card script for ${entry.cardId} from submitted deck list.`,
            },
            model_name: modelName || undefined,
            cost_estimate: estimate,
            run_mode: deckRunMode,
            scope_profile: deckScopeProfile,
          });
          created += 1;
        } catch (err) {
          failed.push(entry.cardId);
          if (!firstError) {
            firstError = err instanceof Error ? err.message : 'Failed to queue some deck cards.';
          }
        }
      }

      await refresh();
      const summaryParts = [
        `Queued ${created} script_autofix task${created === 1 ? '' : 's'}`,
        `from ${cardsToQueue.length} unique deck cards`,
      ];
      if (skipped.length > 0) {
        summaryParts.push(`(${skipped.length} skipped: unsupported id format)`);
      }
      if (failed.length > 0) {
        summaryParts.push(`(${failed.length} failed to queue)`);
      }
      const summary = `${summaryParts.join(' ')}.`;
      setDeckQueueSummary(summary);
      if (firstError && created === 0) {
        setError(firstError);
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to parse deck list and queue tasks';
      setError(message);
    } finally {
      setDeckQueueing(false);
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

      {/* -- Tasks Tab ----------------------------------------------- */}
      {activeTab === 'tasks' && (
        <>
          <div className="border border-gray-700 rounded bg-gray-900">
            <button
              onClick={() => setCreateTaskAccordionOpen((prev) => !prev)}
              className="w-full px-4 py-3 flex items-center justify-between text-left hover:bg-gray-800/40"
            >
              <span className="text-sm text-gray-300 font-medium">Create Task</span>
              <span className="text-xs text-gray-400">{createTaskAccordionOpen ? 'Hide' : 'Show'}</span>
            </button>
            {createTaskAccordionOpen ? (
              <div className="px-4 pb-4 space-y-3 border-t border-gray-800">
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
                    <option value="llm_transpile">llm_transpile</option>
                    <option value="transpiler_learn">transpiler_learn</option>
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
            ) : null}
          </div>

          <div className="border border-gray-700 rounded bg-gray-900">
            <button
              onClick={() => setDeckQueueAccordionOpen((prev) => !prev)}
              className="w-full px-4 py-3 flex items-center justify-between text-left hover:bg-gray-800/40"
            >
              <span className="text-sm text-gray-300 font-medium">Deck List to Implementation Tasks</span>
              <span className="text-xs text-gray-400">{deckQueueAccordionOpen ? 'Hide' : 'Show'}</span>
            </button>
            {deckQueueAccordionOpen ? (
              <div className="px-4 pb-4 space-y-3 border-t border-gray-800">
                <textarea
                  value={deckListText}
                  onChange={(e) => setDeckListText(e.target.value)}
                  rows={8}
                  placeholder="Paste deck list text (digimoncard.io export or card-id list)"
                  className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white font-mono"
                />
                <div className="grid grid-cols-1 md:grid-cols-4 gap-3">
                  <select
                    value={deckRunMode}
                    onChange={(e) => setDeckRunMode(e.target.value as TaskRunMode)}
                    className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
                  >
                    <option value="pr">run_mode: pr</option>
                    <option value="main">run_mode: main</option>
                  </select>
                  <select
                    value={deckScopeProfile}
                    onChange={(e) => setDeckScopeProfile(e.target.value as TaskScopeProfile)}
                    className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
                  >
                    <option value="script">scope: script</option>
                    <option value="script_engine">scope: script_engine</option>
                    <option value="script_engine_transpiler">scope: script_engine_transpiler</option>
                  </select>
                  <input
                    value={deckQueueLimit}
                    onChange={(e) => setDeckQueueLimit(e.target.value)}
                    placeholder="max unique cards (1-500)"
                    className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
                  />
                  <button
                    onClick={() => void onQueueDeckImplementationTasks()}
                    disabled={deckQueueing}
                    className="px-3 py-1.5 rounded bg-emerald-700 hover:bg-emerald-600 disabled:bg-gray-700 text-white text-sm"
                  >
                    {deckQueueing ? 'Queueing...' : 'Queue Deck Implementation'}
                  </button>
                </div>
                {deckQueueSummary ? <div className="text-xs text-emerald-300">{deckQueueSummary}</div> : null}
                {deckParseWarnings.length > 0 ? (
                  <div className="text-xs text-amber-300">
                    Parser warnings: {deckParseWarnings.slice(0, 3).join(' | ')}
                    {deckParseWarnings.length > 3 ? ` | +${deckParseWarnings.length - 3} more` : ''}
                  </div>
                ) : null}
              </div>
            ) : null}
          </div>

          <div className="border border-gray-700 rounded bg-gray-900">
            <button
              onClick={() => setTaskFiltersAccordionOpen((prev) => !prev)}
              className="w-full px-4 py-3 flex items-center justify-between text-left hover:bg-gray-800/40"
            >
              <span className="text-sm text-gray-300 font-medium">Task Filters</span>
              <span className="text-xs text-gray-400">{taskFiltersAccordionOpen ? 'Hide' : 'Show'}</span>
            </button>
            {taskFiltersAccordionOpen ? (
              <div className="px-4 pb-4 space-y-3 border-t border-gray-800">
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
                    <option value="llm_transpile">llm_transpile</option>
                    <option value="transpiler_learn">transpiler_learn</option>
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
                  <select
                    value={applyStatusFilter}
                    onChange={(e) => setApplyStatusFilter(e.target.value as ApplyStatusFilter)}
                    className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
                  >
                    <option value="all">All apply statuses</option>
                    <option value="applicable">applicable</option>
                    <option value="stale">stale</option>
                    <option value="invalid">invalid</option>
                  </select>
                  <input
                    value={setIdFilter}
                    onChange={(e) => setSetIdFilter(e.target.value)}
                    placeholder="set id (e.g. bt24)"
                    className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
                  />
                  <input
                    value={workIdFilter}
                    onChange={(e) => setWorkIdFilter(e.target.value)}
                    placeholder="work id"
                    className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
                  />
                  <input
                    value={limitFilter}
                    onChange={(e) => setLimitFilter(e.target.value)}
                    placeholder="fetch limit (1-500)"
                    className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
                  />
                  <div className="text-sm text-gray-400 flex items-center">
                    showing {groupedTasks.length} groups / {filteredTasks.length} tasks
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
            ) : null}
          </div>

          <div className="text-xs text-gray-400">Total estimated spend: ${totalEstimated.toFixed(4)}</div>

          <div className="space-y-3">
            {loading ? <div className="text-gray-300">Loading...</div> : null}
            {!loading && groupedTasks.length === 0 ? <div className="text-sm text-gray-400">No tasks found.</div> : null}
            {groupedTasks.map((group) => {
              const isCollapsed = collapsedGroups[group.workId] === true;
              return (
                <div key={group.workId} className="border border-gray-700 rounded bg-gray-900">
                  <button
                    onClick={() => setCollapsedGroups((prev) => ({ ...prev, [group.workId]: !isCollapsed }))}
                    className="w-full px-4 py-3 flex items-center justify-between gap-3 text-left hover:bg-gray-800/40"
                  >
                    <div className="space-y-1">
                      <div className="text-sm text-white font-medium flex items-center gap-2">
                        <span>Work ID:</span>
                        <span className="font-mono">{group.workId}</span>
                      </div>
                      <div className="text-xs text-gray-400 flex items-center gap-2 flex-wrap">
                        <span className="px-1.5 py-0.5 rounded bg-slate-700 text-gray-100">{group.workType}</span>
                        <span className="px-1.5 py-0.5 rounded bg-slate-800 text-gray-200">set {group.setLabel}</span>
                        <span>{group.taskCount} tasks</span>
                        <span>{group.cardCount} cards</span>
                      </div>
                    </div>
                    <div className="flex items-center gap-3">
                      <StatusBadge status={group.aggregateStatus} />
                      <span className="text-xs text-gray-300">{isCollapsed ? 'Expand' : 'Collapse'}</span>
                    </div>
                  </button>
                  {!isCollapsed ? <div className="px-4 pb-4 space-y-3">{group.tasks.map((task) => renderTaskCard(task))}</div> : null}
                </div>
              );
            })}
          </div>
        </>
      )}

      {/* -- Applied Cards Tab --------------------------------------- */}
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

