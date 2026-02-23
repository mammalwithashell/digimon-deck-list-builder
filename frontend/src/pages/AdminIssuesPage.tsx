import { useEffect, useMemo, useState } from 'react';
import {
  cancelAIFixBatch,
  createAIFixBatch,
  getAIFixBatch,
  getAIFixBatches,
  getIssues,
  previewAIFixBatch,
  queueIssueFix,
  type AIFixBatch,
  type AIFixBatchDetailResponse,
  type AIFixBatchCreateResponse,
  type IssueItem,
  updateIssue,
} from '@/api/adminApi';

export function AdminIssuesPage() {
  const [issues, setIssues] = useState<IssueItem[]>([]);
  const [batches, setBatches] = useState<AIFixBatch[]>([]);
  const [selectedBatchId, setSelectedBatchId] = useState<string>('');
  const [selectedBatch, setSelectedBatch] = useState<AIFixBatchDetailResponse | null>(null);
  const [preview, setPreview] = useState<AIFixBatchCreateResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [setId, setSetId] = useState('bt24');
  const [runMode, setRunMode] = useState<'pr' | 'main'>('pr');
  const [scopeProfile, setScopeProfile] = useState<'script' | 'script_engine' | 'script_engine_transpiler'>('script');
  const [modelName, setModelName] = useState('');
  const [concurrency, setConcurrency] = useState('4');
  const [maxTotalCostUsd, setMaxTotalCostUsd] = useState('5.0');
  const [failureRateStop, setFailureRateStop] = useState('0.3');
  const [maxTasks, setMaxTasks] = useState('0');
  const [dryRun, setDryRun] = useState(true);
  const [issueSetFilter, setIssueSetFilter] = useState('all');
  const [issueStatusFilter, setIssueStatusFilter] = useState<'all' | IssueItem['status']>('all');

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
    return issues.filter((issue) => {
      const statusPass = issueStatusFilter === 'all' || issue.status === issueStatusFilter;
      const setPrefix = issue.card_id.split('-', 1)[0]?.toLowerCase() ?? '';
      const setPass = issueSetFilter === 'all' || setPrefix === issueSetFilter;
      return statusPass && setPass;
    });
  }, [issues, issueSetFilter, issueStatusFilter]);

  const refresh = async () => {
    setLoading(true);
    setError(null);
    try {
      const [issueData, batchData] = await Promise.all([getIssues(), getAIFixBatches()]);
      setIssues(issueData);
      setBatches(batchData);
      const firstBatch = batchData[0];
      if (!selectedBatchId && firstBatch) {
        setSelectedBatchId(firstBatch.id);
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

  return (
    <div className="max-w-6xl mx-auto p-6 space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold text-white">Admin Issues</h1>
        <button
          onClick={() => void refresh()}
          className="px-3 py-1.5 rounded bg-blue-600 hover:bg-blue-500 text-white text-sm"
        >
          Refresh
        </button>
      </div>

      {error ? <div className="text-red-400 text-sm">{error}</div> : null}
      {loading ? <div className="text-gray-300">Loading...</div> : null}

      <div className="border border-gray-700 rounded p-4 bg-gray-900 space-y-3">
        <div className="text-sm text-gray-300 font-medium">Run Set Batch</div>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
          <input
            value={setId}
            onChange={(e) => setSetId(e.target.value)}
            placeholder="set_id (e.g. bt24)"
            className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
          />
          <select
            value={runMode}
            onChange={(e) => setRunMode(e.target.value as 'pr' | 'main')}
            className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
          >
            <option value="pr">Create PR</option>
            <option value="main">Apply Directly to Main</option>
          </select>
          <select
            value={scopeProfile}
            onChange={(e) => setScopeProfile(e.target.value as typeof scopeProfile)}
            className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
          >
            <option value="script">Script</option>
            <option value="script_engine">Script + Engine</option>
            <option value="script_engine_transpiler">Script + Engine + Transpiler</option>
          </select>
          <input
            value={modelName}
            onChange={(e) => setModelName(e.target.value)}
            placeholder="model override (optional)"
            className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
          />
          <input
            value={concurrency}
            onChange={(e) => setConcurrency(e.target.value)}
            placeholder="concurrency"
            className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
          />
          <input
            value={maxTotalCostUsd}
            onChange={(e) => setMaxTotalCostUsd(e.target.value)}
            placeholder="max_total_cost_usd"
            className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
          />
          <input
            value={failureRateStop}
            onChange={(e) => setFailureRateStop(e.target.value)}
            placeholder="failure_rate_stop"
            className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
          />
          <input
            value={maxTasks}
            onChange={(e) => setMaxTasks(e.target.value)}
            placeholder="max_tasks (0 = all)"
            className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
          />
          <label className="text-sm text-gray-300 flex items-center gap-2">
            <input type="checkbox" checked={dryRun} onChange={(e) => setDryRun(e.target.checked)} />
            dry_run
          </label>
        </div>
        <div className="flex gap-2">
          <button
            onClick={() => void onPreviewBatch()}
            className="px-3 py-1.5 rounded bg-slate-700 hover:bg-slate-600 text-white text-sm"
          >
            Preview Eligible Cards
          </button>
          <button
            onClick={() => void onCreateBatch()}
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
          <div className="text-sm text-gray-400 flex items-center">
            Showing {filteredIssues.length} of {issues.length}
          </div>
        </div>
      </div>

      <div className="space-y-3">
        {filteredIssues.map((issue) => (
          <div key={issue.id} className="border border-gray-700 rounded p-4 bg-gray-900">
            <div className="flex items-center justify-between">
              <div className="text-white font-medium">{issue.card_id}</div>
              <div className="text-xs text-gray-400">{issue.status}</div>
            </div>
            <div className="text-sm text-gray-300 mt-2">{issue.description}</div>
            <div className="text-xs text-gray-500 mt-1">
              source={issue.source} severity={issue.severity}
            </div>
            <div className="flex gap-2 mt-3">
              <button
                onClick={() => void setIssueStatus(issue, 'approved_for_ai')}
                className="px-2 py-1 rounded bg-emerald-700 hover:bg-emerald-600 text-xs text-white"
              >
                Approve for AI
              </button>
              <button
                onClick={() => void onQueueIssueFix(issue.id)}
                className="px-2 py-1 rounded bg-indigo-700 hover:bg-indigo-600 text-xs text-white"
              >
                Queue Fix
              </button>
              <button
                onClick={() => void setIssueStatus(issue, 'rejected')}
                className="px-2 py-1 rounded bg-amber-700 hover:bg-amber-600 text-xs text-white"
              >
                Reject
              </button>
              <button
                onClick={() => void setIssueStatus(issue, 'resolved')}
                className="px-2 py-1 rounded bg-slate-700 hover:bg-slate-600 text-xs text-white"
              >
                Resolve
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
