import { useEffect, useState } from 'react';
import { fetchPatchNotes, type PatchNotesResponse, type Release } from '@/api/patchNotesApi';

function formatDate(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
}

function BulletSection({ label, items }: { label: string; items: string[] }) {
  if (!items.length) return null;
  return (
    <div className="mt-3">
      <h4 className="text-sm font-semibold text-gray-300 uppercase tracking-wide">{label}</h4>
      <ul className="mt-1 list-disc list-inside text-sm text-gray-200 space-y-1">
        {items.map((item, idx) => (
          <li key={idx} className="whitespace-pre-line">{item}</li>
        ))}
      </ul>
    </div>
  );
}

function ReleaseCard({ release }: { release: Release }) {
  return (
    <article className="bg-gray-800 border border-gray-700 rounded-lg p-5">
      <header className="flex flex-wrap items-baseline gap-x-3 gap-y-1">
        <h3 className="text-lg font-bold text-gray-100">v{release.version}</h3>
        <span className="text-sm text-gray-400">{formatDate(release.release_date)}</span>
        {release.title ? (
          <span className="text-sm text-blue-400">— {release.title}</span>
        ) : null}
      </header>
      <BulletSection label="Added" items={release.added} />
      <BulletSection label="Changed" items={release.changed} />
      <BulletSection label="Fixed" items={release.fixed} />
    </article>
  );
}

export function PatchNotesPage() {
  const [data, setData] = useState<PatchNotesResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    fetchPatchNotes()
      .then((res) => {
        if (!cancelled) setData(res);
      })
      .catch((err) => {
        if (!cancelled) {
          const message = err instanceof Error ? err.message : 'Unknown error';
          setError(message);
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <div className="max-w-3xl mx-auto px-4 py-8">
      <h1 className="text-3xl font-bold text-gray-100 mb-6">Patch Notes</h1>

      {loading && <p className="text-gray-400">Loading patch notes…</p>}

      {error && (
        <div className="rounded-lg border border-red-500/40 bg-red-500/10 p-4 text-red-200">
          <p className="font-semibold">Could not load patch notes.</p>
          <p className="text-sm mt-1">{error}</p>
        </div>
      )}

      {!loading && !error && data && (
        <>
          {data.known_issues.length > 0 && (
            <section className="mb-8 rounded-lg border border-amber-500/40 bg-amber-500/10 p-5 text-amber-100">
              <h2 className="text-lg font-bold mb-3">Known Issues</h2>
              <ul className="space-y-3">
                {data.known_issues.map((issue) => (
                  <li key={issue.id}>
                    <p className="font-semibold">{issue.title}</p>
                    {issue.description && (
                      <p className="text-sm mt-1 text-amber-100/80 whitespace-pre-line">
                        {issue.description}
                      </p>
                    )}
                  </li>
                ))}
              </ul>
            </section>
          )}

          {data.releases.length > 0 ? (
            <div className="space-y-5">
              {data.releases.map((release) => (
                <ReleaseCard key={release.id} release={release} />
              ))}
            </div>
          ) : (
            data.known_issues.length === 0 && (
              <p className="text-gray-400">No patch notes yet. Check back soon.</p>
            )
          )}
        </>
      )}
    </div>
  );
}
