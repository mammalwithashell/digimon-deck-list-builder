import { useState, useEffect, useCallback } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useTrainingStore } from '@/stores/trainingStore';
import {
  createGauntlet,
  startGauntlet,
  cancelGauntlet,
  getDeckPools,
  getMetaScopeStores,
  getMetaScopeScenes,
  getDeckLibraryArchetypes,
} from '@/api/trainingApi';
import type {
  GauntletItem,
  GauntletDetailItem,
  ArchetypeInfo,
  GauntletParticipantItem,
  DeckPoolItem,
  StoreInfo,
  SceneInfo,
} from '@/api/trainingApi';

// ── Types ──────────────────────────────────────────────────────────

interface DeckSlot {
  archetypeName: string;
  role: 'core' | 'exploiter';
  metaShare: number;
  threatIndex: number;
  deckSource: string;
  deck: string[];
  deckPoolId?: string | null;
}

type WizardStep = 1 | 2;

// ── Status badge helpers ───────────────────────────────────────────

const STATUS_COLORS: Record<GauntletItem['status'], string> = {
  configuring: 'bg-gray-600 text-gray-200',
  stage_1: 'bg-blue-600 text-blue-100',
  stage_2: 'bg-blue-600 text-blue-100',
  stage_3: 'bg-blue-600 text-blue-100',
  completed: 'bg-emerald-600 text-emerald-100',
  failed: 'bg-red-600 text-red-100',
  canceled: 'bg-gray-600 text-gray-200',
};

function StatusBadge({ status }: { status: GauntletItem['status'] }) {
  return (
    <span className={`inline-block px-2 py-0.5 rounded text-xs font-medium ${STATUS_COLORS[status]}`}>
      {status}
    </span>
  );
}

function RoleBadge({ role }: { role: 'core' | 'exploiter' }) {
  const cls = role === 'core'
    ? 'bg-blue-700/60 text-blue-200'
    : 'bg-amber-700/60 text-amber-200';
  return (
    <span className={`inline-block px-2 py-0.5 rounded text-xs font-medium ${cls}`}>
      {role}
    </span>
  );
}

// ── Heatmap color helper ───────────────────────────────────────────

function getCellColor(wr: number): string {
  if (wr >= 0.6) return 'bg-emerald-700/60 text-emerald-100';
  if (wr >= 0.45) return 'bg-yellow-700/60 text-yellow-100';
  return 'bg-red-700/60 text-red-100';
}

// ── Stage progress bar ─────────────────────────────────────────────

function StageProgressBar({ currentStage, status }: { currentStage: number; status: GauntletItem['status'] }) {
  const isTerminal = status === 'completed' || status === 'failed' || status === 'canceled';
  return (
    <div className="flex gap-1 h-8 rounded overflow-hidden">
      {[1, 2, 3].map(stage => (
        <div
          key={stage}
          className={`flex-1 flex items-center justify-center text-xs font-medium ${
            isTerminal && status === 'completed'
              ? 'bg-emerald-700 text-white'
              : stage < currentStage
                ? 'bg-emerald-700 text-white'
                : stage === currentStage && !isTerminal
                  ? 'bg-blue-600 text-white animate-pulse'
                  : 'bg-gray-700 text-gray-400'
          }`}
        >
          Stage {stage}
        </div>
      ))}
    </div>
  );
}

// ── Create Wizard Modal ────────────────────────────────────────────

