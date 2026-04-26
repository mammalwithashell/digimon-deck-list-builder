import { useEffect, useState } from 'react';
import { listDecks } from '@/api/deckApi';
import { updateModel, type AIModel, type UpdateModelInput } from '@/api/modelsApi';
import type { DeckSummary } from '@/types/deck';

interface Props {
  open: boolean;
  model: AIModel | null;
  onClose: () => void;
  onSaved: () => void;
}

interface FormState {
  name: string;
  engine_commit: string;
  trained_at: string;
  deck_id: string;
  published: boolean;
  notes: string;
}

function toDatetimeLocal(iso: string | null): string {
  if (!iso) return '';
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '';
  // datetime-local value format: "YYYY-MM-DDTHH:MM"
  return d.toISOString().slice(0, 16);
}

export function AdminModelsEditModal({ open, model, onClose, onSaved }: Props) {
  const [form, setForm] = useState<FormState>({
    name: '',
    engine_commit: '',
    trained_at: '',
    deck_id: '',
    published: false,
    notes: '',
  });
  const [decks, setDecks] = useState<DeckSummary[]>([]);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (open && model) {
      setForm({
        name: model.name,
        engine_commit: model.engine_commit ?? '',
        trained_at: toDatetimeLocal(model.trained_at),
        deck_id: model.deck_id ?? '',
        published: model.published,
        notes: model.notes ?? '',
      });
      setError(null);
      listDecks().then(setDecks).catch(() => {});
    }
  }, [open, model]);

  if (!open || !model) return null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      const payload: UpdateModelInput = {
        name: form.name.trim() || undefined,
        engine_commit: form.engine_commit.trim() || null,
        trained_at: form.trained_at ? new Date(form.trained_at).toISOString() : null,
        deck_id: form.deck_id || null,
        published: form.published,
        notes: form.notes.trim() || null,
      };
      await updateModel(model.id, payload);
      onSaved();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to save changes');
    } finally {
      setSubmitting(false);
    }
  };

  const publishDisabled = model.state !== 'uploaded';

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="w-full max-w-lg bg-gray-900 border border-gray-700 rounded-lg shadow-xl p-6">
        <div className="flex items-center justify-between mb-5">
          <h2 className="text-lg font-bold text-gray-100">Edit Model</h2>
          <button
            type="button"
            onClick={onClose}
            className="text-gray-500 hover:text-gray-300 text-xl leading-none"
            aria-label="Close"
          >
            &times;
          </button>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-xs font-semibold text-gray-400 uppercase mb-1">
              Name <span className="text-red-400">*</span>
            </label>
            <input
              type="text"
              required
              value={form.name}
              onChange={(e) => setForm({ ...form, name: e.target.value })}
              className="w-full rounded bg-gray-800 border border-gray-700 px-3 py-2 text-sm text-gray-100 focus:border-blue-500 outline-none"
              maxLength={200}
            />
          </div>

          <div>
            <label className="block text-xs font-semibold text-gray-400 uppercase mb-1">
              Engine Commit
            </label>
            <input
              type="text"
              value={form.engine_commit}
              onChange={(e) => setForm({ ...form, engine_commit: e.target.value })}
              className="w-full rounded bg-gray-800 border border-gray-700 px-3 py-2 text-sm text-gray-100 focus:border-blue-500 outline-none font-mono"
              placeholder="short SHA"
              maxLength={64}
            />
          </div>

          <div>
            <label className="block text-xs font-semibold text-gray-400 uppercase mb-1">
              Trained At
            </label>
            <input
              type="datetime-local"
              value={form.trained_at}
              onChange={(e) => setForm({ ...form, trained_at: e.target.value })}
              className="w-full rounded bg-gray-800 border border-gray-700 px-3 py-2 text-sm text-gray-100 focus:border-blue-500 outline-none"
            />
          </div>

          <div>
            <label className="block text-xs font-semibold text-gray-400 uppercase mb-1">Deck</label>
            <select
              value={form.deck_id}
              onChange={(e) => setForm({ ...form, deck_id: e.target.value })}
              className="w-full rounded bg-gray-800 border border-gray-700 px-3 py-2 text-sm text-gray-100 focus:border-blue-500 outline-none"
            >
              <option value="">— None —</option>
              {decks.map((d) => (
                <option key={d.id} value={d.id}>
                  {d.name}
                </option>
              ))}
            </select>
          </div>

          <div>
            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={form.published}
                disabled={publishDisabled}
                onChange={(e) => setForm({ ...form, published: e.target.checked })}
                className="rounded border-gray-600 bg-gray-800 text-blue-500 focus:ring-blue-500"
              />
              <span className="text-sm text-gray-200">
                Published
                {publishDisabled && (
                  <span className="ml-2 text-xs text-gray-500">
                    (model must be in &apos;uploaded&apos; state)
                  </span>
                )}
              </span>
            </label>
          </div>

          <div>
            <label className="block text-xs font-semibold text-gray-400 uppercase mb-1">Notes</label>
            <textarea
              value={form.notes}
              onChange={(e) => setForm({ ...form, notes: e.target.value })}
              className="w-full rounded bg-gray-800 border border-gray-700 px-3 py-2 text-sm text-gray-100 focus:border-blue-500 outline-none"
              rows={3}
              maxLength={4000}
            />
          </div>

          {error && <p className="text-sm text-red-400">{error}</p>}

          <div className="flex gap-2 pt-1">
            <button
              type="submit"
              disabled={submitting}
              className="rounded bg-blue-500 hover:bg-blue-600 disabled:opacity-50 px-4 py-2 text-sm font-semibold text-white"
            >
              {submitting ? 'Saving…' : 'Save'}
            </button>
            <button
              type="button"
              onClick={onClose}
              className="rounded border border-gray-600 hover:border-gray-500 px-4 py-2 text-sm text-gray-300"
            >
              Cancel
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
