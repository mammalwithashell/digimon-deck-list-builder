import { useEffect, useMemo, useState } from 'react';
import {
  createPromotion,
  getAppliedCards,
  getPromotions,
  promoteTaskCard,
  type AppliedCardItem,
  type PromotionItem,
} from '@/api/adminApi';

type FormErrors = {
  card_id?: string;
  set_id?: string;
  module_name?: string;
  expected_generated_hash?: string;
};

function summarizeHash(hash: string): string {
  if (!hash) return 'n/a';
  if (hash.length <= 20) return hash;
  return `${hash.slice(0, 10)}...${hash.slice(-6)}`;
}

function setIdFromCardId(cardId: string): string {
  const normalized = cardId.trim().toUpperCase();
  const idx = normalized.indexOf('-');
  if (idx <= 0) {
    return 'unknown';
  }
  return normalized.slice(0, idx).toLowerCase();
}

export function AdminPromotionsPage() {
  const [promotions, setPromotions] = useState<PromotionItem[]>([]);
  const [appliedCards, setAppliedCards] = useState<AppliedCardItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [formErrors, setFormErrors] = useState<FormErrors>({});
  const [candidateMessage, setCandidateMessage] = useState<string | null>(null);
  const [candidateAccordionOpen, setCandidateAccordionOpen] = useState(true);
  const [collapsedCandidateSets, setCollapsedCandidateSets] = useState<Record<string, boolean>>({});
  const [promotingCandidateIds, setPromotingCandidateIds] = useState<Record<string, boolean>>({});

  const [cardId, setCardId] = useState('BT24-001');
  const [setId, setSetId] = useState('bt24');
  const [moduleName, setModuleName] = useState('bt24_001');
  const [expectedHash, setExpectedHash] = useState('');
  const [notes, setNotes] = useState('');

  const [filterSetId, setFilterSetId] = useState('');
  const [filterWorkId, setFilterWorkId] = useState('');
  const [filterSearch, setFilterSearch] = useState('');
  const [limit, setLimit] = useState('200');

  const refresh = async () => {
    setLoading(true);
    setError(null);
    setCandidateMessage(null);
    try {
      const params: { set_id?: string; work_id?: string; limit?: number } = {};
      if (filterSetId.trim()) {
        params.set_id = filterSetId.trim();
      }
      if (filterWorkId.trim()) {
        params.work_id = filterWorkId.trim();
      }
      const normalizedLimit = Math.min(500, Math.max(1, Number(limit) || 200));
      params.limit = normalizedLimit;
      const [promotionData, appliedData] = await Promise.all([
        getPromotions(params),
        getAppliedCards({
          status: 'applied',
          set_id: filterSetId.trim() || undefined,
          limit: normalizedLimit,
        }),
      ]);
      setPromotions(promotionData);
      setAppliedCards(appliedData);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load promotions';
      setError(message);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void refresh();
    // initial only
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const filteredPromotions = useMemo(() => {
    const q = filterSearch.trim().toLowerCase();
    if (!q) return promotions;
    return promotions.filter((item) => {
      return item.card_id.toLowerCase().includes(q) || item.module_name.toLowerCase().includes(q);
    });
  }, [promotions, filterSearch]);

  const groupedPromotions = useMemo(() => {
    const groups: Record<string, PromotionItem[]> = {};
    for (const item of filteredPromotions) {
      const key = item.set_id || 'unknown';
      if (!groups[key]) groups[key] = [];
      groups[key].push(item);
    }
    return Object.entries(groups).sort(([a], [b]) => a.localeCompare(b));
  }, [filteredPromotions]);

  const promotionTaskIds = useMemo(() => {
    return new Set(promotions.map((item) => item.ai_task_id).filter((value): value is string => Boolean(value)));
  }, [promotions]);

  const candidateRows = useMemo(() => {
    const q = filterSearch.trim().toLowerCase();
    return appliedCards.filter((item) => {
      if (item.ai_task_id && promotionTaskIds.has(item.ai_task_id)) {
        return false;
      }
      if (filterWorkId.trim()) {
        const taskId = item.ai_task_id ?? '';
        if (!taskId.toLowerCase().includes(filterWorkId.trim().toLowerCase())) {
          return false;
        }
      }
      if (!q) {
        return true;
      }
      return item.card_id.toLowerCase().includes(q) || (item.ai_task_id ?? '').toLowerCase().includes(q);
    });
  }, [appliedCards, promotionTaskIds, filterSearch, filterWorkId]);

  const groupedCandidates = useMemo(() => {
    const groups: Record<string, AppliedCardItem[]> = {};
    for (const item of candidateRows) {
      const key = setIdFromCardId(item.card_id);
      if (!groups[key]) groups[key] = [];
      groups[key].push(item);
    }
    return Object.entries(groups).sort(([a], [b]) => a.localeCompare(b));
  }, [candidateRows]);

  useEffect(() => {
    setCollapsedCandidateSets((prev) => {
      const next = { ...prev };
      for (const [setId] of groupedCandidates) {
        if (!(setId in next)) {
          next[setId] = false;
        }
      }
      return next;
    });
  }, [groupedCandidates]);

  const validateForm = (): FormErrors => {
    const nextErrors: FormErrors = {};
    if (!cardId.trim()) {
      nextErrors.card_id = 'Card ID is required.';
    }
    if (!setId.trim()) {
      nextErrors.set_id = 'Set ID is required.';
    }
    if (!moduleName.trim()) {
      nextErrors.module_name = 'Module Name is required.';
    }
    const normalizedHash = expectedHash.trim();
    if (!normalizedHash) {
      nextErrors.expected_generated_hash = 'Expected Generated Hash is required.';
    } else if (!/^[a-fA-F0-9]{64}$/.test(normalizedHash)) {
      nextErrors.expected_generated_hash = 'Expected Generated Hash must be a 64-character SHA-256 hex string.';
    }
    return nextErrors;
  };

  const onPromote = async () => {
    const nextErrors = validateForm();
    setFormErrors(nextErrors);
    if (Object.keys(nextErrors).length > 0) {
      return;
    }

    try {
      await createPromotion({
        card_id: cardId.trim(),
        set_id: setId.trim(),
        module_name: moduleName.trim(),
        expected_generated_hash: expectedHash.trim(),
        notes: notes.trim(),
      });
      setExpectedHash('');
      setNotes('');
      await refresh();
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Promotion failed';
      setError(message);
    }
  };

  const onPromoteCandidate = async (item: AppliedCardItem) => {
    if (!item.ai_task_id) {
      setError(`Cannot task-link promote ${item.card_id}: missing ai_task_id`);
      return;
    }
    setError(null);
    setCandidateMessage(null);
    setPromotingCandidateIds((prev) => ({ ...prev, [item.id]: true }));
    try {
      await promoteTaskCard(item.ai_task_id, {
        card_id: item.card_id,
        notes: 'Promoted from applied-fix candidate',
      });
      setCandidateMessage(`Promoted ${item.card_id} from applied-fix candidates.`);
      await refresh();
    } catch (err) {
      const message = err instanceof Error ? err.message : `Failed to promote ${item.card_id}`;
      setError(message);
    } finally {
      setPromotingCandidateIds((prev) => ({ ...prev, [item.id]: false }));
    }
  };

  return (
    <div className="max-w-6xl mx-auto p-6 space-y-6">
      <h1 className="text-2xl font-semibold text-white">Admin Promotions</h1>
      {error ? <div className="text-red-400 text-sm">{error}</div> : null}

      <div className="border border-gray-700 rounded p-4 bg-gray-900 space-y-4">
        <div className="text-sm text-gray-300 font-medium">Promote Script</div>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          <label className="text-sm text-gray-300 space-y-1">
            <span>Card ID</span>
            <input
              value={cardId}
              onChange={(e) => setCardId(e.target.value)}
              placeholder="BT24-001"
              className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
            />
            {formErrors.card_id ? <div className="text-xs text-red-400">{formErrors.card_id}</div> : null}
          </label>
          <label className="text-sm text-gray-300 space-y-1">
            <span>Set ID</span>
            <input
              value={setId}
              onChange={(e) => setSetId(e.target.value)}
              placeholder="bt24"
              className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
            />
            {formErrors.set_id ? <div className="text-xs text-red-400">{formErrors.set_id}</div> : null}
          </label>
          <label className="text-sm text-gray-300 space-y-1">
            <span>Module Name</span>
            <input
              value={moduleName}
              onChange={(e) => setModuleName(e.target.value)}
              placeholder="bt24_001"
              className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
            />
            {formErrors.module_name ? <div className="text-xs text-red-400">{formErrors.module_name}</div> : null}
          </label>
          <label className="text-sm text-gray-300 space-y-1">
            <span>Expected Generated Hash</span>
            <input
              value={expectedHash}
              onChange={(e) => setExpectedHash(e.target.value)}
              placeholder="sha256..."
              className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
            />
            {formErrors.expected_generated_hash ? (
              <div className="text-xs text-red-400">{formErrors.expected_generated_hash}</div>
            ) : null}
          </label>
          <label className="text-sm text-gray-300 space-y-1 md:col-span-2">
            <span>Notes (optional)</span>
            <input
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              placeholder="promotion notes"
              className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
            />
          </label>
        </div>
        <button
          onClick={() => void onPromote()}
          className="px-3 py-1.5 rounded bg-blue-600 hover:bg-blue-500 text-white text-sm"
        >
          Promote Script
        </button>
      </div>

      <div className="border border-gray-700 rounded bg-gray-900">
        <button
          onClick={() => setCandidateAccordionOpen((prev) => !prev)}
          className="w-full px-4 py-3 flex items-center justify-between text-left hover:bg-gray-800/40"
        >
          <span className="text-sm text-gray-300 font-medium">Promotion Candidates (Applied Fixes)</span>
          <span className="text-xs text-gray-400">{candidateAccordionOpen ? 'Hide' : 'Show'}</span>
        </button>
        {candidateAccordionOpen ? (
          <div className="px-4 pb-4 border-t border-gray-800 space-y-3">
            <div className="text-xs text-gray-400">
              showing {candidateRows.length} candidate{candidateRows.length !== 1 ? 's' : ''} from {appliedCards.length}{' '}
              applied fix{appliedCards.length !== 1 ? 'es' : ''}
            </div>
            {candidateMessage ? <div className="text-xs text-emerald-300">{candidateMessage}</div> : null}
            {groupedCandidates.length === 0 ? (
              <div className="text-sm text-gray-400">No promotion candidates found.</div>
            ) : null}
            {groupedCandidates.map(([setId, items]) => {
              const isCollapsed = collapsedCandidateSets[setId] === true;
              return (
                <div key={setId} className="border border-gray-700 rounded bg-gray-900">
                  <button
                    onClick={() => setCollapsedCandidateSets((prev) => ({ ...prev, [setId]: !isCollapsed }))}
                    className="w-full px-3 py-2 flex items-center justify-between text-left hover:bg-gray-800/40"
                  >
                    <span className="text-sm text-gray-300 font-medium">
                      {setId.toUpperCase()} <span className="text-gray-500">({items.length})</span>
                    </span>
                    <span className="text-xs text-gray-400">{isCollapsed ? 'Expand' : 'Collapse'}</span>
                  </button>
                  {!isCollapsed ? (
                    <div className="overflow-x-auto border-t border-gray-700">
                      <table className="w-full text-xs">
                        <thead>
                          <tr className="text-gray-400 text-left border-b border-gray-700">
                            <th className="px-2 py-2">Card</th>
                            <th className="px-2 py-2">Task</th>
                            <th className="px-2 py-2">Applied</th>
                            <th className="px-2 py-2">Run</th>
                            <th className="px-2 py-2">Commit / PR</th>
                            <th className="px-2 py-2">Action</th>
                          </tr>
                        </thead>
                        <tbody>
                          {items.map((item) => (
                            <tr key={item.id} className="border-t border-gray-800 text-gray-200 align-top">
                              <td className="px-2 py-2 font-mono">{item.card_id}</td>
                              <td className="px-2 py-2 font-mono text-[11px]">{item.ai_task_id ?? 'n/a'}</td>
                              <td className="px-2 py-2 text-gray-400 whitespace-nowrap">{new Date(item.created_at).toLocaleString()}</td>
                              <td className="px-2 py-2">
                                <div className="space-y-1 text-[11px]">
                                  <div>{item.run_mode}</div>
                                  <div className="text-gray-500">{item.scope_profile}</div>
                                </div>
                              </td>
                              <td className="px-2 py-2">
                                <div className="space-y-1 text-[11px]">
                                  <div>commit {item.commit_sha ? summarizeHash(item.commit_sha) : 'n/a'}</div>
                                  <div>pr {item.pr_url ? 'linked' : 'n/a'}</div>
                                </div>
                              </td>
                              <td className="px-2 py-2">
                                <button
                                  onClick={() => void onPromoteCandidate(item)}
                                  disabled={promotingCandidateIds[item.id] === true || !item.ai_task_id}
                                  className="px-2 py-1 rounded bg-emerald-700 hover:bg-emerald-600 disabled:bg-gray-700 text-xs text-white"
                                >
                                  {promotingCandidateIds[item.id] === true ? 'Promoting...' : 'Promote'}
                                </button>
                              </td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                  ) : null}
                </div>
              );
            })}
          </div>
        ) : null}
      </div>

      <div className="border border-gray-700 rounded p-4 bg-gray-900 space-y-3">
        <div className="text-sm text-gray-300 font-medium">History Filters</div>
        <div className="grid grid-cols-1 md:grid-cols-4 gap-3">
          <input
            value={filterSetId}
            onChange={(e) => setFilterSetId(e.target.value)}
            placeholder="set_id (e.g. bt24)"
            className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
          />
          <input
            value={filterWorkId}
            onChange={(e) => setFilterWorkId(e.target.value)}
            placeholder="work id"
            className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
          />
          <input
            value={filterSearch}
            onChange={(e) => setFilterSearch(e.target.value)}
            placeholder="search card/module"
            className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
          />
          <input
            value={limit}
            onChange={(e) => setLimit(e.target.value)}
            placeholder="fetch limit (1-500)"
            className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
          />
        </div>
        <div className="flex items-center gap-3">
          <button
            onClick={() => void refresh()}
            className="px-3 py-1.5 rounded bg-blue-600 hover:bg-blue-500 text-white text-sm"
          >
            Apply / Refresh
          </button>
          <div className="text-xs text-gray-400">
            showing {filteredPromotions.length} of {promotions.length}
          </div>
        </div>
      </div>

      {loading ? <div className="text-gray-300">Loading...</div> : null}

      {!loading && groupedPromotions.length === 0 ? (
        <div className="text-sm text-gray-400">No promotions found.</div>
      ) : null}

      {groupedPromotions.map(([groupSetId, items]) => (
        <div key={groupSetId} className="space-y-2">
          <div className="text-sm text-gray-300 font-medium border-b border-gray-700 pb-1">
            {groupSetId.toUpperCase()} <span className="text-gray-500">({items.length})</span>
          </div>
          <div className="overflow-x-auto border border-gray-700 rounded bg-gray-900">
            <table className="w-full text-xs">
              <thead>
                <tr className="text-gray-400 text-left border-b border-gray-700">
                  <th className="px-2 py-2">Promoted</th>
                  <th className="px-2 py-2">Card</th>
                  <th className="px-2 py-2">Module</th>
                  <th className="px-2 py-2">Set</th>
                  <th className="px-2 py-2">Work</th>
                  <th className="px-2 py-2">Task</th>
                  <th className="px-2 py-2">Manifest</th>
                  <th className="px-2 py-2">Hashes</th>
                </tr>
              </thead>
              <tbody>
                {items.map((item) => (
                  <tr key={item.id} className="border-t border-gray-800 text-gray-200 align-top">
                    <td className="px-2 py-2 text-gray-400 whitespace-nowrap">{new Date(item.created_at).toLocaleString()}</td>
                    <td className="px-2 py-2 font-mono">{item.card_id}</td>
                    <td className="px-2 py-2 font-mono">{item.module_name}</td>
                    <td className="px-2 py-2">{item.set_id}</td>
                    <td className="px-2 py-2">
                      {item.work_id ? (
                        <div className="space-y-1">
                          <span className="inline-block text-[10px] px-1.5 py-0.5 rounded bg-slate-700 text-gray-100">
                            {item.work_type ?? 'n/a'}
                          </span>
                          <div className="font-mono text-[11px] text-gray-300">{item.work_id}</div>
                        </div>
                      ) : (
                        <span className="text-gray-500">n/a</span>
                      )}
                    </td>
                    <td className="px-2 py-2 font-mono">{item.ai_task_id ?? 'n/a'}</td>
                    <td className="px-2 py-2">v{item.manifest_version}</td>
                    <td className="px-2 py-2">
                      <div className="text-[11px] text-gray-300">gen {summarizeHash(item.generated_hash)}</div>
                      <div className="text-[11px] text-gray-400">frz {summarizeHash(item.frozen_hash)}</div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      ))}
    </div>
  );
}
