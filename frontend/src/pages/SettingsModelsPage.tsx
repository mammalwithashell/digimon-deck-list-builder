import { useEffect } from 'react';
import { useModelStore } from '@/stores/modelStore';
import type { ModelEntry, ModelVersionEntry } from '@/api/modelsApi';

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

function VersionRow({
  entry,
  version,
}: {
  entry: ModelEntry;
  version: ModelVersionEntry;
}) {
  const { key, inFlight, download, remove } = useModelStore();
  const flight = inFlight[key(entry.slug, version.version)];
  const cached = version.is_cached;

  return (
    <tr style={{ borderBottom: '1px solid #2a2a2a' }}>
      <td style={{ padding: '6px 8px', fontFamily: 'monospace' }}>{version.version}</td>
      <td style={{ padding: '6px 8px' }}>{formatSize(version.size_bytes)}</td>
      <td style={{ padding: '6px 8px' }}>
        {version.is_deprecated ? (
          <span style={{ color: '#b07' }}>deprecated</span>
        ) : cached ? (
          <span style={{ color: '#2a2' }}>cached</span>
        ) : (
          <span style={{ color: '#888' }}>not downloaded</span>
        )}
      </td>
      <td style={{ padding: '6px 8px', textAlign: 'right' }}>
        {!cached && (
          <button
            disabled={flight === 'downloading'}
            onClick={() => download(entry.slug, version.version)}
          >
            {flight === 'downloading' ? 'Downloading…' : 'Download'}
          </button>
        )}
        {cached && (
          <button
            disabled={flight === 'deleting'}
            onClick={() => remove(entry.slug, version.version)}
          >
            {flight === 'deleting' ? 'Deleting…' : 'Delete'}
          </button>
        )}
      </td>
    </tr>
  );
}

function ModelCard({ entry }: { entry: ModelEntry }) {
  return (
    <section
      style={{
        border: '1px solid #2a2a2a',
        borderRadius: 8,
        padding: 16,
        marginBottom: 16,
        opacity: entry.is_deprecated ? 0.6 : 1,
      }}
    >
      <header style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 8 }}>
        <div>
          <strong>{entry.name}</strong>
          <span style={{ marginLeft: 8, fontFamily: 'monospace', color: '#888' }}>
            {entry.slug}
          </span>
        </div>
        <div style={{ color: '#888' }}>
          arch: <code>{entry.arch}</code> &middot; requires engine ≥{' '}
          <code>{entry.min_engine_version}</code>
        </div>
      </header>
      {entry.description && (
        <p style={{ marginTop: 0, color: '#bbb' }}>{entry.description}</p>
      )}
      {entry.bundled_filename ? (
        <div style={{ color: '#2a2' }}>
          Bundled with this installer: <code>{entry.bundled_filename}</code>
        </div>
      ) : entry.versions.length === 0 ? (
        <div style={{ color: '#888' }}>No versions available.</div>
      ) : (
        <table style={{ width: '100%', borderCollapse: 'collapse' }}>
          <thead>
            <tr style={{ textAlign: 'left', color: '#888' }}>
              <th style={{ padding: '6px 8px' }}>Version</th>
              <th style={{ padding: '6px 8px' }}>Size</th>
              <th style={{ padding: '6px 8px' }}>Status</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {entry.versions.map((v) => (
              <VersionRow key={v.version} entry={entry} version={v} />
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}

export default function SettingsModelsPage() {
  const { manifest, loading, error, refresh, listModels } = useModelStore();

  useEffect(() => {
    refresh();
  }, [refresh]);

  const models = listModels();
  const totalCachedBytes = models
    .flatMap((m) => m.versions)
    .filter((v) => v.is_cached)
    .reduce((acc, v) => acc + v.size_bytes, 0);

  return (
    <div style={{ maxWidth: 900, margin: '0 auto', padding: 24 }}>
      <header style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <h1>AI Models</h1>
        <button onClick={() => refresh(true)} disabled={loading}>
          {loading ? 'Refreshing…' : 'Refresh'}
        </button>
      </header>

      <p style={{ color: '#888' }}>
        Bundled models ship with the app. Downloaded models are cached for offline play; delete
        anything you don't want to keep around.
      </p>
      <p style={{ color: '#888' }}>
        Cache footprint: <strong>{formatSize(totalCachedBytes)}</strong>
        {manifest?.cache_dir && (
          <>
            {' '}
            in <code>{manifest.cache_dir}</code>
          </>
        )}
      </p>

      {error && (
        <div style={{ color: '#b44', marginBottom: 12 }}>Error: {error}</div>
      )}

      {models.length === 0 && !loading && (
        <p>No models available. Connect to the hosted API or install a build with a bundled baseline.</p>
      )}

      {models.map((m) => (
        <ModelCard key={m.slug} entry={m} />
      ))}
    </div>
  );
}
