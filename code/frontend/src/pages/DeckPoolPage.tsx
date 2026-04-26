import { useState, useEffect, useCallback } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useTrainingStore } from '@/stores/trainingStore';
import {
  createDeckPool,
  deleteDeckPool,
  generateDeckPoolVariants,
} from '@/api/trainingApi';
import type {
  DeckPoolDetail,
  DeckPoolVariant,
  CoreAnalysis,
} from '@/api/trainingApi';

// ── Helpers ───────────────────────────────────────────────────────

function ModeBadge({ mode }: { mode: 'eager' | 'hybrid' }) {
  const cls = mode === 'eager'
    ? 'bg-blue-700/60 text-blue-200'
    : 'bg-purple-700/60 text-purple-200';
  return (
    <span className={`inline-block px-2 py-0.5 rounded text-xs font-medium ${cls}`}>
      {mode}
    </span>
  );
}

/**
 * Parse a deck textarea. Accepts lines like:
 *   "4 CardName BT24-017" -> extracts "BT24-017"
 *   "BT24-017"            -> takes as-is
 *   comma-separated        -> splits by commas
 */
function parseDeckText(text: string): string[] {
  return text
    .split(/[\n,]+/)
    .map(line => line.trim())
    .filter(Boolean)
    .map(line => {
      // Match pattern like "4 CardName BT24-017" -> last token that looks like a card ID
      const tokens = line.split(/\s+/);
      if (tokens.length >= 3) {
        const last = tokens[tokens.length - 1] ?? '';
        // Card IDs typically look like BT24-017, ST19-01, EX11-062, P-123
        if (/^[A-Z]{1,3}\d{0,3}-\d{2,3}$/.test(last)) {
          // Repeat based on the leading count
          const count = parseInt(tokens[0] ?? '', 10);
          if (!isNaN(count) && count >= 1 && count <= 4) {
            return Array(count).fill(last);
          }
          return [last];
        }
      }
      // Single token -- treat as card ID directly
      return [line];
    })
    .flat();
}

// ── Create Modal ──────────────────────────────────────────────────

