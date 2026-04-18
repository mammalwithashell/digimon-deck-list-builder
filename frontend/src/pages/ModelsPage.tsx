import { useCallback, useEffect, useMemo, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import {
  deleteModel as deleteModelApi,
  downloadModel,
  engineContract,
  fetchManifest,
  isCompatible,
  listLocal,
  loadCached,
  type EngineContract,
  type LocalModelMeta,
  type ManifestModel,
} from '@/api/desktopModelsApi';
import * as deckStore from '@/storage/deckStore';
import { createVsAiGame } from '@/api/gameApi';

const MANIFEST_URL =
  (import.meta.env.VITE_MODELS_MANIFEST_URL as string | undefined) ?? '';

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

interface MergedRow {
  id: string;
  manifest?: ManifestModel;
  local?: LocalModelMeta;
}

export function ModelsPage() {
  const navigate = useNavigate();
  const [contract, setContract] = useState<EngineContract | null>(null);
  const [manifest, setManifest] = useState<ManifestModel[]>([]);
  const [local, setLocal] = useState<LocalModelMeta[]>([]);
  const [busyIds, setBusyIds] = useState<Set<string>>(new Set());
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [decks, setDecks] = useState<deckStore.DeckSummary[]>([]);
  const [playDeckId, setPlayDeckId] = useState<string>('');

  const refresh = useCallback(async () => {
    setError(null);
    try {
      const [c, m, l, d] = await Promise.all([
        engineContract(),
        MANIFEST_URL ? fetchManifest(MANIFEST_URL) : Promise.resolve([] as ManifestModel[]),
        listLocal(),
        deckStore.listDecks(),
      ]);
      setContract(c);
      setManifest(m);
      setLocal(l);
      setDecks(d);
      if (d.length > 0 && !playDeckId && d[0]) setPlayDeckId(d[0].id);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [playDeckId]);

  useEffect(() => {
    void refresh();
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const rows = useMemo<MergedRow[]>(() => {
    const byId = new Map<string, MergedRow>();
    for (const m of manifest) byId.set(m.id, { id: m.id, manifest: m });
    for (const l of local) {
      const row = byId.get(l.id) ?? { id: l.id };
      row.local = l;
      byId.set(l.id, row);
    }
    return Array.from(byId.values());
  }, [manifest, local]);

  const withBusy = useCallback(
    async (id: string, fn: () => Promise<void>) => {
      setBusyIds((prev) => new Set(prev).add(id));
      try {
        await fn();
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setBusyIds((prev) => {
          const next = new Set(prev);
          next.delete(id);
          return next;
        });
      }
    },
    [],
  );

  const handleDownload = (m: ManifestModel) =>
    withBusy(m.id, async () => {
      await downloadModel(m);
      await refresh();
    });

  const handleDelete = (id: string) =>
    withBusy(id, async () => {
      if (!window.confirm(`Delete cached model ${id}?`)) return;
      await deleteModelApi(id);
      await refresh();
    });

  const handleActivate = (id: string) =>
    withBusy(id, async () => {
      await loadCached(id);
    });

  const handleTryOnline = async (row: MergedRow) => {
    if (!playDeckId) {
      setError('Select a deck to play with first.');
      return;
    }
    await withBusy(row.id, async () => {
      const deck = await deckStore.getDeck(playDeckId);
      // TODO(alpha+1): use `row.manifest?.deck_id` (see ManifestModel schema) to
      // pick the AI's intended deck. For alpha we mirror the user's deck so a
      // balanced matchup is at least possible.
      const { game_id } = await createVsAiGame({
        modelId: row.id,
        userDeck: { main_deck: deck.main_deck, egg_deck: deck.egg_deck },
        opponentDeck: { main_deck: deck.main_deck, egg_deck: deck.egg_deck },
      });
      navigate(`/game/${game_id}?mode=vsai&player=1`);
    });
  };

  return (
    <div className="p-6 text-gray-100">
      <h1 className="mb-2 text-2xl font-bold">AI Models</h1>
      <p className="mb-4 text-sm text-gray-400">
        Try models online, or download them for offline play. Models whose
        tensor/action shapes don&apos;t match this build are greyed out.
      </p>

      {contract && (
        <div className="mb-6 text-xs text-gray-400">
          Engine contract: tensor={contract.tensor_size}, actions=
          {contract.action_space_size}
          {contract.engine_commit
            ? `, commit=${contract.engine_commit.slice(0, 10)}`
            : ''}
        </div>
      )}

      {error && (
        <div className="mb-4 rounded border border-red-500/40 bg-red-500/10 p-3 text-sm text-red-200">
          {error}
        </div>
      )}

      <div className="mb-6 flex items-end gap-3">
        <div>
          <label className="mb-1 block text-xs text-gray-400">Your deck</label>
          <select
            value={playDeckId}
            onChange={(e) => setPlayDeckId(e.target.value)}
            disabled={decks.length === 0}
            className="rounded border border-gray-600 bg-gray-700 px-3 py-2 text-sm text-white disabled:opacity-50"
          >
            {decks.length === 0 ? (
              <option value="">No decks yet</option>
            ) : (
              decks.map((d) => (
                <option key={d.id} value={d.id}>
                  {d.name}
                </option>
              ))
            )}
          </select>
          {decks.length === 0 && (
            <Link to="/deckbuilder" className="mt-1 block text-xs text-blue-400 hover:text-blue-300">
              Build your first deck →
            </Link>
          )}
        </div>
        <button
          onClick={() => void refresh()}
          className="rounded bg-gray-700 px-3 py-2 text-sm hover:bg-gray-600"
        >
          Refresh
        </button>
      </div>

      {loading ? (
        <p className="text-sm text-gray-400">Loading…</p>
      ) : rows.length === 0 ? (
        <p className="text-sm text-gray-500">
          No models available. Check that <code>VITE_MODELS_MANIFEST_URL</code> is set
          and the server is reachable.
        </p>
      ) : (
        <table className="w-full text-sm">
          <thead className="text-xs uppercase text-gray-400">
            <tr>
              <th className="py-2 pr-4 text-left">Name</th>
              <th className="py-2 pr-4 text-left">Type</th>
              <th className="py-2 pr-4 text-left">Size</th>
              <th className="py-2 pr-4 text-left">Status</th>
              <th className="py-2 text-right">Actions</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => {
              const busy = busyIds.has(row.id);
              const compatible =
                !contract ||
                !row.manifest ||
                isCompatible(row.manifest, contract);
              const have = !!row.local;
              const name = row.manifest?.name ?? row.local?.name ?? row.id;
              const type = (row.manifest?.model_type ?? row.local?.model_type ?? '').toUpperCase();
              const size = row.manifest?.file_size_bytes ?? row.local?.file_size_bytes ?? 0;
              return (
                <tr key={row.id} className="border-t border-gray-700">
                  <td className="py-2 pr-4">{name}</td>
                  <td className="py-2 pr-4">{type}</td>
                  <td className="py-2 pr-4 text-gray-400">{formatBytes(size)}</td>
                  <td className="py-2 pr-4">
                    {have ? (
                      <span className="text-xs text-green-400">downloaded</span>
                    ) : compatible ? (
                      <span className="text-xs text-gray-400">online only</span>
                    ) : (
                      <span className="text-xs text-red-400">incompatible</span>
                    )}
                  </td>
                  <td className="py-2 text-right">
                    <div className="flex justify-end gap-2">
                      {row.manifest && compatible && (
                        <button
                          disabled={busy || !playDeckId}
                          onClick={() => void handleTryOnline(row)}
                          className="rounded bg-blue-600 px-2 py-1 text-xs hover:bg-blue-500 disabled:opacity-40"
                        >
                          {busy ? 'Preparing…' : 'Try online'}
                        </button>
                      )}
                      {row.manifest && !have && compatible && (
                        <button
                          disabled={busy}
                          onClick={() => void handleDownload(row.manifest!)}
                          className="rounded bg-gray-700 px-2 py-1 text-xs hover:bg-gray-600 disabled:opacity-40"
                        >
                          Download
                        </button>
                      )}
                      {have && (
                        <>
                          <button
                            disabled={busy}
                            onClick={() => void handleActivate(row.id)}
                            className="rounded bg-green-700 px-2 py-1 text-xs hover:bg-green-600 disabled:opacity-40"
                          >
                            Activate
                          </button>
                          <button
                            disabled={busy}
                            onClick={() => void handleDelete(row.id)}
                            className="rounded bg-red-700 px-2 py-1 text-xs hover:bg-red-600 disabled:opacity-40"
                          >
                            Delete
                          </button>
                        </>
                      )}
                    </div>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </div>
  );
}
