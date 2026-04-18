import { useCallback, useEffect, useMemo, useState } from 'react';
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

const DEFAULT_MANIFEST_URL =
  (import.meta.env.VITE_MODELS_MANIFEST_URL as string | undefined) ?? '';

const MANIFEST_URL_STORAGE_KEY = 'digimon-tcg:models:manifest-url';

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

function formatDate(epochSec: number): string {
  return new Date(epochSec * 1000).toLocaleString();
}

function shortSha(sha: string): string {
  return sha.slice(0, 10);
}

export function ModelsPage() {
  const [baseUrl, setBaseUrl] = useState<string>(
    () => localStorage.getItem(MANIFEST_URL_STORAGE_KEY) ?? DEFAULT_MANIFEST_URL,
  );
  const [contract, setContract] = useState<EngineContract | null>(null);
  const [manifest, setManifest] = useState<ManifestModel[]>([]);
  const [local, setLocal] = useState<LocalModelMeta[]>([]);
  const [loadingManifest, setLoadingManifest] = useState(false);
  const [busyIds, setBusyIds] = useState<Set<string>>(new Set());
  const [error, setError] = useState<string | null>(null);

  const localIds = useMemo(() => new Set(local.map((m) => m.id)), [local]);

  const refreshLocal = useCallback(async () => {
    try {
      setLocal(await listLocal());
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  const refreshContract = useCallback(async () => {
    try {
      setContract(await engineContract());
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  useEffect(() => {
    void refreshContract();
    void refreshLocal();
  }, [refreshContract, refreshLocal]);

  const handleFetch = useCallback(async () => {
    if (!baseUrl) {
      setError('Enter a manifest URL first.');
      return;
    }
    setLoadingManifest(true);
    setError(null);
    try {
      localStorage.setItem(MANIFEST_URL_STORAGE_KEY, baseUrl);
      setManifest(await fetchManifest(baseUrl));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoadingManifest(false);
    }
  }, [baseUrl]);

  const withBusy = useCallback(
    async (id: string, fn: () => Promise<void>) => {
      setBusyIds((prev) => new Set(prev).add(id));
      try {
        await fn();
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

  const handleDownload = useCallback(
    (model: ManifestModel) =>
      withBusy(model.id, async () => {
        setError(null);
        try {
          await downloadModel(model);
          await refreshLocal();
        } catch (err) {
          setError(err instanceof Error ? err.message : String(err));
        }
      }),
    [refreshLocal, withBusy],
  );

  const handleLoad = useCallback(
    (modelId: string) =>
      withBusy(modelId, async () => {
        setError(null);
        try {
          await loadCached(modelId);
        } catch (err) {
          setError(err instanceof Error ? err.message : String(err));
        }
      }),
    [withBusy],
  );

  const handleDelete = useCallback(
    (modelId: string) =>
      withBusy(modelId, async () => {
        if (!window.confirm(`Delete cached model ${modelId}?`)) return;
        setError(null);
        try {
          await deleteModelApi(modelId);
          await refreshLocal();
        } catch (err) {
          setError(err instanceof Error ? err.message : String(err));
        }
      }),
    [refreshLocal, withBusy],
  );

  return (
    <div className="p-6 text-gray-100">
      <h1 className="text-2xl font-bold mb-2">AI Models</h1>
      <p className="text-sm text-gray-400 mb-4">
        Download trained ONNX policies to play against. Only models whose tensor and
        action-space shapes match this build of the engine can be used.
      </p>

      {contract && (
        <div className="mb-6 text-xs text-gray-400">
          Engine contract: tensor={contract.tensor_size}, actions={contract.action_space_size}
          {contract.engine_commit ? `, commit=${contract.engine_commit.slice(0, 10)}` : ''}
        </div>
      )}

      {error && (
        <div className="rounded border border-red-500/40 bg-red-500/10 p-3 text-red-200 mb-4 text-sm">
          {error}
        </div>
      )}

      <section className="mb-8">
        <h2 className="text-lg font-semibold mb-2">Downloaded</h2>
        {local.length === 0 ? (
          <p className="text-sm text-gray-500">No models downloaded yet.</p>
        ) : (
          <table className="w-full text-sm">
            <thead className="text-xs uppercase text-gray-400">
              <tr>
                <th className="text-left py-2 pr-4">Name</th>
                <th className="text-left py-2 pr-4">Type</th>
                <th className="text-left py-2 pr-4">Downloaded</th>
                <th className="text-left py-2 pr-4">Size</th>
                <th className="text-left py-2 pr-4">SHA256</th>
                <th className="text-right py-2">Actions</th>
              </tr>
            </thead>
            <tbody>
              {local.map((m) => {
                const busy = busyIds.has(m.id);
                return (
                  <tr key={m.id} className="border-t border-gray-700">
                    <td className="py-2 pr-4">{m.name}</td>
                    <td className="py-2 pr-4 uppercase">{m.model_type}</td>
                    <td className="py-2 pr-4 text-gray-400">{formatDate(m.downloaded_at)}</td>
                    <td className="py-2 pr-4">{formatBytes(m.file_size_bytes)}</td>
                    <td className="py-2 pr-4 text-gray-500 font-mono">{shortSha(m.file_sha256)}</td>
                    <td className="py-2 text-right space-x-2">
                      <button
                        disabled={busy}
                        onClick={() => handleLoad(m.id)}
                        className="px-2 py-1 bg-blue-600 hover:bg-blue-500 disabled:opacity-50 rounded text-xs"
                      >
                        Activate
                      </button>
                      <button
                        disabled={busy}
                        onClick={() => handleDelete(m.id)}
                        className="px-2 py-1 bg-red-600 hover:bg-red-500 disabled:opacity-50 rounded text-xs"
                      >
                        Delete
                      </button>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </section>

      <section>
        <h2 className="text-lg font-semibold mb-2">Remote manifest</h2>
        <div className="flex flex-wrap gap-2 items-end mb-4">
          <div className="flex-1 min-w-[320px]">
            <label className="block text-xs text-gray-400 mb-1">
              Manifest base URL (e.g. <code>https://api.example.com</code>)
            </label>
            <input
              type="url"
              value={baseUrl}
              onChange={(e) => setBaseUrl(e.target.value)}
              placeholder="https://…"
              className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-gray-200"
            />
          </div>
          <button
            onClick={handleFetch}
            disabled={loadingManifest || !baseUrl}
            className="px-4 py-2 bg-blue-600 hover:bg-blue-500 disabled:opacity-50 rounded"
          >
            {loadingManifest ? 'Fetching…' : 'Fetch manifest'}
          </button>
        </div>

        {manifest.length === 0 ? (
          <p className="text-sm text-gray-500">
            {loadingManifest
              ? 'Loading manifest…'
              : 'No remote models listed yet — set a URL and click "Fetch manifest."'}
          </p>
        ) : (
          <table className="w-full text-sm">
            <thead className="text-xs uppercase text-gray-400">
              <tr>
                <th className="text-left py-2 pr-4">Name</th>
                <th className="text-left py-2 pr-4">Type</th>
                <th className="text-left py-2 pr-4">Shape</th>
                <th className="text-left py-2 pr-4">Size</th>
                <th className="text-right py-2">Actions</th>
              </tr>
            </thead>
            <tbody>
              {manifest.map((m) => {
                const compatible = !contract || isCompatible(m, contract);
                const have = localIds.has(m.id);
                const busy = busyIds.has(m.id);
                return (
                  <tr key={m.id} className="border-t border-gray-700">
                    <td className="py-2 pr-4">
                      {m.name}
                      {m.notes ? (
                        <div className="text-xs text-gray-500">{m.notes}</div>
                      ) : null}
                    </td>
                    <td className="py-2 pr-4 uppercase">{m.model_type}</td>
                    <td className="py-2 pr-4 text-gray-400">
                      {m.tensor_size}→{m.action_space_size}
                      {!compatible && (
                        <span className="ml-2 text-red-400" title="Shape mismatch">
                          incompatible
                        </span>
                      )}
                    </td>
                    <td className="py-2 pr-4">{formatBytes(m.file_size_bytes)}</td>
                    <td className="py-2 text-right">
                      {have ? (
                        <span className="text-xs text-green-400">downloaded</span>
                      ) : (
                        <button
                          disabled={busy || !compatible}
                          onClick={() => handleDownload(m)}
                          title={!compatible ? 'Shape mismatch' : undefined}
                          className="px-2 py-1 bg-blue-600 hover:bg-blue-500 disabled:opacity-40 rounded text-xs"
                        >
                          {busy ? 'Downloading…' : 'Download'}
                        </button>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </section>
    </div>
  );
}
