import { useCallback, useEffect, useState } from 'react';
import {
  deleteModel,
  listModels,
  updateModel,
  type AIModel,
  type ListModelsFilters,
  type ModelState,
  type ModelType,
} from '@/api/modelsApi';
import { AdminModelsEditModal } from './AdminModelsEditModal';
import { AdminModelsUploadModal } from './AdminModelsUploadModal';

function formatBytes(bytes: number | null): string {
  if (bytes === null) return '—';
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
}

function formatDate(iso: string | null): string {
  if (!iso) return '—';
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '—';
  return d.toLocaleDateString();
}

function StateChip({ state }: { state: ModelState }) {
  const colours: Record<ModelState, string> = {
    uploaded: 'bg-green-500/20 text-green-300 border-green-500/40',
    pending: 'bg-yellow-500/20 text-yellow-300 border-yellow-500/40',
    failed: 'bg-red-500/20 text-red-300 border-red-500/40',
  };
  return (
    <span className={`inline-block rounded border px-1.5 py-0.5 text-xs font-semibold ${colours[state]}`}>
      {state}
    </span>
  );
}

function TypeChip({ type }: { type: ModelType }) {
  const colours: Record<ModelType, string> = {
    mlp: 'bg-blue-500/20 text-blue-300 border-blue-500/40',
    lstm: 'bg-purple-500/20 text-purple-300 border-purple-500/40',
  };
  return (
    <span className={`inline-block rounded border px-1.5 py-0.5 text-xs font-semibold uppercase ${colours[type]}`}>
      {type}
    </span>
  );
}

function DeckChip({ name }: { name: string | null }) {
  if (!name) return <span className="text-gray-600 text-xs">—</span>;
  return (
    <span className="inline-block rounded border border-gray-600 bg-gray-700/40 px-1.5 py-0.5 text-xs text-gray-300 max-w-[120px] truncate" title={name}>
      {name}
    </span>
  );
}

