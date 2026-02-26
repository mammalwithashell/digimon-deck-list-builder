import { useEffect, useMemo, useState } from 'react';
import {
  approveSetIssues,
  cancelAIFixBatch,
  cancelSetRun,
  createAIFixBatch,
  createSetRun,
  getAIFixBatch,
  getAIFixBatches,
  getSetRun,
  getSetRuns,
  getIssues,
  previewAIFixBatch,
  queueSetIssueFixes,
  queueIssueFix,
  type AISetRun,
  type AISetRunDetailResponse,
  type AIFixBatch,
  type AIFixBatchDetailResponse,
  type AIFixBatchCreateResponse,
  type IssueItem,
  type QueueSetIssueFixesResponse,
  updateIssue,
} from '@/api/adminApi';

const AVAILABLE_MODEL_OPTIONS = ['gpt-4.1', 'gpt-4.1-mini'] as const;
type IssuesViewTab = 'issues' | 'batches' | 'set_runs';

const TAB_HELP_TEXT: Record<IssuesViewTab, string> = {
  issues:
    'Use this tab to triage issues, approve cards for AI, queue single fixes, or bulk-queue fixes for a set while preventing duplicate card queueing.',
  batches:
    'Use this tab to run script_autofix batches for a set, preview eligible cards, and monitor per-card batch outcomes and commit status.',
  set_runs:
    'Use this tab to run full set workflows (QA -> review -> fix), then monitor stage progress, failures, and generated PR queue entries.',
};