function CreatePoolModal({
  onClose,
  onCreated,
}: {
  onClose: () => void;
  onCreated: (id: string) => void;
}) {
  const [name, setName] = useState('');
  const [archetype, setArchetype] = useState('');
  const [baseDeckText, setBaseDeckText] = useState('');
  const [sideCardsText, setSideCardsText] = useState('');
  const [generationMode, setGenerationMode] = useState<'eager' | 'hybrid'>('eager');
  const [variantCount, setVariantCount] = useState(4);
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleCreate = async () => {
    if (!name.trim()) {
      setError('Name is required');
      return;
    }
    if (!archetype.trim()) {
      setError('Archetype is required');
      return;
    }
    const baseDeck = parseDeckText(baseDeckText);
    if (baseDeck.length === 0) {
      setError('Base deck is required');
      return;
    }
    const sideCards = parseDeckText(sideCardsText);

    setCreating(true);
    setError(null);
    try {
      const result = await createDeckPool({
        name: name.trim(),
        archetype: archetype.trim(),
        base_deck: baseDeck,
        side_cards: sideCards.length > 0 ? sideCards : undefined,
        generation_mode: generationMode,
        target_variant_count: variantCount,
      });
      onCreated(result.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create deck pool');
    } finally {
      setCreating(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="bg-gray-900 border border-gray-700 rounded-lg w-full max-w-2xl max-h-[90vh] overflow-y-auto p-6 space-y-5">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold text-white">New Deck Pool</h2>
          <button onClick={onClose} className="text-gray-400 hover:text-white text-sm">
            Close
          </button>
        </div>

        {error && <div className="text-red-400 text-sm">{error}</div>}

        <div className="space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm text-gray-300 mb-1">Name</label>
              <input
                value={name}
                onChange={e => setName(e.target.value)}
                placeholder="e.g. Imperialdramon Variants"
                className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-sm text-white"
              />
            </div>
            <div>
              <label className="block text-sm text-gray-300 mb-1">Archetype</label>
              <input
                value={archetype}
                onChange={e => setArchetype(e.target.value)}
                placeholder="e.g. Imperialdramon"
                className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-sm text-white"
              />
            </div>
          </div>

          <div>
            <label className="block text-sm text-gray-300 mb-1">
              Base Deck
              <span className="text-gray-500 ml-1">(one card per line: "4 CardName BT24-017" or "BT24-017")</span>
            </label>
            <textarea
              value={baseDeckText}
              onChange={e => setBaseDeckText(e.target.value)}
              rows={8}
              placeholder={"4 Agumon BT15-038\n4 Greymon BT15-040\nBT15-042"}
              className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-sm text-white font-mono"
            />
          </div>

          <div>
            <label className="block text-sm text-gray-300 mb-1">
              Side Cards
              <span className="text-gray-500 ml-1">(optional, same format)</span>
            </label>
            <textarea
              value={sideCardsText}
              onChange={e => setSideCardsText(e.target.value)}
              rows={4}
              placeholder="BT15-043&#10;BT15-044"
              className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-sm text-white font-mono"
            />
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm text-gray-300 mb-1">Generation Mode</label>
              <div className="flex gap-4 mt-1">
                <label className="flex items-center gap-2 text-sm text-white cursor-pointer">
                  <input
                    type="radio"
                    name="generationMode"
                    value="eager"
                    checked={generationMode === 'eager'}
                    onChange={() => setGenerationMode('eager')}
                    className="accent-blue-500"
                  />
                  Eager
                </label>
                <label className="flex items-center gap-2 text-sm text-white cursor-pointer">
                  <input
                    type="radio"
                    name="generationMode"
                    value="hybrid"
                    checked={generationMode === 'hybrid'}
                    onChange={() => setGenerationMode('hybrid')}
                    className="accent-purple-500"
                  />
                  Hybrid
                </label>
              </div>
            </div>
            <div>
              <label className="block text-sm text-gray-300 mb-1">Variant Count</label>
              <input
                type="number"
                value={variantCount}
                onChange={e => setVariantCount(Number(e.target.value) || 1)}
                min={1}
                max={100}
                className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-sm text-white"
              />
            </div>
          </div>
        </div>

        <div className="flex justify-end gap-3 pt-2">
          <button
            onClick={onClose}
            className="px-4 py-2 rounded bg-gray-700 hover:bg-gray-600 text-sm text-white"
          >
            Cancel
          </button>
          <button
            onClick={() => void handleCreate()}
            disabled={creating}
            className="px-4 py-2 rounded bg-blue-600 hover:bg-blue-500 disabled:bg-gray-700 disabled:text-gray-500 text-sm text-white"
          >
            {creating ? 'Creating...' : 'Create Pool'}
          </button>
        </div>
      </div>
    </div>
  );
}

// ── Core Analysis Section ─────────────────────────────────────────

function CoreAnalysisSection({ analysis }: { analysis: CoreAnalysis }) {
  const coreEntries = Object.entries(analysis.core_cards);
  const flexEntries = Object.entries(analysis.flex_cards);

  return (
    <div className="space-y-3">
      <h3 className="text-sm font-medium text-gray-300">
        Core / Flex Analysis
        <span className="text-gray-500 ml-2 font-normal">
          ({analysis.total_main} main, {analysis.total_egg} egg)
        </span>
      </h3>

      {coreEntries.length > 0 && (
        <div>
          <div className="text-xs text-gray-400 mb-1">Core Cards</div>
          <div className="flex flex-wrap gap-1.5">
            {coreEntries.map(([cardId, count]) => (
              <span
                key={cardId}
                className="inline-flex items-center gap-1 px-2 py-0.5 rounded bg-blue-900/40 border border-blue-700/40 text-xs text-blue-200"
              >
                <span className="font-mono">{cardId}</span>
                <span className="text-blue-400">x{count}</span>
              </span>
            ))}
          </div>
        </div>
      )}

      {flexEntries.length > 0 && (
        <div>
          <div className="text-xs text-gray-400 mb-1">Flex Cards</div>
          <div className="flex flex-wrap gap-1.5">
            {flexEntries.map(([cardId, count]) => (
              <span
                key={cardId}
                className="inline-flex items-center gap-1 px-2 py-0.5 rounded bg-gray-800 border border-gray-700 text-xs text-gray-300"
              >
                <span className="font-mono">{cardId}</span>
                <span className="text-gray-500">x{count}</span>
              </span>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

// ── Variant Row ───────────────────────────────────────────────────

function VariantRow({ variant }: { variant: DeckPoolVariant }) {
  const [expanded, setExpanded] = useState(false);
  const changes = Object.entries(variant.changes_from_base);
  const additions = changes.filter(([, count]) => count > 0);
  const removals = changes.filter(([, count]) => count < 0);

  return (
    <div className="border border-gray-700 rounded bg-gray-800">
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full px-3 py-2 flex items-center justify-between text-sm text-left hover:bg-gray-750"
      >
        <span className="text-gray-200">
          Variant #{variant.index}
          <span className="text-gray-500 ml-2">
            ({variant.deck.length} cards, {changes.length} change{changes.length !== 1 ? 's' : ''})
          </span>
        </span>
        <span className="text-gray-500 text-xs">{expanded ? 'Collapse' : 'Expand'}</span>
      </button>
      {expanded && (
        <div className="px-3 pb-3 space-y-2">
          {/* Changes summary */}
          {changes.length > 0 && (
            <div className="flex flex-wrap gap-1.5">
              {additions.map(([cardId, count]) => (
                <span
                  key={cardId}
                  className="inline-flex items-center gap-1 px-2 py-0.5 rounded bg-emerald-900/30 text-xs text-emerald-300"
                >
                  +{count} {cardId}
                </span>
              ))}
              {removals.map(([cardId, count]) => (
                <span
                  key={cardId}
                  className="inline-flex items-center gap-1 px-2 py-0.5 rounded bg-red-900/30 text-xs text-red-300"
                >
                  {count} {cardId}
                </span>
              ))}
            </div>
          )}
          {changes.length === 0 && (
            <div className="text-xs text-gray-500">No changes from base deck</div>
          )}
          {/* Full deck list */}
          <details className="text-xs">
            <summary className="text-gray-500 cursor-pointer hover:text-gray-300">
              Full deck list ({variant.deck.length} cards)
            </summary>
            <div className="mt-1 font-mono text-gray-400 whitespace-pre-wrap">
              {variant.deck.join('\n')}
            </div>
          </details>
        </div>
      )}
    </div>
  );
}

// ── Detail View ───────────────────────────────────────────────────

function DeckPoolDetailView({
  detail,
  onRefresh,
  onBack,
}: {
  detail: DeckPoolDetail;
  onRefresh: () => void;
  onBack: () => void;
}) {
  const { pool, core_analysis, variants } = detail;
  const [actionLoading, setActionLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleRegenerate = async () => {
    setActionLoading(true);
    setError(null);
    try {
      await generateDeckPoolVariants(pool.id);
      onRefresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to regenerate variants');
    } finally {
      setActionLoading(false);
    }
  };

  const handleDelete = async () => {
    if (!window.confirm(`Delete pool "${pool.name}"? This cannot be undone.`)) return;
    setActionLoading(true);
    setError(null);
    try {
      await deleteDeckPool(pool.id);
      onBack();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to delete pool');
    } finally {
      setActionLoading(false);
    }
  };

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
          <h2 className="text-xl font-semibold text-white">{pool.name}</h2>
          <ModeBadge mode={pool.generation_mode} />
        </div>
        <div className="flex gap-2">
          <button
            onClick={() => void handleRegenerate()}
            disabled={actionLoading}
            className="px-4 py-2 rounded bg-blue-600 hover:bg-blue-500 disabled:bg-gray-700 text-sm text-white"
          >
            {actionLoading ? 'Working...' : 'Regenerate'}
          </button>
          <button
            onClick={() => void handleDelete()}
            disabled={actionLoading}
            className="px-4 py-2 rounded bg-red-700 hover:bg-red-600 disabled:bg-gray-700 text-sm text-white"
          >
            Delete
          </button>
        </div>
      </div>

      {error && <div className="text-red-400 text-sm">{error}</div>}

      {/* Pool Info */}
      <div className="grid grid-cols-4 gap-4 text-sm">
        <div>
          <div className="text-xs text-gray-500">Archetype</div>
          <div className="text-white">{pool.archetype}</div>
        </div>
        <div>
          <div className="text-xs text-gray-500">Variants</div>
          <div className="text-white">{pool.variant_count} / {pool.target_variant_count}</div>
        </div>
        <div>
          <div className="text-xs text-gray-500">Base Deck</div>
          <div className="text-white">{pool.base_deck.length} cards</div>
        </div>
        <div>
          <div className="text-xs text-gray-500">Created</div>
          <div className="text-white">{new Date(pool.created_at).toLocaleDateString()}</div>
        </div>
      </div>

      {/* Core / Flex Analysis */}
      {core_analysis && <CoreAnalysisSection analysis={core_analysis} />}

      {/* Side Cards */}
      {pool.side_cards.length > 0 && (
        <div className="space-y-2">
          <h3 className="text-sm font-medium text-gray-300">Side Cards</h3>
          <div className="flex flex-wrap gap-1.5">
            {pool.side_cards.map((cardId, i) => (
              <span
                key={`${cardId}-${i}`}
                className="inline-block px-2 py-0.5 rounded bg-gray-800 border border-gray-700 text-xs text-gray-300 font-mono"
              >
                {cardId}
              </span>
            ))}
          </div>
        </div>
      )}

      {/* Variants */}
      <div className="space-y-2">
        <h3 className="text-sm font-medium text-gray-300">
          Variants ({variants.length})
        </h3>
        {variants.length === 0 ? (
          <div className="text-gray-500 text-sm border border-gray-700 rounded p-4 text-center">
            No variants generated yet. Click "Regenerate" to create variants.
          </div>
        ) : (
          <div className="space-y-1">
            {variants.map(v => (
              <VariantRow key={v.index} variant={v} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

// ── Main Page ─────────────────────────────────────────────────────

export function DeckPoolPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();

  const {
    deckPools,
    selectedDeckPool,
    isLoading,
    error,
    fetchDeckPools,
    fetchDeckPoolDetail,
    clearError,
  } = useTrainingStore();

  const [showCreate, setShowCreate] = useState(false);

  // Initial data load
  useEffect(() => {
    void fetchDeckPools();
  }, [fetchDeckPools]);

  // Load detail when id param present
  useEffect(() => {
    if (id) {
      void fetchDeckPoolDetail(id);
    }
  }, [id, fetchDeckPoolDetail]);

  const handleRowClick = useCallback(
    (poolId: string) => {
      navigate(`/admin/deck-pools/${poolId}`);
    },
    [navigate],
  );

  const handleBack = useCallback(() => {
    navigate('/admin/deck-pools');
  }, [navigate]);

  const handleCreated = useCallback(
    (newId: string) => {
      setShowCreate(false);
      void fetchDeckPools();
      navigate(`/admin/deck-pools/${newId}`);
    },
    [fetchDeckPools, navigate],
  );

  const handleRefresh = useCallback(() => {
    if (id) {
      void fetchDeckPoolDetail(id);
    }
  }, [id, fetchDeckPoolDetail]);

  // ── Detail View ──────────────────────────────────────────────
  if (id && selectedDeckPool) {
    return (
      <div className="max-w-7xl mx-auto px-4 py-6">
        <DeckPoolDetailView
          detail={selectedDeckPool}
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
        <h1 className="text-2xl font-semibold text-white">Deck Pools</h1>
        <button
          onClick={() => setShowCreate(true)}
          className="px-4 py-2 rounded bg-blue-600 hover:bg-blue-500 text-sm text-white"
        >
          Create Pool
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

      {isLoading && deckPools.length === 0 ? (
        <div className="text-gray-400 text-sm">Loading deck pools...</div>
      ) : deckPools.length === 0 ? (
        <div className="text-gray-500 text-sm border border-gray-700 rounded p-8 text-center">
          No deck pools yet. Create one to get started.
        </div>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-xs text-gray-400 border-b border-gray-700">
                <th className="py-2 px-3">Name</th>
                <th className="py-2 px-3">Archetype</th>
                <th className="py-2 px-3">Variants</th>
                <th className="py-2 px-3">Mode</th>
                <th className="py-2 px-3">Created</th>
              </tr>
            </thead>
            <tbody>
              {deckPools.map(pool => (
                <tr
                  key={pool.id}
                  onClick={() => handleRowClick(pool.id)}
                  className="border-b border-gray-800 hover:bg-gray-800/50 cursor-pointer transition-colors"
                >
                  <td className="py-3 px-3 text-white font-medium">{pool.name}</td>
                  <td className="py-3 px-3 text-gray-300">{pool.archetype}</td>
                  <td className="py-3 px-3 text-gray-300">
                    {pool.variant_count} / {pool.target_variant_count}
                  </td>
                  <td className="py-3 px-3">
                    <ModeBadge mode={pool.generation_mode} />
                  </td>
                  <td className="py-3 px-3 text-gray-400 text-xs">
                    {new Date(pool.created_at).toLocaleDateString()}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {/* Create Modal */}
      {showCreate && (
        <CreatePoolModal
          onClose={() => setShowCreate(false)}
          onCreated={handleCreated}
        />
      )}
    </div>
  );
}
