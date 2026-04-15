import { useCallback, useEffect, useState } from 'react';
import {
  createKnownIssue,
  createRelease,
  deleteKnownIssue,
  deleteRelease,
  fetchPatchNotes,
  updateKnownIssue,
  updateRelease,
  type KnownIssue,
  type PatchNotesResponse,
  type Release,
  type ReleaseInput,
} from '@/api/patchNotesApi';

function linesToList(value: string): string[] {
  return value.split('\n').map((s) => s.trim()).filter(Boolean);
}

function listToLines(items: string[]): string {
  return items.join('\n');
}

function toDateInputValue(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '';
  return d.toISOString().slice(0, 10);
}

function fromDateInputValue(value: string): string {
  // Treat the date picker value as UTC midnight for consistent API input.
  return new Date(`${value}T00:00:00.000Z`).toISOString();
}

interface KnownIssueFormState {
  title: string;
  description: string;
}

const EMPTY_ISSUE: KnownIssueFormState = { title: '', description: '' };

function KnownIssueForm({
  initial,
  submitLabel,
  onSubmit,
  onCancel,
}: {
  initial?: KnownIssueFormState;
  submitLabel: string;
  onSubmit: (state: KnownIssueFormState) => Promise<void>;
  onCancel?: () => void;
}) {
  const [state, setState] = useState<KnownIssueFormState>(initial ?? EMPTY_ISSUE);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      await onSubmit(state);
      if (!initial) setState(EMPTY_ISSUE);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Request failed');
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-3 bg-gray-900 p-4 rounded border border-gray-700">
      <div>
        <label className="block text-xs font-semibold text-gray-400 uppercase mb-1">Title</label>
        <input
          type="text"
          value={state.title}
          onChange={(e) => setState({ ...state, title: e.target.value })}
          className="w-full rounded bg-gray-800 border border-gray-700 px-3 py-2 text-sm text-gray-100 focus:border-blue-500 outline-none"
          required
          maxLength={200}
        />
      </div>
      <div>
        <label className="block text-xs font-semibold text-gray-400 uppercase mb-1">Description</label>
        <textarea
          value={state.description}
          onChange={(e) => setState({ ...state, description: e.target.value })}
          className="w-full rounded bg-gray-800 border border-gray-700 px-3 py-2 text-sm text-gray-100 focus:border-blue-500 outline-none"
          rows={3}
          maxLength={4000}
        />
      </div>
      {error && <p className="text-sm text-red-400">{error}</p>}
      <div className="flex gap-2">
        <button
          type="submit"
          disabled={submitting}
          className="rounded bg-blue-500 hover:bg-blue-600 disabled:opacity-50 px-3 py-1.5 text-sm font-semibold text-white"
        >
          {submitting ? 'Saving…' : submitLabel}
        </button>
        {onCancel && (
          <button
            type="button"
            onClick={onCancel}
            className="rounded border border-gray-600 hover:border-gray-500 px-3 py-1.5 text-sm text-gray-300"
          >
            Cancel
          </button>
        )}
      </div>
    </form>
  );
}

interface ReleaseFormState {
  version: string;
  release_date: string;  // YYYY-MM-DD from date input
  title: string;
  added: string;         // newline-delimited
  changed: string;
  fixed: string;
}

const EMPTY_RELEASE: ReleaseFormState = {
  version: '',
  release_date: new Date().toISOString().slice(0, 10),
  title: '',
  added: '',
  changed: '',
  fixed: '',
};