function CreateWizardModal({
  archetypes,
  onClose,
  onCreated,
}: {
  archetypes: ArchetypeInfo[];
  onClose: () => void;
  onCreated: (id: string) => void;
}) {
  const [step, setStep] = useState<WizardStep>(1);
  const [name, setName] = useState('');
  const [stage1Games, setStage1Games] = useState(1000);
  const [stage2Games, setStage2Games] = useState(5000);
  const [stage3Games, setStage3Games] = useState(100);
  const [slots, setSlots] = useState<DeckSlot[]>([]);
  const [importingSlot, setImportingSlot] = useState<number | null>(null);
  const [importText, setImportText] = useState('');
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [deckPools, setDeckPools] = useState<DeckPoolItem[]>([]);

  // Meta scope state
  const [scopeMode, setScopeMode] = useState<'global' | 'store' | 'scene'>('global');
  const [stores, setStores] = useState<StoreInfo[]>([]);
  const [scenes, setScenes] = useState<SceneInfo[]>([]);
  const [selectedStoreIds, setSelectedStoreIds] = useState<number[]>([]);
  const [selectedSceneId, setSelectedSceneId] = useState<number | null>(null);
  const [scopedArchetypes, setScopedArchetypes] = useState<ArchetypeInfo[] | null>(null);
  const [scopeLoading, setScopeLoading] = useState(false);
  const [storeSearch, setStoreSearch] = useState('');

  // Fetch available deck pools on mount
  useEffect(() => {
    void getDeckPools().then(setDeckPools).catch(() => {});
  }, []);

  // Fetch stores/scenes when scope mode changes
  useEffect(() => {
    if (scopeMode === 'store' && stores.length === 0) {
      void getMetaScopeStores().then(setStores).catch(() => {});
    }
    if (scopeMode === 'scene' && scenes.length === 0) {
      void getMetaScopeScenes().then(setScenes).catch(() => {});
    }
  }, [scopeMode, stores.length, scenes.length]);

  // Re-fetch archetypes when scope selection changes
  useEffect(() => {
    if (scopeMode === 'global') {
      setScopedArchetypes(null);
      return;
    }
    if (scopeMode === 'store' && selectedStoreIds.length === 0) {
      setScopedArchetypes(null);
      return;
    }
    if (scopeMode === 'scene' && selectedSceneId === null) {
      setScopedArchetypes(null);
      return;
    }
    setScopeLoading(true);
    const params = scopeMode === 'store'
      ? { scope: 'store' as const, store_ids: selectedStoreIds.join(',') }
      : { scope: 'scene' as const, scene_id: selectedSceneId! };
    void getDeckLibraryArchetypes(params)
      .then(setScopedArchetypes)
      .catch(() => setScopedArchetypes(null))
      .finally(() => setScopeLoading(false));
  }, [scopeMode, selectedStoreIds, selectedSceneId]);

  // Use scoped archetypes when available, otherwise fall back to global
  const activeArchetypes = scopedArchetypes ?? archetypes;

  // Sort archetypes by threat_index descending for auto-suggestions
  const sortedArchetypes = [...activeArchetypes].sort((a, b) => b.threat_index - a.threat_index);
  const suggestedCore = sortedArchetypes.slice(0, 8);

  const coreSlots = slots.filter(s => s.role === 'core');
  const exploiterSlots = slots.filter(s => s.role === 'exploiter');

  const addFromLibrary = (archetype: ArchetypeInfo, role: 'core' | 'exploiter') => {
    if (role === 'core' && coreSlots.length >= 8) return;
    if (role === 'exploiter' && exploiterSlots.length >= 2) return;
    if (slots.length >= 10) return;

    setSlots(prev => [
      ...prev,
      {
        archetypeName: archetype.name,
        role,
        metaShare: archetype.meta_share,
        threatIndex: archetype.threat_index,
        deckSource: 'library',
        deck: archetype.sample_decklist,
      },
    ]);
  };

  const removeSlot = (index: number) => {
    setSlots(prev => prev.filter((_, i) => i !== index));
  };

  const updateSlotArchetype = (index: number, value: string) => {
    setSlots(prev => prev.map((slot, i) => {
      if (i !== index) return slot;
      const match = archetypes.find(a => a.name === value);
      return {
        ...slot,
        archetypeName: value,
        metaShare: match?.meta_share ?? slot.metaShare,
        threatIndex: match?.threat_index ?? slot.threatIndex,
        deck: match?.sample_decklist ?? slot.deck,
        deckSource: match ? 'library' : 'custom',
      };
    }));
  };

  const updateSlotDeckPool = (index: number, poolId: string | null) => {
    setSlots(prev => prev.map((slot, i) =>
      i === index ? { ...slot, deckPoolId: poolId } : slot
    ));
  };

  const finishImport = (slotIndex: number) => {
    const cardIds = importText
      .split(/[\n,]+/)
      .map(s => s.trim())
      .filter(Boolean);
    if (cardIds.length === 0) return;

    setSlots(prev => prev.map((slot, i) =>
      i === slotIndex
        ? { ...slot, deck: cardIds, deckSource: 'import' }
        : slot
    ));
    setImportText('');
    setImportingSlot(null);
  };

  const handleCreate = async () => {
    if (!name.trim()) {
      setError('Name is required');
      return;
    }
    if (slots.length === 0) {
      setError('At least one participant is required');
      return;
    }

    setCreating(true);
    setError(null);
    try {
      const result = await createGauntlet({
        name: name.trim(),
        participants: slots.map(slot => ({
          archetype_name: slot.archetypeName,
          deck: slot.deck,
          deck_source: slot.deckSource,
          deck_pool_id: slot.deckPoolId ?? undefined,
          role: slot.role,
          meta_share: slot.metaShare,
          threat_index: slot.threatIndex,
        })),
        stage1_games: stage1Games,
        stage2_games: stage2Games,
        stage3_games_per_matchup: stage3Games,
      });
      onCreated(result.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create gauntlet');
    } finally {
      setCreating(false);
    }
  };

  // Available archetypes not yet added
  const availableArchetypes = activeArchetypes.filter(
    a => !slots.some(s => s.archetypeName === a.name)
  );

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="bg-gray-900 border border-gray-700 rounded-lg w-full max-w-3xl max-h-[90vh] overflow-y-auto p-6 space-y-5">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold text-white">New Gauntlet</h2>
          <button onClick={onClose} className="text-gray-400 hover:text-white text-sm">
            Close
          </button>
        </div>

        {/* Step indicator */}
        <div className="flex gap-2">
          {[1, 2].map(s => (
            <div
              key={s}
              className={`flex-1 h-1 rounded ${s <= step ? 'bg-blue-500' : 'bg-gray-700'}`}
            />
          ))}
        </div>

        {error && (
          <div className="text-red-400 text-sm">{error}</div>
        )}

        {/* ── Step 1: Configuration ─────────────────────────── */}
        {step === 1 && (
          <div className="space-y-4">
            <div>
              <label className="block text-sm text-gray-300 mb-1">Gauntlet Name</label>
              <input
                value={name}
                onChange={e => setName(e.target.value)}
                placeholder="e.g. BT-18 Meta Gauntlet"
                className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-sm text-white"
              />
            </div>
            <div className="grid grid-cols-3 gap-3">
              <div>
                <label className="block text-sm text-gray-300 mb-1">Stage 1 Games</label>
                <input
                  type="number"
                  value={stage1Games}
                  onChange={e => setStage1Games(Number(e.target.value) || 0)}
                  className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-sm text-white"
                />
              </div>
              <div>
                <label className="block text-sm text-gray-300 mb-1">Stage 2 Games</label>
                <input
                  type="number"
                  value={stage2Games}
                  onChange={e => setStage2Games(Number(e.target.value) || 0)}
                  className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-sm text-white"
                />
              </div>
              <div>
                <label className="block text-sm text-gray-300 mb-1">Stage 3 Games / Matchup</label>
                <input
                  type="number"
                  value={stage3Games}
                  onChange={e => setStage3Games(Number(e.target.value) || 0)}
                  className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-sm text-white"
                />
              </div>
            </div>
            <div className="flex justify-end">
              <button
                onClick={() => setStep(2)}
                disabled={!name.trim()}
                className="px-4 py-2 rounded bg-blue-600 hover:bg-blue-500 disabled:bg-gray-700 disabled:text-gray-500 text-white text-sm"
              >
                Next
              </button>
            </div>
          </div>
        )}

        {/* ── Step 2: Deck Selection ────────────────────────── */}
        {step === 2 && (
          <div className="space-y-4">
            {/* Meta Scope selector */}
            <div className="border border-gray-700 rounded p-3 bg-gray-800/50 space-y-3">
              <div className="text-xs font-medium text-gray-400">Meta Scope</div>
              <div className="flex gap-3">
                {(['global', 'store', 'scene'] as const).map(mode => (
                  <label key={mode} className="flex items-center gap-1.5 text-sm text-gray-300 cursor-pointer">
                    <input
                      type="radio"
                      name="metaScope"
                      checked={scopeMode === mode}
                      onChange={() => { setScopeMode(mode); setSlots([]); }}
                      className="accent-blue-500"
                    />
                    {mode === 'global' ? 'Global' : mode === 'store' ? 'By Store' : 'By Scene'}
                  </label>
                ))}
              </div>

              {scopeMode === 'store' && (
                <div className="space-y-2">
                  <input
                    type="text"
                    placeholder="Search stores..."
                    value={storeSearch}
                    onChange={e => setStoreSearch(e.target.value)}
                    className="w-full px-2 py-1 rounded bg-gray-900 border border-gray-600 text-sm text-white placeholder-gray-500"
                  />
                  <div className="max-h-40 overflow-y-auto space-y-1">
                    {stores
                      .filter(s => !storeSearch || s.name.toLowerCase().includes(storeSearch.toLowerCase()) || s.city.toLowerCase().includes(storeSearch.toLowerCase()))
                      .map(s => (
                        <label key={s.store_id} className="flex items-center gap-2 text-xs text-gray-300 cursor-pointer hover:bg-gray-700/50 px-1 py-0.5 rounded">
                          <input
                            type="checkbox"
                            checked={selectedStoreIds.includes(s.store_id)}
                            onChange={e => {
                              setSelectedStoreIds(prev =>
                                e.target.checked
                                  ? [...prev, s.store_id]
                                  : prev.filter(id => id !== s.store_id)
                              );
                            }}
                            className="accent-blue-500"
                          />
                          {s.name} <span className="text-gray-500">({s.city}, {s.state}) — {s.tournament_count} events</span>
                        </label>
                      ))}
                  </div>
                </div>
              )}

              {scopeMode === 'scene' && (
                <select
                  value={selectedSceneId ?? ''}
                  onChange={e => setSelectedSceneId(e.target.value ? Number(e.target.value) : null)}
                  className="w-full px-2 py-1 rounded bg-gray-900 border border-gray-600 text-sm text-white"
                >
                  <option value="">Select a scene...</option>
                  {scenes.map(s => (
                    <option key={s.scene_id} value={s.scene_id}>
                      {s.display_name} ({s.tournament_count} events)
                    </option>
                  ))}
                </select>
              )}

              {scopeLoading && <div className="text-xs text-blue-400">Loading scoped meta...</div>}

              {scopeMode !== 'global' && scopedArchetypes && (
                <div className="text-xs text-green-400">
                  Showing {scopedArchetypes.length} archetypes with local data
                  {scopeMode === 'store' && selectedStoreIds.length > 0 && (
                    <> from {stores.filter(s => selectedStoreIds.includes(s.store_id)).map(s => s.name).join(' + ')}</>
                  )}
                  {scopeMode === 'scene' && selectedSceneId && (
                    <> from {scenes.find(s => s.scene_id === selectedSceneId)?.display_name}</>
                  )}
                </div>
              )}
            </div>

            {/* Auto-suggestion banner */}
            {slots.length === 0 && suggestedCore.length > 0 && (
              <div className="border border-gray-700 rounded p-3 bg-gray-800/50 space-y-2">
                <div className="text-xs text-gray-400">
                  Suggested core decks (top 8 by threat index):
                </div>
                <div className="flex flex-wrap gap-2">
                  {suggestedCore.map(arch => (
                    <button
                      key={arch.name}
                      onClick={() => addFromLibrary(arch, 'core')}
                      className="px-2 py-1 rounded bg-gray-700 hover:bg-gray-600 text-xs text-white"
                    >
                      {arch.name}
                    </button>
                  ))}
                </div>
              </div>
            )}

            {/* Core Decks section */}
            <div>
              <h3 className="text-sm font-medium text-gray-300 mb-2">
                Core Decks ({coreSlots.length}/8)
              </h3>
              <div className="space-y-2">
                {slots.map((slot, index) => {
                  if (slot.role !== 'core') return null;
                  return (
                    <SlotCard
                      key={index}
                      slot={slot}
                      index={index}
                      archetypes={activeArchetypes}
                      deckPools={deckPools}
                      onUpdateArchetype={updateSlotArchetype}
                      onUpdateDeckPool={updateSlotDeckPool}
                      onRemove={removeSlot}
                      isImporting={importingSlot === index}
                      onStartImport={() => setImportingSlot(index)}
                      importText={importText}
                      onImportTextChange={setImportText}
                      onFinishImport={() => finishImport(index)}
                      onCancelImport={() => { setImportingSlot(null); setImportText(''); }}
                    />
                  );
                })}
              </div>
            </div>

            {/* Exploiter Decks section */}
            <div>
              <h3 className="text-sm font-medium text-gray-300 mb-2">
                Exploiter Decks ({exploiterSlots.length}/2)
              </h3>
              <div className="space-y-2">
                {slots.map((slot, index) => {
                  if (slot.role !== 'exploiter') return null;
                  return (
                    <SlotCard
                      key={index}
                      slot={slot}
                      index={index}
                      archetypes={activeArchetypes}
                      deckPools={deckPools}
                      onUpdateArchetype={updateSlotArchetype}
                      onUpdateDeckPool={updateSlotDeckPool}
                      onRemove={removeSlot}
                      isImporting={importingSlot === index}
                      onStartImport={() => setImportingSlot(index)}
                      importText={importText}
                      onImportTextChange={setImportText}
                      onFinishImport={() => finishImport(index)}
                      onCancelImport={() => { setImportingSlot(null); setImportText(''); }}
                    />
                  );
                })}
              </div>
            </div>

            {/* Add controls */}
            <div className="flex flex-wrap gap-2 items-center">
              {availableArchetypes.length > 0 && (
                <div className="flex items-center gap-2">
                  <select
                    defaultValue=""
                    onChange={e => {
                      const arch = activeArchetypes.find(a => a.name === e.target.value);
                      if (!arch) return;
                      const role = coreSlots.length < 8 ? 'core' : 'exploiter';
                      addFromLibrary(arch, role);
                      e.target.value = '';
                    }}
                    className="bg-gray-800 border border-gray-700 rounded px-2 py-1.5 text-sm text-white"
                  >
                    <option value="" disabled>Add from Library...</option>
                    {availableArchetypes.map(a => (
                      <option key={a.name} value={a.name}>
                        {a.name} (TI: {a.threat_index.toFixed(2)}, share: {(a.meta_share * 100).toFixed(1)}%{a.local_times_played != null ? `, ${a.local_times_played} local` : ''})
                      </option>
                    ))}
                  </select>
                </div>
              )}
              {slots.length < 10 && (
                <button
                  onClick={() => {
                    const role = coreSlots.length < 8 ? 'core' : 'exploiter';
                    setSlots(prev => [
                      ...prev,
                      {
                        archetypeName: '',
                        role,
                        metaShare: 0,
                        threatIndex: 0,
                        deckSource: 'custom',
                        deck: [],
                      },
                    ]);
                  }}
                  className="px-3 py-1.5 rounded bg-gray-700 hover:bg-gray-600 text-xs text-white"
                >
                  + Empty Slot
                </button>
              )}
            </div>

            {/* Navigation */}
            <div className="flex justify-between pt-2">
              <button
                onClick={() => setStep(1)}
                className="px-4 py-2 rounded bg-gray-700 hover:bg-gray-600 text-white text-sm"
              >
                Back
              </button>
              <button
                onClick={() => void handleCreate()}
                disabled={creating || slots.length === 0}
                className="px-4 py-2 rounded bg-emerald-600 hover:bg-emerald-500 disabled:bg-gray-700 disabled:text-gray-500 text-white text-sm"
              >
                {creating ? 'Creating...' : 'Create Gauntlet'}
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

// ── Slot Card ──────────────────────────────────────────────────────

function SlotCard({
  slot,
  index,
  archetypes,
  deckPools,
  onUpdateArchetype,
  onUpdateDeckPool,
  onRemove,
  isImporting,
  onStartImport,
  importText,
  onImportTextChange,
  onFinishImport,
  onCancelImport,
}: {
  slot: DeckSlot;
  index: number;
  archetypes: ArchetypeInfo[];
  deckPools: DeckPoolItem[];
  onUpdateArchetype: (index: number, value: string) => void;
  onUpdateDeckPool: (index: number, poolId: string | null) => void;
  onRemove: (index: number) => void;
  isImporting: boolean;
  onStartImport: () => void;
  importText: string;
  onImportTextChange: (text: string) => void;
  onFinishImport: () => void;
  onCancelImport: () => void;
}) {
  const sourceLabel =
    slot.deckSource === 'library' ? 'Library'
      : slot.deckSource === 'import' ? 'Imported'
        : slot.deckPoolId ? 'Pool'
          : 'Custom';

  // Filter pools matching the slot archetype (or show all if no archetype selected)
  const matchingPools = slot.archetypeName
    ? deckPools.filter(p => p.archetype === slot.archetypeName)
    : deckPools;

  return (
    <div className="border border-gray-700 rounded p-3 bg-gray-800 space-y-2">
      <div className="flex items-center gap-3">
        <div className="flex-1">
          <select
            value={slot.archetypeName}
            onChange={e => onUpdateArchetype(index, e.target.value)}
            className="w-full bg-gray-900 border border-gray-700 rounded px-2 py-1.5 text-sm text-white"
          >
            <option value="">-- Select Archetype --</option>
            {archetypes.map(a => (
              <option key={a.name} value={a.name}>{a.name}</option>
            ))}
            {slot.archetypeName && !archetypes.find(a => a.name === slot.archetypeName) && (
              <option value={slot.archetypeName}>{slot.archetypeName} (custom)</option>
            )}
          </select>
        </div>
        <div className="text-xs text-gray-400 whitespace-nowrap">
          {(slot.metaShare * 100).toFixed(1)}% share
        </div>
        <div className="text-xs text-gray-500 whitespace-nowrap">
          {sourceLabel}
        </div>
        <button
          onClick={onStartImport}
          className="px-2 py-1 rounded bg-gray-700 hover:bg-gray-600 text-xs text-white"
        >
          Import Deck
        </button>
        <button
          onClick={() => onRemove(index)}
          className="px-2 py-1 rounded bg-red-800/60 hover:bg-red-700/60 text-xs text-red-200"
        >
          Remove
        </button>
      </div>
      {/* Deck Pool selector */}
      {deckPools.length > 0 && (
        <div className="flex items-center gap-2">
          <label className="text-xs text-gray-500 whitespace-nowrap">Deck Pool:</label>
          <select
            value={slot.deckPoolId ?? ''}
            onChange={e => onUpdateDeckPool(index, e.target.value || null)}
            className="flex-1 bg-gray-900 border border-gray-700 rounded px-2 py-1 text-xs text-white"
          >
            <option value="">None</option>
            {(matchingPools.length > 0 ? matchingPools : deckPools).map(pool => (
              <option key={pool.id} value={pool.id}>
                {pool.name} ({pool.variant_count} variants)
              </option>
            ))}
          </select>
          {slot.deckPoolId && (
            <span className="inline-block px-1.5 py-0.5 rounded bg-purple-900/40 text-[10px] text-purple-300">
              pool
            </span>
          )}
        </div>
      )}
      {slot.deck.length > 0 && (
        <div className="text-xs text-gray-500">
          {slot.deck.length} card{slot.deck.length !== 1 ? 's' : ''} loaded
        </div>
      )}
      {isImporting && (
        <div className="space-y-2">
          <textarea
            value={importText}
            onChange={e => onImportTextChange(e.target.value)}
            placeholder="Paste card IDs (one per line or comma-separated)"
            rows={4}
            className="w-full bg-gray-900 border border-gray-700 rounded px-2 py-1.5 text-xs text-white font-mono"
          />
          <div className="flex gap-2">
            <button
              onClick={onFinishImport}
              className="px-2 py-1 rounded bg-emerald-700 hover:bg-emerald-600 text-xs text-white"
            >
              Apply
            </button>
            <button
              onClick={onCancelImport}
              className="px-2 py-1 rounded bg-gray-700 hover:bg-gray-600 text-xs text-white"
            >
              Cancel
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

// ── Matchup Matrix Heatmap ─────────────────────────────────────────

function MatchupMatrix({
  matrix,
  participants,
}: {
  matrix: Record<string, Record<string, number>>;
  participants: GauntletParticipantItem[];
}) {
  const coreParticipants = participants.filter(p => p.role === 'core');
  const labels = coreParticipants.map(p => p.archetype_name);
  const agentIds = coreParticipants.map(p => p.agent_id ?? p.id);

  if (labels.length === 0) return null;

  return (
    <div className="space-y-2">
      <h3 className="text-sm font-medium text-gray-300">Matchup Matrix</h3>
      <div className="overflow-x-auto">
        <div
          className="inline-grid gap-px bg-gray-800 rounded overflow-hidden"
          style={{
            gridTemplateColumns: `120px repeat(${labels.length}, 64px)`,
          }}
        >
          {/* Header row */}
          <div className="bg-gray-900" />
          {labels.map((label, i) => (
            <div
              key={`header-${i}`}
              className="bg-gray-900 flex items-end justify-center pb-1 h-20"
            >
              <span
                className="text-[10px] text-gray-300 font-medium whitespace-nowrap origin-bottom-left"
                style={{ transform: 'rotate(-45deg)', display: 'inline-block' }}
              >
                {label}
              </span>
            </div>
          ))}

          {/* Data rows */}
          {agentIds.map((rowId, rowIdx) => (
            <>
              <div
                key={`label-${rowIdx}`}
                className="bg-gray-900 flex items-center px-2 text-xs text-gray-300 font-medium truncate"
              >
                {labels[rowIdx]}
              </div>
              {agentIds.map((colId, colIdx) => {
                if (rowIdx === colIdx) {
                  return (
                    <div
                      key={`cell-${rowIdx}-${colIdx}`}
                      className="bg-gray-700/40 flex items-center justify-center text-[10px] text-gray-500"
                    >
                      --
                    </div>
                  );
                }
                const rowLabel = labels[rowIdx] ?? '';
                const colLabel = labels[colIdx] ?? '';
                const wr = matrix[rowId]?.[colId] ?? matrix[rowLabel]?.[colLabel] ?? null;
                if (wr === null) {
                  return (
                    <div
                      key={`cell-${rowIdx}-${colIdx}`}
                      className="bg-gray-800 flex items-center justify-center text-[10px] text-gray-600"
                    >
                      ?
                    </div>
                  );
                }
                return (
                  <div
                    key={`cell-${rowIdx}-${colIdx}`}
                    className={`flex items-center justify-center text-[11px] font-medium ${getCellColor(wr)}`}
                  >
                    {Math.round(wr * 100)}%
                  </div>
                );
              })}
            </>
          ))}
        </div>
      </div>
    </div>
  );
}

// ── Tournament Rankings ────────────────────────────────────────────

interface RankingEntry {
  archetype_name: string;
  avg_matchup_wr: number;
  expected_meta_wr: number;
  meta_share: number;
}

function TournamentRankings({
  rankings,
}: {
  rankings: Array<Record<string, unknown>>;
}) {
  const rows: RankingEntry[] = rankings
    .map(r => ({
      archetype_name: String(r.archetype_name ?? r.archetype ?? ''),
      avg_matchup_wr: Number(r.avg_matchup_wr ?? r.tournament_win_rate ?? 0),
      expected_meta_wr: Number(r.expected_meta_wr ?? r.expected_meta_win_rate ?? 0),
      meta_share: Number(r.meta_share ?? 0),
    }))
    .sort((a, b) => b.expected_meta_wr - a.expected_meta_wr);

  return (
    <div className="space-y-2">
      <h3 className="text-sm font-medium text-gray-300">Tournament Rankings</h3>
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="text-left text-xs text-gray-400 border-b border-gray-700">
              <th className="py-2 px-3 w-12">Rank</th>
              <th className="py-2 px-3">Archetype</th>
              <th className="py-2 px-3 text-right">Avg Matchup WR</th>
              <th className="py-2 px-3 text-right">Expected Meta WR</th>
              <th className="py-2 px-3 text-right">Meta Share</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row, idx) => (
              <tr
                key={row.archetype_name}
                className={`border-b border-gray-800 ${
                  idx < 3 ? 'bg-emerald-900/20' : ''
                }`}
              >
                <td className="py-2 px-3 text-gray-400 font-mono">{idx + 1}</td>
                <td className="py-2 px-3 text-white">{row.archetype_name}</td>
                <td className="py-2 px-3 text-right text-gray-300">
                  {(row.avg_matchup_wr * 100).toFixed(1)}%
                </td>
                <td className="py-2 px-3 text-right text-gray-300">
                  {(row.expected_meta_wr * 100).toFixed(1)}%
                </td>
                <td className="py-2 px-3 text-right text-gray-400">
                  {(row.meta_share * 100).toFixed(1)}%
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

// ── Detail View ────────────────────────────────────────────────────

function GauntletDetail({
  detail,
  onRefresh,
  onBack,
}: {
  detail: GauntletDetailItem;
  onRefresh: () => void;
  onBack: () => void;
}) {
  const { gauntlet, jobs, matchup_matrix, tournament_rankings } = detail;
  const [actionLoading, setActionLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleStart = async () => {
    setActionLoading(true);
    setError(null);
    try {
      await startGauntlet(gauntlet.id);
      onRefresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to start gauntlet');
    } finally {
      setActionLoading(false);
    }
  };

  const handleCancel = async () => {
    setActionLoading(true);
    setError(null);
    try {
      await cancelGauntlet(gauntlet.id);
      onRefresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to cancel gauntlet');
    } finally {
      setActionLoading(false);
    }
  };

  const isActive = ['stage_1', 'stage_2', 'stage_3'].includes(gauntlet.status);

  // Count jobs per stage
  const jobsByStage = [1, 2, 3].map(stage =>
    jobs.filter(j => j.gauntlet_stage === stage)
  );

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <button
            onClick={onBack}
            className="px-3 py-1.5 rounded bg-gray-700 hover:bg-gray-600 text-sm text-white"
          >
            Back
          </button>
          <h2 className="text-xl font-semibold text-white">{gauntlet.name}</h2>
          <StatusBadge status={gauntlet.status} />
        </div>
        <div className="flex gap-2">
          {gauntlet.status === 'configuring' && (
            <button
              onClick={() => void handleStart()}
              disabled={actionLoading}
              className="px-4 py-2 rounded bg-emerald-600 hover:bg-emerald-500 disabled:bg-gray-700 text-sm text-white"
            >
              {actionLoading ? 'Starting...' : 'Start Gauntlet'}
            </button>
          )}
          {isActive && (
            <button
              onClick={() => void handleCancel()}
              disabled={actionLoading}
              className="px-4 py-2 rounded bg-red-700 hover:bg-red-600 disabled:bg-gray-700 text-sm text-white"
            >
              {actionLoading ? 'Canceling...' : 'Cancel Gauntlet'}
            </button>
          )}
        </div>
      </div>

      {error && <div className="text-red-400 text-sm">{error}</div>}

      {gauntlet.error_text && (
        <div className="text-red-400 text-sm border border-red-800 rounded p-3 bg-red-950/30">
          {gauntlet.error_text}
        </div>
      )}

      {/* Stage Progress */}
      <StageProgressBar currentStage={gauntlet.current_stage} status={gauntlet.status} />
      <div className="grid grid-cols-3 gap-1 text-xs text-gray-400 text-center">
        {[0, 1, 2].map(i => {
          const stageJobs = jobsByStage[i] ?? [];
          return (
            <div key={i}>
              {stageJobs.length} job{stageJobs.length !== 1 ? 's' : ''}
              {' / '}
              {gauntlet[`stage${i + 1}_games` as 'stage1_games' | 'stage2_games'] ??
                gauntlet.stage3_games_per_matchup}{' '}
              games
            </div>
          );
        })}
      </div>

      {/* Participants Table */}
      <div className="space-y-2">
        <h3 className="text-sm font-medium text-gray-300">Participants</h3>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-xs text-gray-400 border-b border-gray-700">
                <th className="py-2 px-3">Archetype</th>
                <th className="py-2 px-3">Role</th>
                <th className="py-2 px-3">Agent</th>
                <th className="py-2 px-3 text-right">Stage 1 WR</th>
                <th className="py-2 px-3 text-right">Stage 2 WR</th>
              </tr>
            </thead>
            <tbody>
              {gauntlet.participants.map(p => {
                // Find win rates from jobs
                const s1Jobs = jobs.filter(
                  j => j.gauntlet_stage === 1 && j.agent_id === p.agent_id
                );
                const s2Jobs = jobs.filter(
                  j => j.gauntlet_stage === 2 && j.agent_id === p.agent_id
                );
                const s1Wr = s1Jobs.length > 0
                  ? s1Jobs.reduce((sum, j) => sum + (j.win_rate ?? 0), 0) / s1Jobs.length
                  : null;
                const s2Wr = s2Jobs.length > 0
                  ? s2Jobs.reduce((sum, j) => sum + (j.win_rate ?? 0), 0) / s2Jobs.length
                  : null;

                return (
                  <tr key={p.id} className="border-b border-gray-800">
                    <td className="py-2 px-3 text-white">{p.archetype_name}</td>
                    <td className="py-2 px-3">
                      <RoleBadge role={p.role} />
                    </td>
                    <td className="py-2 px-3 text-gray-300 text-xs font-mono">
                      {p.agent_id ? p.agent_id.slice(0, 8) : (
                        <span className="text-gray-500 italic">pending</span>
                      )}
                    </td>
                    <td className="py-2 px-3 text-right text-gray-300">
                      {s1Wr !== null ? `${(s1Wr * 100).toFixed(1)}%` : '--'}
                    </td>
                    <td className="py-2 px-3 text-right text-gray-300">
                      {s2Wr !== null ? `${(s2Wr * 100).toFixed(1)}%` : '--'}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>

      {/* Matchup Matrix - visible after Stage 3 */}
      {matchup_matrix && Object.keys(matchup_matrix).length > 0 && (
        <MatchupMatrix matrix={matchup_matrix} participants={gauntlet.participants} />
      )}

      {/* Tournament Rankings - visible after Stage 3 */}
      {tournament_rankings && tournament_rankings.length > 0 && (
        <TournamentRankings rankings={tournament_rankings} />
      )}
    </div>
  );
}

// ── Main Page ──────────────────────────────────────────────────────

export function GauntletPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();

  const {
    gauntlets,
    selectedGauntlet,
    deckLibraryArchetypes,
    isLoading,
    error,
    fetchGauntlets,
    fetchGauntletDetail,
    fetchDeckLibrary,
    clearError,
  } = useTrainingStore();

  const [showWizard, setShowWizard] = useState(false);

  // Initial data load
  useEffect(() => {
    void fetchGauntlets();
    void fetchDeckLibrary();
  }, [fetchGauntlets, fetchDeckLibrary]);

  // Load detail when id param present
  useEffect(() => {
    if (id) {
      void fetchGauntletDetail(id);
    }
  }, [id, fetchGauntletDetail]);

  // Poll active gauntlets
  useEffect(() => {
    if (
      selectedGauntlet &&
      ['stage_1', 'stage_2', 'stage_3'].includes(selectedGauntlet.gauntlet.status)
    ) {
      const interval = setInterval(() => fetchGauntletDetail(id!), 5000);
      return () => clearInterval(interval);
    }
  }, [selectedGauntlet?.gauntlet.status, id, fetchGauntletDetail]);

  const handleRowClick = useCallback(
    (gauntletId: string) => {
      navigate(`/admin/gauntlet/${gauntletId}`);
    },
    [navigate],
  );

  const handleBack = useCallback(() => {
    navigate('/admin/gauntlet');
  }, [navigate]);

  const handleCreated = useCallback(
    (newId: string) => {
      setShowWizard(false);
      void fetchGauntlets();
      navigate(`/admin/gauntlet/${newId}`);
    },
    [fetchGauntlets, navigate],
  );

  const handleRefresh = useCallback(() => {
    if (id) {
      void fetchGauntletDetail(id);
    }
  }, [id, fetchGauntletDetail]);

  // ── Detail View ──────────────────────────────────────────────
  if (id && selectedGauntlet) {
    return (
      <div className="max-w-7xl mx-auto px-4 py-6">
        <GauntletDetail
          detail={selectedGauntlet}
          onRefresh={handleRefresh}
          onBack={handleBack}
        />
      </div>
    );
  }

  // ── List View ────────────────────────────────────────────────
  return (
    <div className="max-w-7xl mx-auto px-4 py-6 space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold text-white">Gauntlet Manager</h1>
        <button
          onClick={() => setShowWizard(true)}
          className="px-4 py-2 rounded bg-blue-600 hover:bg-blue-500 text-sm text-white"
        >
          New Gauntlet
        </button>
      </div>

      {error && (
        <div className="flex items-center justify-between text-red-400 text-sm border border-red-800 rounded p-3 bg-red-950/30">
          <span>{error}</span>
          <button onClick={clearError} className="text-red-300 hover:text-white text-xs ml-4">
            Dismiss
          </button>
        </div>
      )}

      {isLoading && gauntlets.length === 0 ? (
        <div className="text-gray-400 text-sm">Loading gauntlets...</div>
      ) : gauntlets.length === 0 ? (
        <div className="text-gray-500 text-sm border border-gray-700 rounded p-8 text-center">
          No gauntlets yet. Create one to get started.
        </div>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-xs text-gray-400 border-b border-gray-700">
                <th className="py-2 px-3">Name</th>
                <th className="py-2 px-3">Status</th>
                <th className="py-2 px-3">Stage</th>
                <th className="py-2 px-3">Created</th>
              </tr>
            </thead>
            <tbody>
              {gauntlets.map(g => (
                <tr
                  key={g.id}
                  onClick={() => handleRowClick(g.id)}
                  className="border-b border-gray-800 hover:bg-gray-800/50 cursor-pointer transition-colors"
                >
                  <td className="py-3 px-3 text-white font-medium">{g.name}</td>
                  <td className="py-3 px-3">
                    <StatusBadge status={g.status} />
                  </td>
                  <td className="py-3 px-3">
                    <div className="flex gap-0.5">
                      {[1, 2, 3].map(stage => (
                        <div
                          key={stage}
                          className={`w-6 h-5 rounded-sm flex items-center justify-center text-[10px] font-medium ${
                            stage < g.current_stage
                              ? 'bg-emerald-700 text-white'
                              : stage === g.current_stage &&
                                  ['stage_1', 'stage_2', 'stage_3'].includes(g.status)
                                ? 'bg-blue-600 text-white'
                                : g.status === 'completed'
                                  ? 'bg-emerald-700 text-white'
                                  : 'bg-gray-700 text-gray-500'
                          }`}
                        >
                          {stage}
                        </div>
                      ))}
                    </div>
                  </td>
                  <td className="py-3 px-3 text-gray-400 text-xs">
                    {new Date(g.created_at).toLocaleDateString()}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {/* Create Wizard Modal */}
      {showWizard && (
        <CreateWizardModal
          archetypes={deckLibraryArchetypes}
          onClose={() => setShowWizard(false)}
          onCreated={handleCreated}
        />
      )}
    </div>
  );
}
