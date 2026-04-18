import { useEffect, useRef, useState } from 'react';
import { listDecks } from '@/api/deckApi';
import {
  confirmModel,
  createModel,
  deleteModel,
  uploadWithProgress,
  type ConfirmModelResponse,
  type CreateModelResponse,
  type ModelType,
} from '@/api/modelsApi';
import type { DeckSummary } from '@/types/deck';

interface Props {
  open: boolean;
  onClose: () => void;
  onCreated: () => void;
}

type Step = 'metadata' | 'upload' | 'confirm';

interface MetaFormState {
  name: string;
  model_type: ModelType;
  engine_commit: string;
  trained_at: string;
  deck_id: string;
  notes: string;
}

const EMPTY_META: MetaFormState = {
  name: '',
  model_type: 'mlp',
  engine_commit: '',
  trained_at: '',
  deck_id: '',
  notes: '',
};

function formatBytes(bytes: number): string {
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
}

export function AdminModelsUploadModal({ open, onClose, onCreated }: Props) {
  const [step, setStep] = useState<Step>('metadata');
  const [meta, setMeta] = useState<MetaFormState>(EMPTY_META);
  const [decks, setDecks] = useState<DeckSummary[]>([]);
  const [createResp, setCreateResp] = useState<CreateModelResponse | null>(null);
  const [confirmResp, setConfirmResp] = useState<ConfirmModelResponse | null>(null);

  const [file, setFile] = useState<File | null>(null);
  const [uploadPct, setUploadPct] = useState(0);

  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fileInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (open) {
      listDecks().then(setDecks).catch(() => {});
    }
  }, [open]);

  // Reset all state when the modal opens
  useEffect(() => {
    if (open) {
      setStep('metadata');
      setMeta(EMPTY_META);
      setCreateResp(null);
      setConfirmResp(null);
      setFile(null);
      setUploadPct(0);
      setError(null);
      setSubmitting(false);
    }
  }, [open]);

  if (!open) return null;

  const handleMetaSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      const resp = await createModel({
        name: meta.name.trim(),
        model_type: meta.model_type,
        engine_commit: meta.engine_commit.trim() || null,
        trained_at: meta.trained_at ? new Date(meta.trained_at).toISOString() : null,
        deck_id: meta.deck_id || null,
        notes: meta.notes.trim() || null,
      });
      setCreateResp(resp);
      setStep('upload');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create model record');
    } finally {
      setSubmitting(false);
    }
  };

  const handleUpload = async () => {
    if (!file || !createResp) return;
    setError(null);
    setSubmitting(true);
    setUploadPct(0);
    try {
      await uploadWithProgress(createResp.upload_url, file, setUploadPct);
      setStep('confirm');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Upload failed');
    } finally {
      setSubmitting(false);
    }
  };

  const handleConfirm = async () => {
    if (!createResp) return;
    setError(null);
    setSubmitting(true);
    try {
      const resp = await confirmModel(createResp.id);
      setConfirmResp(resp);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Confirm failed');
    } finally {
      setSubmitting(false);
    }
  };

  const handleDeletePending = async () => {
    if (!createResp) return;
    try {
      await deleteModel(createResp.id);
    } catch {
      // best effort
    }
    onClose();
  };

  const handleDone = () => {
    onCreated();
    onClose();
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="w-full max-w-lg bg-gray-900 border border-gray-700 rounded-lg shadow-xl p-6">
        {/* Header */}
        <div className="flex items-center justify-between mb-5">
          <h2 className="text-lg font-bold text-gray-100">
            Upload Model
            <span className="ml-3 text-sm font-normal text-gray-400">
              Step {step === 'metadata' ? 1 : step === 'upload' ? 2 : 3} / 3
            </span>
          </h2>
          <button
            type="button"
            onClick={onClose}
            className="text-gray-500 hover:text-gray-300 text-xl leading-none"
            aria-label="Close"
          >
            &times;
          </button>
        </div>

        {/* Step 1: Metadata */}
        {step === 'metadata' && (
          <form onSubmit={handleMetaSubmit} className="space-y-4">
            <div>
              <label className="block text-xs font-semibold text-gray-400 uppercase mb-1">
                Name <span className="text-red-400">*</span>
              </label>
              <input
                type="text"
                required
                value={meta.name}
                onChange={(e) => setMeta({ ...meta, name: e.target.value })}
                className="w-full rounded bg-gray-800 border border-gray-700 px-3 py-2 text-sm text-gray-100 focus:border-blue-500 outline-none"
                placeholder="MLP vs Beelzemon (standard)"
                maxLength={200}
              />
            </div>

            <div>
              <label className="block text-xs font-semibold text-gray-400 uppercase mb-1">
                Model Type <span className="text-red-400">*</span>
              </label>
              <select
                required
                value={meta.model_type}
                onChange={(e) => setMeta({ ...meta, model_type: e.target.value as ModelType })}
                className="w-full rounded bg-gray-800 border border-gray-700 px-3 py-2 text-sm text-gray-100 focus:border-blue-500 outline-none"
              >
                <option value="mlp">MLP</option>
                <option value="lstm">LSTM</option>
              </select>
            </div>

            <div>
              <label className="block text-xs font-semibold text-gray-400 uppercase mb-1">
                Engine Commit (optional)
              </label>
              <input
                type="text"
                value={meta.engine_commit}
                onChange={(e) => setMeta({ ...meta, engine_commit: e.target.value })}
                className="w-full rounded bg-gray-800 border border-gray-700 px-3 py-2 text-sm text-gray-100 focus:border-blue-500 outline-none font-mono"
                placeholder="git rev-parse HEAD (short SHA)"
                maxLength={64}
              />
            </div>

            <div>
              <label className="block text-xs font-semibold text-gray-400 uppercase mb-1">
                Trained At (optional)
              </label>
              <input
                type="datetime-local"
                value={meta.trained_at}
                onChange={(e) => setMeta({ ...meta, trained_at: e.target.value })}
                className="w-full rounded bg-gray-800 border border-gray-700 px-3 py-2 text-sm text-gray-100 focus:border-blue-500 outline-none"
              />
            </div>

            <div>
              <label className="block text-xs font-semibold text-gray-400 uppercase mb-1">
                Deck (optional)
              </label>
              <select
                value={meta.deck_id}
                onChange={(e) => setMeta({ ...meta, deck_id: e.target.value })}
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
              <label className="block text-xs font-semibold text-gray-400 uppercase mb-1">
                Notes (optional)
              </label>
              <textarea
                value={meta.notes}
                onChange={(e) => setMeta({ ...meta, notes: e.target.value })}
                className="w-full rounded bg-gray-800 border border-gray-700 px-3 py-2 text-sm text-gray-100 focus:border-blue-500 outline-none"
                rows={3}
                maxLength={4000}
                placeholder="Optional markdown notes…"
              />
            </div>

            {error && <p className="text-sm text-red-400">{error}</p>}

            <div className="flex gap-2 pt-1">
              <button
                type="submit"
                disabled={submitting}
                className="rounded bg-blue-500 hover:bg-blue-600 disabled:opacity-50 px-4 py-2 text-sm font-semibold text-white"
              >
                {submitting ? 'Creating…' : 'Next: Upload File'}
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
        )}

        {/* Step 2: Upload */}
        {step === 'upload' && createResp && (
          <div className="space-y-4">
            <p className="text-sm text-gray-400">
              Select your <code className="text-gray-200">.onnx</code> file and click Upload. The
              file will be sent directly to storage.
            </p>

            <div>
              <label className="block text-xs font-semibold text-gray-400 uppercase mb-1">
                ONNX File <span className="text-red-400">*</span>
              </label>
              <input
                ref={fileInputRef}
                type="file"
                accept=".onnx"
                disabled={submitting}
                onChange={(e) => setFile(e.target.files?.[0] ?? null)}
                className="w-full rounded bg-gray-800 border border-gray-700 px-3 py-2 text-sm text-gray-300 file:mr-3 file:rounded file:border-0 file:bg-gray-700 file:px-3 file:py-1 file:text-xs file:text-gray-200 focus:border-blue-500 outline-none"
              />
              {file && (
                <p className="mt-1 text-xs text-gray-500">
                  {file.name} — {formatBytes(file.size)}
                </p>
              )}
            </div>

            {submitting && (
              <div>
                <div className="flex justify-between text-xs text-gray-400 mb-1">
                  <span>Uploading…</span>
                  <span>{uploadPct}%</span>
                </div>
                <div className="w-full h-2 bg-gray-700 rounded overflow-hidden">
                  <div
                    className="h-2 bg-blue-500 transition-all duration-100"
                    style={{ width: `${uploadPct}%` }}
                  />
                </div>
              </div>
            )}

            {error && (
              <div>
                <p className="text-sm text-red-400 mb-2">{error}</p>
                <button
                  type="button"
                  onClick={() => { void handleDeletePending(); }}
                  className="rounded border border-red-500/60 hover:bg-red-500/10 px-3 py-1.5 text-xs text-red-300"
                >
                  Delete pending row and close
                </button>
              </div>
            )}

            <div className="flex gap-2 pt-1">
              <button
                type="button"
                disabled={!file || submitting}
                onClick={() => { void handleUpload(); }}
                className="rounded bg-blue-500 hover:bg-blue-600 disabled:opacity-50 px-4 py-2 text-sm font-semibold text-white"
              >
                {submitting ? 'Uploading…' : 'Upload'}
              </button>
              <button
                type="button"
                disabled={submitting}
                onClick={() => { void handleDeletePending(); }}
                className="rounded border border-gray-600 hover:border-gray-500 px-4 py-2 text-sm text-gray-300 disabled:opacity-50"
              >
                Cancel
              </button>
            </div>
          </div>
        )}

        {/* Step 3: Confirm */}
        {step === 'confirm' && createResp && (
          <div className="space-y-4">
            {!confirmResp ? (
              <>
                <p className="text-sm text-gray-400">
                  Upload complete. Click Confirm to let the server hash the file and inspect the
                  ONNX model. This may take 5–15 seconds.
                </p>

                {error && (
                  <div>
                    <p className="text-sm text-red-400 mb-2">{error}</p>
                    <button
                      type="button"
                      onClick={() => { void handleDeletePending(); }}
                      className="rounded border border-red-500/60 hover:bg-red-500/10 px-3 py-1.5 text-xs text-red-300"
                    >
                      Delete pending row and close
                    </button>
                  </div>
                )}

                <div className="flex gap-2 pt-1">
                  <button
                    type="button"
                    disabled={submitting}
                    onClick={() => { void handleConfirm(); }}
                    className="rounded bg-green-600 hover:bg-green-700 disabled:opacity-50 px-4 py-2 text-sm font-semibold text-white"
                  >
                    {submitting ? 'Confirming…' : 'Confirm'}
                  </button>
                  <button
                    type="button"
                    disabled={submitting}
                    onClick={() => { void handleDeletePending(); }}
                    className="rounded border border-gray-600 hover:border-gray-500 px-4 py-2 text-sm text-gray-300 disabled:opacity-50"
                  >
                    Cancel
                  </button>
                </div>
              </>
            ) : (
              <>
                <div className="rounded border border-green-500/40 bg-green-500/10 p-4 space-y-2">
                  <p className="text-sm font-semibold text-green-300">Model confirmed successfully.</p>
                  <dl className="grid grid-cols-2 gap-x-4 gap-y-1 text-xs text-gray-300 mt-2">
                    <dt className="text-gray-500">State</dt>
                    <dd>{confirmResp.state}</dd>
                    <dt className="text-gray-500">Tensor size</dt>
                    <dd>{confirmResp.tensor_size}</dd>
                    <dt className="text-gray-500">Action space</dt>
                    <dd>{confirmResp.action_space_size}</dd>
                    <dt className="text-gray-500">File size</dt>
                    <dd>{formatBytes(confirmResp.file_size_bytes)}</dd>
                    <dt className="text-gray-500 col-span-1">SHA-256</dt>
                    <dd className="col-span-1 font-mono break-all">{confirmResp.file_sha256}</dd>
                  </dl>
                </div>

                <div className="flex gap-2 pt-1">
                  <button
                    type="button"
                    onClick={handleDone}
                    className="rounded bg-blue-500 hover:bg-blue-600 px-4 py-2 text-sm font-semibold text-white"
                  >
                    Done
                  </button>
                </div>
              </>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