export function AdminIssuesPage() {
  const [issues, setIssues] = useState<IssueItem[]>([]);
  const [batches, setBatches] = useState<AIFixBatch[]>([]);
  const [selectedBatchId, setSelectedBatchId] = useState<string>('');
  const [selectedBatch, setSelectedBatch] = useState<AIFixBatchDetailResponse | null>(null);
  const [setRuns, setSetRuns] = useState<AISetRun[]>([]);
  const [selectedSetRunId, setSelectedSetRunId] = useState<string>('');
  const [selectedSetRun, setSelectedSetRun] = useState<AISetRunDetailResponse | null>(null);
  const [preview, setPreview] = useState<AIFixBatchCreateResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [setId, setSetId] = useState('bt24');
  const [runMode, setRunMode] = useState<'pr' | 'main'>('pr');
  const [scopeProfile, setScopeProfile] = useState<'script' | 'script_engine' | 'script_engine_transpiler'>('script');
  const [modelName, setModelName] = useState('');
  const [concurrency, setConcurrency] = useState('4');
  const [maxTotalCostUsd, setMaxTotalCostUsd] = useState('5.0');
  const [failureRateStop, setFailureRateStop] = useState('0.3');
  const [maxTasks, setMaxTasks] = useState('0');
  const [dryRun, setDryRun] = useState(true);
  const [activeTab, setActiveTab] = useState<IssuesViewTab>('issues');
  const [tabHelpOpen, setTabHelpOpen] = useState<Record<IssuesViewTab, boolean>>({
    issues: true,
    batches: true,
    set_runs: true,
  });
  const [issueSetFilter, setIssueSetFilter] = useState('all');
  const [issueStatusFilter, setIssueStatusFilter] = useState<'all' | IssueItem['status']>('all');
  const [issueSeverityFilter, setIssueSeverityFilter] = useState<'all' | IssueItem['severity']>('all');
  const [issueSourceFilter, setIssueSourceFilter] = useState<'all' | IssueItem['source']>('all');
  const [issueSearch, setIssueSearch] = useState('');
  const [issueQueueMode, setIssueQueueMode] = useState<'approved_only' | 'new_and_approved'>('approved_only');
  const [expandedIssue, setExpandedIssue] = useState<IssueItem | null>(null);

  const issueSetOptions = useMemo(() => {
    const sets = new Set<string>();
    for (const issue of issues) {
      const prefix = issue.card_id.split('-', 1)[0]?.toLowerCase();
      if (prefix) {
        sets.add(prefix);
      }
    }
    return Array.from(sets).sort();
  }, [issues]);

  const filteredIssues = useMemo(() => {
    const search = issueSearch.trim().toLowerCase();
    return issues.filter((issue) => {
      const statusPass = issueStatusFilter === 'all' || issue.status === issueStatusFilter;
      const setPrefix = issue.card_id.split('-', 1)[0]?.toLowerCase() ?? '';
      const setPass = issueSetFilter === 'all' || setPrefix === issueSetFilter;
      const severityPass = issueSeverityFilter === 'all' || issue.severity === issueSeverityFilter;
      const sourcePass = issueSourceFilter === 'all' || issue.source === issueSourceFilter;
      const searchPass =
        !search ||
        issue.card_id.toLowerCase().includes(search) ||
        issue.description.toLowerCase().includes(search) ||
        issue.triage_notes.toLowerCase().includes(search) ||
        issue.resolution_notes.toLowerCase().includes(search);
      return statusPass && setPass && severityPass && sourcePass && searchPass;
    });
  }, [issues, issueSetFilter, issueStatusFilter, issueSeverityFilter, issueSourceFilter, issueSearch]);

  useEffect(() => {
    if (!expandedIssue) {
      return;
    }
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setExpandedIssue(null);
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [expandedIssue]);

  const refresh = async () => {
    setLoading(true);
    setError(null);
    try {
      const [issueData, batchData, setRunData] = await Promise.all([getIssues(), getAIFixBatches(), getSetRuns({ limit: 50 })]);
      setIssues(issueData);
      setBatches(batchData);
      setSetRuns(setRunData);
      const firstBatch = batchData[0];
      if (!selectedBatchId && firstBatch) {
        setSelectedBatchId(firstBatch.id);
      }
      const firstSetRun = setRunData[0];
      if (!selectedSetRunId && firstSetRun) {
        setSelectedSetRunId(firstSetRun.id);
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load admin data';
      setError(message);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void refresh();
  }, []);

  useEffect(() => {
    if (!selectedBatchId) {
      setSelectedBatch(null);
      return;
    }
    let ignore = false;
    const load = async () => {
      try {
        const detail = await getAIFixBatch(selectedBatchId);
        if (!ignore) {
          setSelectedBatch(detail);
        }
      } catch (err) {
        if (!ignore) {
          const message = err instanceof Error ? err.message : 'Failed to load batch detail';
          setError(message);
        }
      }
    };
    void load();
    return () => {
      ignore = true;
    };
  }, [selectedBatchId]);

  useEffect(() => {
    if (!selectedSetRunId) {
      setSelectedSetRun(null);
      return;
    }
    let ignore = false;
    const load = async () => {
      try {
        const detail = await getSetRun(selectedSetRunId);
        if (!ignore) {
          setSelectedSetRun(detail);
        }
      } catch (err) {
        if (!ignore) {
          const message = err instanceof Error ? err.message : 'Failed to load set run detail';
          setError(message);
        }
      }
    };
    void load();
    return () => {
      ignore = true;
    };
  }, [selectedSetRunId]);

  const setIssueStatus = async (issue: IssueItem, status: IssueItem['status']) => {
    try {
      const updated = await updateIssue(issue.id, { status });
      setIssues((prev) => prev.map((it) => (it.id === updated.id ? updated : it)));
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to update issue';
      setError(message);
    }
  };

  const onQueueIssueFix = async (issueId: string) => {
    try {
      await queueIssueFix(issueId);
      await refresh();
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to queue issue fix';
      setError(message);
    }
  };

  const onQueueSetIssueFixes = async () => {
    try {
      const result: QueueSetIssueFixesResponse = await queueSetIssueFixes({
        set_id: setId,
        queue_mode: issueQueueMode,
      });
      const skippedList = result.skipped_cards.join(', ');
      setActionMessage(
        `Queued ${result.queued_tasks}/${result.matched_issues} fixes for ${result.set_id.toUpperCase()} (${result.queue_mode}). ` +
          `Skipped duplicate cards=${result.skipped_duplicate_cards}, skipped existing-task cards=${result.skipped_existing_task_cards}` +
          (skippedList ? ` | skipped: ${skippedList}` : ''),
      );
      await refresh();
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to queue set issue fixes';
      setError(message);
    }
  };

  const onPreviewBatch = async () => {
    try {
      const data = await previewAIFixBatch({
        set_id: setId,
        max_tasks: Number(maxTasks) || 0,
      });
      setPreview(data);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to preview batch';
      setError(message);
    }
  };

  const onCreateBatch = async () => {
    try {
      const data = await createAIFixBatch({
        set_id: setId,
        run_mode: runMode,
        scope_profile: scopeProfile,
        model_name: modelName || undefined,
        concurrency: Number(concurrency) || 4,
        max_total_cost_usd: Number(maxTotalCostUsd) || 0,
        failure_rate_stop: Number(failureRateStop) || 0.3,
        max_tasks: Number(maxTasks) || 0,
        dry_run: dryRun,
      });
      setPreview(data);
      await refresh();
      if (data.batch?.id) {
        setSelectedBatchId(data.batch.id);
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to create batch';
      setError(message);
    }
  };

  const onApproveSet = async () => {
    try {
      const result = await approveSetIssues({
        set_id: setId,
        status_filter: 'new',
      });
      setActionMessage(
        `Approved ${result.approved_count}/${result.matched_count} issues for ${result.set_id.toUpperCase()} (skipped ${result.skipped_count}).`,
      );
      await refresh();
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to approve set issues';
      setError(message);
    }
  };

  const onCreateSetRun = async () => {
    try {
      const run = await createSetRun({
        set_id: setId,
        run_mode: runMode,
        scope_profile: scopeProfile,
        model_name: modelName || undefined,
        max_total_cost_usd: Number(maxTotalCostUsd) || 5.0,
        failure_rate_stop: Number(failureRateStop) || 0.3,
        max_fix_tasks: Number(maxTasks) || 0,
      });
      setActionMessage(`Started set run ${run.id} for ${run.set_id.toUpperCase()}.`);
      await refresh();
      setSelectedSetRunId(run.id);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to start set run';
      setError(message);
    }
  };

  const onCancelBatch = async (batchId: string) => {
    try {
      await cancelAIFixBatch(batchId);
      await refresh();
      if (selectedBatchId === batchId) {
        const detail = await getAIFixBatch(batchId);
        setSelectedBatch(detail);
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to cancel batch';
      setError(message);
    }
  };

  const onCancelSetRun = async (setRunId: string) => {
    try {
      await cancelSetRun(setRunId);
      await refresh();
      if (selectedSetRunId === setRunId) {
        const detail = await getSetRun(setRunId);
        setSelectedSetRun(detail);
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to cancel set run';
      setError(message);
    }
  };

  return (
    <div className="max-w-6xl mx-auto p-6 space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold text-white">Admin Issues</h1>
        <button
          onClick={() => void refresh()}
          title="Reload issues, batches, and set runs from the server"
          className="px-3 py-1.5 rounded bg-blue-600 hover:bg-blue-500 text-white text-sm"
        >
          Refresh
        </button>
      </div>

      {error ? <div className="text-red-400 text-sm">{error}</div> : null}
      {actionMessage ? <div className="text-emerald-300 text-sm">{actionMessage}</div> : null}
      {loading ? <div className="text-gray-300">Loading...</div> : null}

      <div className="flex gap-1 border-b border-gray-700 pb-0">
        <button
          onClick={() => setActiveTab('issues')}
          title="View issue triage tools and queue fix actions"
          className={`px-4 py-2 text-sm rounded-t border border-b-0 ${
            activeTab === 'issues'
              ? 'bg-gray-800 text-white border-gray-700'
              : 'text-gray-400 hover:text-white border-transparent'
          }`}
        >
          Issues
        </button>
        <button
          onClick={() => setActiveTab('batches')}
          title="View and run AI fix batches for a set"
          className={`px-4 py-2 text-sm rounded-t border border-b-0 ${
            activeTab === 'batches'
              ? 'bg-gray-800 text-white border-gray-700'
              : 'text-gray-400 hover:text-white border-transparent'
          }`}
        >
          Batches
        </button>
        <button
          onClick={() => setActiveTab('set_runs')}
          title="View and run end-to-end set workflows"
          className={`px-4 py-2 text-sm rounded-t border border-b-0 ${
            activeTab === 'set_runs'
              ? 'bg-gray-800 text-white border-gray-700'
              : 'text-gray-400 hover:text-white border-transparent'
          }`}
        >
          Set Runs
        </button>
      </div>

      <div className="border border-gray-700 rounded bg-gray-900">
        <button
          onClick={() =>
            setTabHelpOpen((prev) => ({
              ...prev,
              [activeTab]: !prev[activeTab],
            }))
          }
          title="Toggle usage guidance for this tab"
          className="w-full px-4 py-2 text-left text-sm text-gray-300 hover:bg-gray-800/40 flex items-center justify-between"
        >
          <span>How To Use This Tab</span>
          <span className="text-xs text-gray-400">{tabHelpOpen[activeTab] ? 'Hide' : 'Show'}</span>
        </button>
        {tabHelpOpen[activeTab] ? (
          <div className="px-4 pb-3 text-xs text-gray-300 border-t border-gray-800">{TAB_HELP_TEXT[activeTab]}</div>
        ) : null}
      </div>

      {activeTab === 'batches' && (
        <div className="border border-gray-700 rounded p-4 bg-gray-900 space-y-3">
        <div className="text-sm text-gray-300 font-medium">Run Set Batch</div>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
          <label className="text-xs text-gray-300 space-y-1">
            <span>Set ID</span>
            <input
              value={setId}
              onChange={(e) => setSetId(e.target.value)}
              placeholder="e.g. bt24"
              className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
            />
          </label>
          <label className="text-xs text-gray-300 space-y-1">
            <span>Run Mode</span>
            <select
              value={runMode}
              onChange={(e) => setRunMode(e.target.value as 'pr' | 'main')}
              className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
            >
              <option value="pr">Create PR</option>
              <option value="main">Apply Directly to Main</option>
            </select>
          </label>
          <label className="text-xs text-gray-300 space-y-1">
            <span>Scope Profile</span>
            <select
              value={scopeProfile}
              onChange={(e) => setScopeProfile(e.target.value as typeof scopeProfile)}
              className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
            >
              <option value="script">Script</option>
              <option value="script_engine">Script + Engine</option>
              <option value="script_engine_transpiler">Script + Engine + Transpiler</option>
            </select>
          </label>
          <label className="text-xs text-gray-300 space-y-1">
            <span>Model</span>
            <select
              value={modelName}
              onChange={(e) => setModelName(e.target.value)}
              className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
            >
              <option value="">Use backend defaults</option>
              {AVAILABLE_MODEL_OPTIONS.map((model) => (
                <option key={model} value={model}>
                  {model}
                </option>
              ))}
            </select>
          </label>
          <label className="text-xs text-gray-300 space-y-1">
            <span>Concurrency</span>
            <input
              value={concurrency}
              onChange={(e) => setConcurrency(e.target.value)}
              placeholder="4"
              className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
            />
          </label>
          <label className="text-xs text-gray-300 space-y-1">
            <span>Max Total Cost (USD)</span>
            <input
              value={maxTotalCostUsd}
              onChange={(e) => setMaxTotalCostUsd(e.target.value)}
              placeholder="5.0"
              className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
            />
          </label>
          <label className="text-xs text-gray-300 space-y-1">
            <span>Failure Rate Stop</span>
            <input
              value={failureRateStop}
              onChange={(e) => setFailureRateStop(e.target.value)}
              placeholder="0.3"
              className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
            />
          </label>
          <label className="text-xs text-gray-300 space-y-1">
            <span>Max Tasks</span>
            <input
              value={maxTasks}
              onChange={(e) => setMaxTasks(e.target.value)}
              placeholder="0 (all)"
              className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
            />
          </label>
          <label className="text-xs text-gray-300 flex items-center gap-2 pt-5">
            <input type="checkbox" checked={dryRun} onChange={(e) => setDryRun(e.target.checked)} />
            Dry Run
          </label>
        </div>
        <div className="flex gap-2">
          <button
            onClick={() => void onApproveSet()}
            title="Approve matching new issues in this set for AI processing"
            className="px-3 py-1.5 rounded bg-emerald-700 hover:bg-emerald-600 text-white text-sm"
          >
            Approve Set For AI
          </button>
          <button
            onClick={() => void onCreateSetRun()}
            title="Create and start a full set run (QA, review, and fix pipeline)"
            className="px-3 py-1.5 rounded bg-fuchsia-700 hover:bg-fuchsia-600 text-white text-sm"
          >
            Run Set End-to-End
          </button>
          <button
            onClick={() => void onPreviewBatch()}
            title="Preview which set cards are eligible for batch fixing"
            className="px-3 py-1.5 rounded bg-slate-700 hover:bg-slate-600 text-white text-sm"
          >
            Preview Eligible Cards
          </button>
          <button
            onClick={() => void onCreateBatch()}
            title="Start batch execution or dry-run preview with current form values"
            className="px-3 py-1.5 rounded bg-blue-600 hover:bg-blue-500 text-white text-sm"
          >
            {dryRun ? 'Run Dry Preview' : 'Start Batch'}
          </button>
        </div>
        {preview ? (
          <div className="text-xs text-gray-300 border border-gray-800 rounded p-3 bg-gray-950">
            eligible={preview.eligible_count} selected={preview.selected_count} preview_only={String(preview.preview_only)}
            <div className="mt-1 text-gray-400">cards: {preview.cards.join(', ') || '(none)'}</div>
          </div>
        ) : null}
        </div>
      )}

      {activeTab === 'batches' && (
        <div className="border border-gray-700 rounded p-4 bg-gray-900 space-y-3">
        <div className="text-sm text-gray-300 font-medium">Batch Status</div>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          <select
            value={selectedBatchId}
            onChange={(e) => setSelectedBatchId(e.target.value)}
            className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
          >
            <option value="">Select batch</option>
            {batches.map((batch) => (
              <option key={batch.id} value={batch.id}>
                {batch.set_id} {batch.run_mode} {batch.scope_profile} ({batch.status})
              </option>
            ))}
          </select>
          {selectedBatch ? (
            <button
              onClick={() => void onCancelBatch(selectedBatch.batch.id)}
              title="Stop scheduling/running this batch"
              className="px-3 py-1.5 rounded bg-amber-700 hover:bg-amber-600 text-white text-sm w-fit"
            >
              Cancel Batch
            </button>
          ) : null}
        </div>
        {selectedBatch ? (
          <div className="text-xs text-gray-300 space-y-1">
            <div>
              {selectedBatch.batch.id} status={selectedBatch.batch.status} applied={selectedBatch.batch.applied_count} failed={selectedBatch.batch.failed_count} commits={selectedBatch.batch.commit_count}
            </div>
            <div>stop_reason={selectedBatch.batch.stopped_reason ?? '(none)'}</div>
            <div>pr_url={selectedBatch.batch.pr_url ?? '(none)'}</div>
            <div className="max-h-44 overflow-auto border border-gray-800 rounded p-2 bg-gray-950">
              {selectedBatch.items.map((item) => (
                <div key={item.id} className="text-[11px] text-gray-300 py-0.5">
                  {item.card_id} status={item.status} task={item.task_id ?? 'n/a'} commit={item.commit_sha ?? 'n/a'}
                  {item.error_text ? ` error=${item.error_text}` : ''}
                </div>
              ))}
            </div>
          </div>
        ) : null}
        </div>
      )}

      {activeTab === 'set_runs' && (
        <div className="border border-gray-700 rounded p-4 bg-gray-900 space-y-3">
          <div className="text-sm text-gray-300 font-medium">Set Run Actions</div>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
            <label className="text-xs text-gray-300 space-y-1">
              <span>Set ID</span>
              <input
                value={setId}
                onChange={(e) => setSetId(e.target.value)}
                placeholder="e.g. bt24"
                className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
              />
            </label>
            <label className="text-xs text-gray-300 space-y-1">
              <span>Run Mode</span>
              <select
                value={runMode}
                onChange={(e) => setRunMode(e.target.value as 'pr' | 'main')}
                className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
              >
                <option value="pr">Create PR</option>
                <option value="main">Apply Directly to Main</option>
              </select>
            </label>
            <label className="text-xs text-gray-300 space-y-1">
              <span>Scope Profile</span>
              <select
                value={scopeProfile}
                onChange={(e) => setScopeProfile(e.target.value as typeof scopeProfile)}
                className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
              >
                <option value="script">Script</option>
                <option value="script_engine">Script + Engine</option>
                <option value="script_engine_transpiler">Script + Engine + Transpiler</option>
              </select>
            </label>
            <label className="text-xs text-gray-300 space-y-1">
              <span>Model</span>
              <select
                value={modelName}
                onChange={(e) => setModelName(e.target.value)}
                className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
              >
                <option value="">Use backend defaults</option>
                {AVAILABLE_MODEL_OPTIONS.map((model) => (
                  <option key={model} value={model}>
                    {model}
                  </option>
                ))}
              </select>
            </label>
            <label className="text-xs text-gray-300 space-y-1">
              <span>Max Total Cost (USD)</span>
              <input
                value={maxTotalCostUsd}
                onChange={(e) => setMaxTotalCostUsd(e.target.value)}
                placeholder="5.0"
                className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
              />
            </label>
            <label className="text-xs text-gray-300 space-y-1">
              <span>Failure Rate Stop</span>
              <input
                value={failureRateStop}
                onChange={(e) => setFailureRateStop(e.target.value)}
                placeholder="0.3"
                className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
              />
            </label>
            <label className="text-xs text-gray-300 space-y-1">
              <span>Max Fix Tasks</span>
              <input
                value={maxTasks}
                onChange={(e) => setMaxTasks(e.target.value)}
                placeholder="0 (all)"
                className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
              />
            </label>
          </div>
          <div className="flex gap-2">
            <button
              onClick={() => void onApproveSet()}
              title="Approve matching new issues in this set for AI processing"
              className="px-3 py-1.5 rounded bg-emerald-700 hover:bg-emerald-600 text-white text-sm"
            >
              Approve Set For AI
            </button>
            <button
              onClick={() => void onCreateSetRun()}
              title="Start an end-to-end set run with current settings"
              className="px-3 py-1.5 rounded bg-fuchsia-700 hover:bg-fuchsia-600 text-white text-sm"
            >
              Run Set End-to-End
            </button>
          </div>
        </div>
      )}

      {activeTab === 'set_runs' && (
        <div className="border border-gray-700 rounded p-4 bg-gray-900 space-y-3">
        <div className="text-sm text-gray-300 font-medium">Set Run Report</div>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          <select
            value={selectedSetRunId}
            onChange={(e) => setSelectedSetRunId(e.target.value)}
            className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
          >
            <option value="">Select set run</option>
            {setRuns.map((run) => (
              <option key={run.id} value={run.id}>
                {run.set_id} {run.stage} ({run.status})
              </option>
            ))}
          </select>
          {selectedSetRun ? (
            <button
              onClick={() => void onCancelSetRun(selectedSetRun.run.id)}
              title="Cancel this set run and stop pending fix execution"
              className="px-3 py-1.5 rounded bg-amber-700 hover:bg-amber-600 text-white text-sm w-fit"
            >
              Cancel Set Run
            </button>
          ) : null}
        </div>
        {selectedSetRun ? (
          <div className="text-xs text-gray-300 space-y-1">
            <div>
              {selectedSetRun.run.id} stage={selectedSetRun.run.stage} status={selectedSetRun.run.status}
            </div>
            <div>
              qa={selectedSetRun.run.qa_completed}/{selectedSetRun.run.qa_total} review={selectedSetRun.run.review_completed}/{selectedSetRun.run.review_total} fix_applied={selectedSetRun.run.fix_applied}/{selectedSetRun.run.fix_total}
            </div>
            <div>stop_reason={selectedSetRun.run.stopped_reason ?? '(none)'}</div>
            <div className="max-h-40 overflow-auto border border-gray-800 rounded p-2 bg-gray-950">
              {selectedSetRun.items.map((item) => (
                <div key={item.id} className="text-[11px] text-gray-300 py-0.5">
                  {item.card_id} faithful={String(item.review_faithful)} apply={item.fix_apply_status} pr={item.pr_url ?? 'n/a'}
                  {item.error_text ? ` error=${item.error_text}` : ''}
                </div>
              ))}
            </div>
            <div className="max-h-32 overflow-auto border border-gray-800 rounded p-2 bg-gray-950">
              <div className="text-[11px] text-gray-400">PR queue</div>
              {selectedSetRun.pr_queue.map((item) => (
                <div key={`${item.id}-pr`} className="text-[11px] text-gray-300 py-0.5">
                  {item.card_id} {'->'} {item.pr_url}
                </div>
              ))}
            </div>
          </div>
        ) : null}
        </div>
      )}

      {activeTab === 'issues' && (
        <>
          <div className="border border-gray-700 rounded p-4 bg-gray-900 space-y-3">
            <div className="text-sm text-gray-300 font-medium">Issue Actions</div>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
              <label className="text-xs text-gray-300 space-y-1">
                <span>Set ID</span>
                <input
                  value={setId}
                  onChange={(e) => setSetId(e.target.value)}
                  placeholder="e.g. bt24"
                  className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
                />
              </label>
              <label className="text-xs text-gray-300 space-y-1">
                <span>Queue Mode</span>
                <select
                  value={issueQueueMode}
                  onChange={(e) => setIssueQueueMode(e.target.value as 'approved_only' | 'new_and_approved')}
                  className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
                >
                  <option value="approved_only">approved_only</option>
                  <option value="new_and_approved">new_and_approved</option>
                </select>
              </label>
            </div>
            <div className="flex gap-2">
              <button
                onClick={() => void onApproveSet()}
                title="Approve new issues in this set so they become eligible for AI fix queueing"
                className="px-3 py-1.5 rounded bg-emerald-700 hover:bg-emerald-600 text-white text-sm"
              >
                Approve Set For AI
              </button>
              <button
                onClick={() => void onQueueSetIssueFixes()}
                title="Queue one script_autofix task per card for this set, skipping duplicate/overlapping cards"
                className="px-3 py-1.5 rounded bg-indigo-700 hover:bg-indigo-600 text-white text-sm"
              >
                Queue Fixes For Set
              </button>
            </div>
          </div>

          <div className="border border-gray-700 rounded p-4 bg-gray-900 space-y-3">
            <div className="text-sm text-gray-300 font-medium">Issue Filters</div>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
              <select
                value={issueSetFilter}
                onChange={(e) => setIssueSetFilter(e.target.value)}
                className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
              >
                <option value="all">All sets</option>
                {issueSetOptions.map((setOption) => (
                  <option key={setOption} value={setOption}>
                    {setOption.toUpperCase()}
                  </option>
                ))}
              </select>
              <select
                value={issueStatusFilter}
                onChange={(e) => setIssueStatusFilter(e.target.value as 'all' | IssueItem['status'])}
                className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
              >
                <option value="all">All statuses</option>
                <option value="new">new</option>
                <option value="approved_for_ai">approved_for_ai</option>
                <option value="rejected">rejected</option>
                <option value="resolved">resolved</option>
              </select>
              <select
                value={issueSeverityFilter}
                onChange={(e) => setIssueSeverityFilter(e.target.value as 'all' | IssueItem['severity'])}
                className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
              >
                <option value="all">All severities</option>
                <option value="low">low</option>
                <option value="medium">medium</option>
                <option value="high">high</option>
                <option value="critical">critical</option>
              </select>
              <select
                value={issueSourceFilter}
                onChange={(e) => setIssueSourceFilter(e.target.value as 'all' | IssueItem['source'])}
                className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
              >
                <option value="all">All sources</option>
                <option value="player">player</option>
                <option value="judge">judge</option>
                <option value="system">system</option>
              </select>
              <input
                value={issueSearch}
                onChange={(e) => setIssueSearch(e.target.value)}
                placeholder="search card, description, notes"
                className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white md:col-span-2"
              />
              <div className="text-sm text-gray-400 flex items-center">
                Showing {filteredIssues.length} of {issues.length}
              </div>
            </div>
          </div>

          <div className="overflow-x-auto border border-gray-700 rounded bg-gray-900">
            <table className="w-full text-xs">
              <thead>
                <tr className="text-gray-400 text-left border-b border-gray-700">
                  <th className="px-2 py-2">Card</th>
                  <th className="px-2 py-2">Status</th>
                  <th className="px-2 py-2">Severity</th>
                  <th className="px-2 py-2">Source</th>
                  <th className="px-2 py-2">Description</th>
                  <th className="px-2 py-2">Actions</th>
                </tr>
              </thead>
              <tbody>
                {filteredIssues.map((issue) => (
                  <tr key={issue.id} className="border-t border-gray-800 text-gray-200 align-top">
                    <td className="px-2 py-2 font-mono">{issue.card_id}</td>
                    <td className="px-2 py-2">{issue.status}</td>
                    <td className="px-2 py-2">{issue.severity}</td>
                    <td className="px-2 py-2">{issue.source}</td>
                    <td className="px-2 py-2 max-w-md">
                      <div className="text-gray-300 break-words">
                        {issue.description.length > 220 ? `${issue.description.slice(0, 220)}...` : issue.description}
                      </div>
                      <button
                        onClick={() => setExpandedIssue(issue)}
                        title="Open full issue details in a modal"
                        className="mt-1 px-2 py-0.5 rounded bg-gray-700 hover:bg-gray-600 text-[11px] text-white"
                      >
                        View Full
                      </button>
                    </td>
                    <td className="px-2 py-2">
                      <div className="flex flex-wrap gap-2">
                        <button
                          onClick={() => void setIssueStatus(issue, 'approved_for_ai')}
                          title="Mark this issue approved_for_ai"
                          className="px-2 py-1 rounded bg-emerald-700 hover:bg-emerald-600 text-xs text-white"
                        >
                          Approve
                        </button>
                        <button
                          onClick={() => void onQueueIssueFix(issue.id)}
                          title="Queue one script_autofix task for this specific issue"
                          className="px-2 py-1 rounded bg-indigo-700 hover:bg-indigo-600 text-xs text-white"
                        >
                          Queue Fix
                        </button>
                        <button
                          onClick={() => void setIssueStatus(issue, 'rejected')}
                          title="Mark this issue rejected"
                          className="px-2 py-1 rounded bg-amber-700 hover:bg-amber-600 text-xs text-white"
                        >
                          Reject
                        </button>
                        <button
                          onClick={() => void setIssueStatus(issue, 'resolved')}
                          title="Mark this issue resolved"
                          className="px-2 py-1 rounded bg-slate-700 hover:bg-slate-600 text-xs text-white"
                        >
                          Resolve
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}

      {expandedIssue ? (
        <div
          className="fixed inset-0 z-50 bg-black/70 p-4 flex items-center justify-center"
          onClick={() => setExpandedIssue(null)}
        >
          <div
            className="w-full max-w-3xl max-h-[85vh] overflow-hidden border border-gray-700 rounded bg-gray-900"
            onClick={(event) => event.stopPropagation()}
          >
            <div className="px-4 py-3 border-b border-gray-700 flex items-center justify-between">
              <div className="text-sm text-white font-medium">Issue Description</div>
              <button
                onClick={() => setExpandedIssue(null)}
                title="Close issue detail modal"
                className="px-2 py-1 rounded bg-gray-700 hover:bg-gray-600 text-xs text-white"
              >
                Close
              </button>
            </div>
            <div className="p-4 overflow-y-auto max-h-[calc(85vh-56px)] space-y-3">
              <div className="text-xs text-gray-400">
                {expandedIssue.card_id} | status={expandedIssue.status} | severity={expandedIssue.severity} | source=
                {expandedIssue.source}
              </div>
              <div className="text-sm text-gray-100 whitespace-pre-wrap break-words">{expandedIssue.description}</div>
              {expandedIssue.triage_notes ? (
                <div>
                  <div className="text-xs text-gray-400 mb-1">Triage Notes</div>
                  <div className="text-sm text-gray-200 whitespace-pre-wrap break-words">{expandedIssue.triage_notes}</div>
                </div>
              ) : null}
              {expandedIssue.resolution_notes ? (
                <div>
                  <div className="text-xs text-gray-400 mb-1">Resolution Notes</div>
                  <div className="text-sm text-gray-200 whitespace-pre-wrap break-words">
                    {expandedIssue.resolution_notes}
                  </div>
                </div>
              ) : null}
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}