function ReleaseForm({
  initial,
  submitLabel,
  onSubmit,
  onCancel,
}: {
  initial?: ReleaseFormState;
  submitLabel: string;
  onSubmit: (payload: ReleaseInput) => Promise<void>;
  onCancel?: () => void;
}) {
  const [state, setState] = useState<ReleaseFormState>(initial ?? EMPTY_RELEASE);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      await onSubmit({
        version: state.version.trim(),
        release_date: fromDateInputValue(state.release_date),
        title: state.title.trim() || null,
        added: linesToList(state.added),
        changed: linesToList(state.changed),
        fixed: linesToList(state.fixed),
      });
      if (!initial) setState(EMPTY_RELEASE);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Request failed');
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-3 bg-gray-900 p-4 rounded border border-gray-700">
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
        <div>
          <label className="block text-xs font-semibold text-gray-400 uppercase mb-1">Version</label>
          <input
            type="text"
            value={state.version}
            onChange={(e) => setState({ ...state, version: e.target.value })}
            className="w-full rounded bg-gray-800 border border-gray-700 px-3 py-2 text-sm text-gray-100 focus:border-blue-500 outline-none"
            placeholder="0.4.2"
            required
          />
        </div>
        <div>
          <label className="block text-xs font-semibold text-gray-400 uppercase mb-1">Release date</label>
          <input
            type="date"
            value={state.release_date}
            onChange={(e) => setState({ ...state, release_date: e.target.value })}
            className="w-full rounded bg-gray-800 border border-gray-700 px-3 py-2 text-sm text-gray-100 focus:border-blue-500 outline-none"
            required
          />
        </div>
        <div>
          <label className="block text-xs font-semibold text-gray-400 uppercase mb-1">Title (optional)</label>
          <input
            type="text"
            value={state.title}
            onChange={(e) => setState({ ...state, title: e.target.value })}
            className="w-full rounded bg-gray-800 border border-gray-700 px-3 py-2 text-sm text-gray-100 focus:border-blue-500 outline-none"
          />
        </div>
      </div>
      {(['added', 'changed', 'fixed'] as const).map((field) => (
        <div key={field}>
          <label className="block text-xs font-semibold text-gray-400 uppercase mb-1">
            {field} (one per line)
          </label>
          <textarea
            value={state[field]}
            onChange={(e) => setState({ ...state, [field]: e.target.value })}
            className="w-full rounded bg-gray-800 border border-gray-700 px-3 py-2 text-sm text-gray-100 focus:border-blue-500 outline-none"
            rows={3}
          />
        </div>
      ))}
      {error && <p className="text-sm text-red-400">{error}</p>}
      <div className="flex gap-2">
        <button
          type="submit"
          disabled={submitting}
          className="rounded bg-blue-500 hover:bg-blue-600 disabled:opacity-50 px-3 py-1.5 text-sm font-semibold text-white"
        >
          {submitting ? 'Saving…' : submitLabel}
        </button>
        {onCancel && (
          <button
            type="button"
            onClick={onCancel}
            className="rounded border border-gray-600 hover:border-gray-500 px-3 py-1.5 text-sm text-gray-300"
          >
            Cancel
          </button>
        )}
      </div>
    </form>
  );
}

function releaseToFormState(release: Release): ReleaseFormState {
  return {
    version: release.version,
    release_date: toDateInputValue(release.release_date),
    title: release.title ?? '',
    added: listToLines(release.added),
    changed: listToLines(release.changed),
    fixed: listToLines(release.fixed),
  };
}