export function AdminModelsPage() {
  const [models, setModels] = useState<AIModel[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [filterState, setFilterState] = useState<'' | ModelState>('');
  const [filterPublished, setFilterPublished] = useState<'' | 'true' | 'false'>('');
  const [filterType, setFilterType] = useState<'' | ModelType>('');

  const [showUpload, setShowUpload] = useState(false);
  const [editModel, setEditModel] = useState<AIModel | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const filters: ListModelsFilters = {};
      if (filterState) filters.state = filterState;
      if (filterPublished !== '') filters.published = filterPublished === 'true';
      if (filterType) filters.model_type = filterType;
      setModels(await listModels(filters));
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load models');
    } finally {
      setLoading(false);
    }
  }, [filterState, filterPublished, filterType]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const handleTogglePublished = async (model: AIModel) => {
    setError(null);
    try {
      await updateModel(model.id, { published: !model.published });
      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to update published state');
    }
  };

  const handleDelete = async (model: AIModel) => {
    if (!window.confirm('Delete this model? This removes the Spaces object too.')) return;
    setError(null);
    try {
      await deleteModel(model.id);
      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to delete model');
    }
  };

  return (
    <div className="max-w-7xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-3xl font-bold text-gray-100">AI Models</h1>
        <button
          type="button"
          onClick={() => setShowUpload(true)}
          className="rounded bg-blue-500 hover:bg-blue-600 px-3 py-1.5 text-sm font-semibold text-white"
        >
          + Upload Model
        </button>
      </div>

      {/* Filters */}
      <div className="flex flex-wrap gap-3 mb-5">
        <div>
          <label className="block text-xs text-gray-500 mb-1">State</label>
          <select
            value={filterState}
            onChange={(e) => setFilterState(e.target.value as '' | ModelState)}
            className="rounded bg-gray-800 border border-gray-700 px-2 py-1.5 text-sm text-gray-100 focus:border-blue-500 outline-none"
          >
            <option value="">All states</option>
            <option value="uploaded">uploaded</option>
            <option value="pending">pending</option>
            <option value="failed">failed</option>
          </select>
        </div>

        <div>
          <label className="block text-xs text-gray-500 mb-1">Published</label>
          <select
            value={filterPublished}
            onChange={(e) => setFilterPublished(e.target.value as '' | 'true' | 'false')}
            className="rounded bg-gray-800 border border-gray-700 px-2 py-1.5 text-sm text-gray-100 focus:border-blue-500 outline-none"
          >
            <option value="">All</option>
            <option value="true">Published</option>
            <option value="false">Unpublished</option>
          </select>
        </div>

        <div>
          <label className="block text-xs text-gray-500 mb-1">Type</label>
          <select
            value={filterType}
            onChange={(e) => setFilterType(e.target.value as '' | ModelType)}
            className="rounded bg-gray-800 border border-gray-700 px-2 py-1.5 text-sm text-gray-100 focus:border-blue-500 outline-none"
          >
            <option value="">All types</option>
            <option value="mlp">MLP</option>
            <option value="lstm">LSTM</option>
          </select>
        </div>
      </div>

      {/* Error */}
      {error && (
        <div className="rounded border border-red-500/40 bg-red-500/10 p-3 text-red-200 mb-4 text-sm">
          {error}
        </div>
      )}

      {/* Loading */}
      {loading && <p className="text-gray-400">Loading…</p>}

      {/* Table */}
      {!loading && (
        <>
          {models.length === 0 ? (
            <p className="text-gray-500 text-sm">No models found.</p>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full text-sm text-gray-300 border-collapse">
                <thead>
                  <tr className="border-b border-gray-700 text-xs uppercase text-gray-500">
                    <th className="text-left py-2 pr-4 font-semibold">Name</th>
                    <th className="text-left py-2 pr-4 font-semibold">Type</th>
                    <th className="text-left py-2 pr-4 font-semibold">Deck</th>
                    <th className="text-left py-2 pr-4 font-semibold">State</th>
                    <th className="text-left py-2 pr-4 font-semibold">Published</th>
                    <th className="text-left py-2 pr-4 font-semibold">SHA-256</th>
                    <th className="text-left py-2 pr-4 font-semibold">Size</th>
                    <th className="text-left py-2 pr-4 font-semibold">Uploader</th>
                    <th className="text-left py-2 pr-4 font-semibold">Trained</th>
                    <th className="text-left py-2 font-semibold">Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {models.map((m) => (
                    <tr
                      key={m.id}
                      className="border-b border-gray-800 hover:bg-gray-800/40 transition-colors"
                    >
                      <td className="py-2 pr-4">
                        <span className="font-medium text-gray-100 max-w-[160px] block truncate" title={m.name}>
                          {m.name}
                        </span>
                      </td>
                      <td className="py-2 pr-4">
                        <TypeChip type={m.model_type} />
                      </td>
                      <td className="py-2 pr-4">
                        <DeckChip name={m.deck_name} />
                      </td>
                      <td className="py-2 pr-4">
                        <StateChip state={m.state} />
                      </td>
                      <td className="py-2 pr-4">
                        <button
                          type="button"
                          disabled={m.state !== 'uploaded'}
                          onClick={() => { void handleTogglePublished(m); }}
                          title={
                            m.state !== 'uploaded'
                              ? "Model must be in 'uploaded' state to publish"
                              : m.published
                              ? 'Click to unpublish'
                              : 'Click to publish'
                          }
                          className={`relative inline-flex h-5 w-9 items-center rounded-full transition-colors focus:outline-none disabled:opacity-40 disabled:cursor-not-allowed ${
                            m.published ? 'bg-blue-500' : 'bg-gray-600'
                          }`}
                        >
                          <span
                            className={`inline-block h-3.5 w-3.5 rounded-full bg-white shadow transform transition-transform ${
                              m.published ? 'translate-x-4' : 'translate-x-0.5'
                            }`}
                          />
                        </button>
                      </td>
                      <td className="py-2 pr-4 font-mono text-xs text-gray-400">
                        {m.file_sha256 ? m.file_sha256.slice(0, 12) : '—'}
                      </td>
                      <td className="py-2 pr-4 whitespace-nowrap">
                        {formatBytes(m.file_size_bytes)}
                      </td>
                      <td className="py-2 pr-4 text-xs text-gray-400">
                        {m.uploader_username ?? '—'}
                      </td>
                      <td className="py-2 pr-4 whitespace-nowrap text-xs text-gray-400">
                        {formatDate(m.trained_at)}
                      </td>
                      <td className="py-2">
                        <div className="flex gap-2">
                          <button
                            type="button"
                            onClick={() => setEditModel(m)}
                            className="rounded border border-gray-600 hover:border-gray-500 px-2 py-1 text-xs text-gray-300"
                          >
                            Edit
                          </button>
                          <button
                            type="button"
                            onClick={() => { void handleDelete(m); }}
                            className="rounded border border-red-500/60 hover:bg-red-500/10 px-2 py-1 text-xs text-red-300"
                          >
                            Delete
                          </button>
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </>
      )}

      {/* Modals */}
      <AdminModelsUploadModal
        open={showUpload}
        onClose={() => setShowUpload(false)}
        onCreated={() => { void refresh(); }}
      />
      <AdminModelsEditModal
        open={editModel !== null}
        model={editModel}
        onClose={() => setEditModel(null)}
        onSaved={() => { void refresh(); }}
      />
    </div>
  );
}