export function AdminPatchNotesPage() {
  const [data, setData] = useState<PatchNotesResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const [showNewIssue, setShowNewIssue] = useState(false);
  const [editingIssueId, setEditingIssueId] = useState<string | null>(null);

  const [showNewRelease, setShowNewRelease] = useState(false);
  const [editingReleaseId, setEditingReleaseId] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setData(await fetchPatchNotes());
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Request failed');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const handleCreateIssue = async (state: { title: string; description: string }) => {
    await createKnownIssue(state);
    setShowNewIssue(false);
    await refresh();
  };

  const handleUpdateIssue = (id: string) => async (state: { title: string; description: string }) => {
    await updateKnownIssue(id, state);
    setEditingIssueId(null);
    await refresh();
  };

  const handleDeleteIssue = async (issue: KnownIssue) => {
    if (!window.confirm(`Delete known issue "${issue.title}"?`)) return;
    await deleteKnownIssue(issue.id);
    await refresh();
  };

  const handleCreateRelease = async (payload: ReleaseInput) => {
    await createRelease(payload);
    setShowNewRelease(false);
    await refresh();
  };

  const handleUpdateRelease = (id: string) => async (payload: ReleaseInput) => {
    await updateRelease(id, payload);
    setEditingReleaseId(null);
    await refresh();
  };

  const handleDeleteRelease = async (release: Release) => {
    if (!window.confirm(`Delete release v${release.version}?`)) return;
    await deleteRelease(release.id);
    await refresh();
  };

  return (
    <div className="max-w-4xl mx-auto px-4 py-8">
      <h1 className="text-3xl font-bold text-gray-100 mb-6">Patch Notes — Admin</h1>

      {loading && <p className="text-gray-400">Loading…</p>}
      {error && (
        <div className="rounded border border-red-500/40 bg-red-500/10 p-3 text-red-200 mb-4">
          {error}
        </div>
      )}

      {data && (
        <>
          <section className="mb-10">
            <div className="flex items-center justify-between mb-3">
              <h2 className="text-xl font-bold text-gray-100">Known Issues</h2>
              {!showNewIssue && (
                <button
                  type="button"
                  onClick={() => setShowNewIssue(true)}
                  className="rounded bg-blue-500 hover:bg-blue-600 px-3 py-1.5 text-sm font-semibold text-white"
                >
                  + Add Known Issue
                </button>
              )}
            </div>
            {showNewIssue && (
              <div className="mb-4">
                <KnownIssueForm
                  submitLabel="Create"
                  onSubmit={handleCreateIssue}
                  onCancel={() => setShowNewIssue(false)}
                />
              </div>
            )}
            {data.known_issues.length === 0 && !showNewIssue ? (
              <p className="text-gray-500 text-sm">No known issues.</p>
            ) : (
              <ul className="space-y-3">
                {data.known_issues.map((issue) => (
                  <li
                    key={issue.id}
                    className="bg-gray-800 border border-gray-700 rounded-lg p-4"
                  >
                    {editingIssueId === issue.id ? (
                      <KnownIssueForm
                        initial={{ title: issue.title, description: issue.description }}
                        submitLabel="Save"
                        onSubmit={handleUpdateIssue(issue.id)}
                        onCancel={() => setEditingIssueId(null)}
                      />
                    ) : (
                      <div>
                        <div className="flex items-start justify-between gap-3">
                          <div className="flex-1 min-w-0">
                            <p className="font-semibold text-gray-100">{issue.title}</p>
                            {issue.description && (
                              <p className="text-sm text-gray-400 mt-1 whitespace-pre-line">
                                {issue.description}
                              </p>
                            )}
                          </div>
                          <div className="flex gap-2 flex-shrink-0">
                            <button
                              type="button"
                              onClick={() => setEditingIssueId(issue.id)}
                              className="rounded border border-gray-600 hover:border-gray-500 px-2 py-1 text-xs text-gray-300"
                            >
                              Edit
                            </button>
                            <button
                              type="button"
                              onClick={() => { void handleDeleteIssue(issue); }}
                              className="rounded border border-red-500/60 hover:bg-red-500/10 px-2 py-1 text-xs text-red-300"
                            >
                              Delete
                            </button>
                          </div>
                        </div>
                      </div>
                    )}
                  </li>
                ))}
              </ul>
            )}
          </section>

          <section>
            <div className="flex items-center justify-between mb-3">
              <h2 className="text-xl font-bold text-gray-100">Releases</h2>
              {!showNewRelease && (
                <button
                  type="button"
                  onClick={() => setShowNewRelease(true)}
                  className="rounded bg-blue-500 hover:bg-blue-600 px-3 py-1.5 text-sm font-semibold text-white"
                >
                  + Add Release
                </button>
              )}
            </div>
            {showNewRelease && (
              <div className="mb-4">
                <ReleaseForm
                  submitLabel="Create"
                  onSubmit={handleCreateRelease}
                  onCancel={() => setShowNewRelease(false)}
                />
              </div>
            )}
            {data.releases.length === 0 && !showNewRelease ? (
              <p className="text-gray-500 text-sm">No releases yet.</p>
            ) : (
              <ul className="space-y-4">
                {data.releases.map((release) => (
                  <li
                    key={release.id}
                    className="bg-gray-800 border border-gray-700 rounded-lg p-4"
                  >
                    {editingReleaseId === release.id ? (
                      <ReleaseForm
                        initial={releaseToFormState(release)}
                        submitLabel="Save"
                        onSubmit={handleUpdateRelease(release.id)}
                        onCancel={() => setEditingReleaseId(null)}
                      />
                    ) : (
                      <div>
                        <div className="flex items-start justify-between gap-3">
                          <div className="flex-1 min-w-0">
                            <div className="flex flex-wrap items-baseline gap-x-3">
                              <h3 className="font-bold text-gray-100">v{release.version}</h3>
                              <span className="text-sm text-gray-400">
                                {new Date(release.release_date).toLocaleDateString()}
                              </span>
                              {release.title && (
                                <span className="text-sm text-blue-400">— {release.title}</span>
                              )}
                            </div>
                            <div className="mt-2 grid grid-cols-1 sm:grid-cols-3 gap-3 text-xs text-gray-400">
                              <div>
                                <div className="font-semibold uppercase">Added</div>
                                <div>{release.added.length} items</div>
                              </div>
                              <div>
                                <div className="font-semibold uppercase">Changed</div>
                                <div>{release.changed.length} items</div>
                              </div>
                              <div>
                                <div className="font-semibold uppercase">Fixed</div>
                                <div>{release.fixed.length} items</div>
                              </div>
                            </div>
                          </div>
                          <div className="flex gap-2 flex-shrink-0">
                            <button
                              type="button"
                              onClick={() => setEditingReleaseId(release.id)}
                              className="rounded border border-gray-600 hover:border-gray-500 px-2 py-1 text-xs text-gray-300"
                            >
                              Edit
                            </button>
                            <button
                              type="button"
                              onClick={() => { void handleDeleteRelease(release); }}
                              className="rounded border border-red-500/60 hover:bg-red-500/10 px-2 py-1 text-xs text-red-300"
                            >
                              Delete
                            </button>
                          </div>
                        </div>
                      </div>
                    )}
                  </li>
                ))}
              </ul>
            )}
          </section>
        </>
      )}
    </div>
  );
}
