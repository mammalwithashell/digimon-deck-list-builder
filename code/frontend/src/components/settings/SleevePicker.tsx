import { useEffect, useMemo, useRef, useState, useSyncExternalStore, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import {
  CARD_SLEEVES,
  CUSTOM_CARD_SLEEVE_ID,
  CUSTOM_EGG_SLEEVE_ID,
  DEFAULT_CARD_SLEEVE,
  DEFAULT_EGG_SLEEVE,
  clearCustomCardSleeve,
  clearCustomEggSleeve,
  getCardSleeveId,
  getCustomCardSleeveSrc,
  getCustomEggSleeveSrc,
  getEggSleeveId,
  resetCardSleeve,
  resetEggSleeve,
  setCardSleeveId,
  setCustomCardSleeve,
  setCustomEggSleeve,
  setEggSleeveId,
  sleeveUploadHelper,
  subscribeCardSleeve,
  useCardSleeve,
  useEggSleeve,
  type CardSleeve,
} from '@/stores/sleeveStore';

function SleeveThumbnail({
  sleeve,
  isEgg = false,
}: {
  sleeve: CardSleeve;
  isEgg?: boolean;
}) {
  const [imgFailed, setImgFailed] = useState(false);

  useEffect(() => {
    setImgFailed(false);
  }, [sleeve.src]);

  if (sleeve.src && !imgFailed) {
    return (
      <img
        src={sleeve.src}
        alt=""
        className="w-full h-full object-cover"
        draggable={false}
        onError={() => setImgFailed(true)}
      />
    );
  }

  return (
    <div
      className={`w-full h-full flex items-center justify-center ${
        isEgg
          ? 'bg-gradient-to-br from-slate-100 to-slate-300 text-slate-700'
          : 'bg-gradient-to-br from-blue-900 to-blue-700 text-blue-200'
      }`}
      aria-hidden="true"
    >
      <span className="text-[10px] font-bold tracking-wider">DCG</span>
    </div>
  );
}

interface SleeveGridProps {
  sleeves: readonly CardSleeve[];
  selectedId: string;
  onSelect: (id: string) => void;
  leading?: ReactNode;
  isEgg?: boolean;
}

function SleeveGrid({
  sleeves,
  selectedId,
  onSelect,
  leading,
  isEgg = false,
}: SleeveGridProps) {
  return (
    <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-5 gap-2.5">
      {leading}
      {sleeves.map((sleeve) => {
        const selected = sleeve.id === selectedId;
        return (
          <button
            key={sleeve.id}
            type="button"
            aria-label={`${sleeve.label}, ${sleeve.collection}`}
            aria-pressed={selected}
            onClick={() => onSelect(sleeve.id)}
            className={`group flex flex-col items-center gap-1.5 p-2 rounded-lg border text-left transition-all cursor-pointer ${
              selected
                ? 'border-[var(--accent)] bg-[var(--surface-raised)] ring-2 ring-[var(--accent)]/40 shadow-md'
                : 'border-[var(--line-1)] bg-[var(--surface-sunken)] hover:border-[var(--accent)]/70 hover:bg-[var(--surface-raised)]'
            }`}
          >
            <span className="relative w-14 aspect-[5/7] rounded overflow-hidden border border-white/15 bg-slate-950 shrink-0 shadow-sm">
              <SleeveThumbnail sleeve={sleeve} isEgg={isEgg} />
              {selected && (
                <span className="absolute top-1 right-1 px-1 py-0.5 rounded bg-[var(--accent)] text-[var(--accent-ink)] text-[8px] font-bold uppercase leading-none shadow">
                  Active
                </span>
              )}
            </span>
            <span className="w-full text-center">
              <span className="block text-[11px] font-semibold text-[var(--ink-0)] leading-tight line-clamp-2">
                {sleeve.label}
              </span>
              <span className="block text-[9px] text-[var(--ink-2)] leading-tight truncate mt-0.5">
                {sleeve.collection}
              </span>
            </span>
          </button>
        );
      })}
    </div>
  );
}

function useSleeveUpload({
  customSrc,
  save,
  clear,
  isDefault,
  onResetDefault,
  uploadLabel,
}: {
  customSrc: string | undefined;
  save: (dataUrl: string) => void;
  clear: () => void;
  isDefault: boolean;
  onResetDefault: () => void;
  uploadLabel: string;
}) {
  const fileRef = useRef<HTMLInputElement>(null);
  const [uploadError, setUploadError] = useState<string>();
  const [uploading, setUploading] = useState(false);

  async function pickFile(file: File | undefined) {
    if (!file) return;
    setUploadError(undefined);
    if (!file.type.startsWith('image/')) {
      setUploadError('Please select an image file.');
      return;
    }
    setUploading(true);
    try {
      const dataUrl = await sleeveUploadHelper.toStorableDataUrl(file);
      save(dataUrl);
    } catch {
      setUploadError('Could not load or store this image. Try another or smaller file.');
    } finally {
      setUploading(false);
    }
  }

  const tile = (
    <button
      type="button"
      disabled={uploading}
      onClick={() => fileRef.current?.click()}
      className="flex flex-col items-center justify-center gap-1.5 p-2 rounded-lg border border-dashed border-[var(--line-strong)] bg-[var(--surface-sunken)] hover:border-[var(--accent)] hover:bg-[var(--surface-raised)] transition-colors cursor-pointer disabled:opacity-50"
    >
      <span className="w-14 aspect-[5/7] rounded border border-[var(--line-1)] bg-[var(--surface)] flex flex-col items-center justify-center text-[var(--accent)]">
        <span className="text-lg leading-none font-bold">+</span>
        <span className="text-[9px] font-semibold uppercase tracking-wider mt-1">
          Image
        </span>
      </span>
      <span className="text-[11px] font-semibold text-[var(--ink-0)] text-center leading-tight">
        {uploading
          ? 'Uploading…'
          : customSrc
            ? 'Replace image'
            : uploadLabel}
      </span>
    </button>
  );

  const controls = (
    <div className="mt-3 flex flex-wrap items-center justify-between gap-2 pt-3 border-t border-[var(--line-0)]">
      <p className="text-[11px] text-[var(--ink-2)]">
        Saved locally on this device. Portrait images fill the sleeve frame best.
      </p>
      <div className="flex items-center gap-2">
        {!isDefault && (
          <button
            type="button"
            onClick={() => {
              onResetDefault();
              setUploadError(undefined);
            }}
            className="px-2.5 py-1 rounded border border-[var(--line-1)] bg-[var(--surface-sunken)] hover:border-[var(--accent)] text-[11px] font-semibold text-[var(--ink-1)] hover:text-[var(--ink-0)] cursor-pointer"
          >
            Reset to default
          </button>
        )}
        {customSrc && (
          <button
            type="button"
            disabled={uploading}
            onClick={() => {
              clear();
              setUploadError(undefined);
            }}
            className="px-2.5 py-1 rounded border border-[var(--danger)]/60 bg-transparent hover:bg-[var(--danger)] text-[11px] font-semibold text-[var(--danger)] hover:text-white cursor-pointer"
          >
            Remove custom image
          </button>
        )}
      </div>
      <input
        ref={fileRef}
        type="file"
        accept="image/*"
        hidden
        aria-label={uploadLabel}
        onChange={(event) => {
          void pickFile(event.target.files?.[0]);
          event.target.value = '';
        }}
      />
      {uploadError && (
        <p className="w-full text-xs text-[var(--danger)] font-medium" role="status">
          {uploadError}
        </p>
      )}
    </div>
  );

  return { tile, controls };
}

function filterSleeves(sleeves: readonly CardSleeve[], query: string): readonly CardSleeve[] {
  const trimmed = query.trim().toLowerCase();
  if (!trimmed) return sleeves;
  return sleeves.filter(
    (s) =>
      s.label.toLowerCase().includes(trimmed) ||
      s.collection.toLowerCase().includes(trimmed) ||
      s.id.toLowerCase().includes(trimmed),
  );
}

/** Visual grid picker for Main Card Sleeves (Deck, Hand, Security). */
export function CardSleevePicker({ query = '' }: { query?: string }) {
  const selectedId = useSyncExternalStore(
    subscribeCardSleeve,
    getCardSleeveId,
    () => DEFAULT_CARD_SLEEVE.id,
  );
  const customSrc = useSyncExternalStore(
    subscribeCardSleeve,
    getCustomCardSleeveSrc,
    () => undefined,
  );
  const upload = useSleeveUpload({
    customSrc,
    save: setCustomCardSleeve,
    clear: clearCustomCardSleeve,
    isDefault: selectedId === DEFAULT_CARD_SLEEVE.id,
    onResetDefault: resetCardSleeve,
    uploadLabel: 'Upload sleeve image',
  });

  const allSleeves = useMemo(() => {
    const base = customSrc
      ? [
          {
            id: CUSTOM_CARD_SLEEVE_ID,
            label: 'Custom Image',
            collection: 'This device',
            src: customSrc,
          },
          ...CARD_SLEEVES,
        ]
      : CARD_SLEEVES;
    return filterSleeves(base, query);
  }, [customSrc, query]);

  return (
    <div data-testid="card-sleeve-picker">
      <SleeveGrid
        sleeves={allSleeves}
        selectedId={selectedId}
        onSelect={setCardSleeveId}
        leading={upload.tile}
      />
      {upload.controls}
    </div>
  );
}

/** Visual grid picker for Digi-Egg Sleeves (Egg Deck). */
export function EggSleevePicker({ query = '' }: { query?: string }) {
  const selectedId = useSyncExternalStore(
    subscribeCardSleeve,
    getEggSleeveId,
    () => DEFAULT_EGG_SLEEVE.id,
  );
  const customSrc = useSyncExternalStore(
    subscribeCardSleeve,
    getCustomCardSleeveSrc,
    () => undefined,
  );
  const customEggSrc = useSyncExternalStore(
    subscribeCardSleeve,
    getCustomEggSleeveSrc,
    () => undefined,
  );
  const upload = useSleeveUpload({
    customSrc: customEggSrc,
    save: setCustomEggSleeve,
    clear: clearCustomEggSleeve,
    isDefault: selectedId === DEFAULT_EGG_SLEEVE.id,
    onResetDefault: resetEggSleeve,
    uploadLabel: 'Upload egg sleeve',
  });

  const allSleeves = useMemo(() => {
    const custom: CardSleeve[] = [
      ...(customEggSrc
        ? [
            {
              id: CUSTOM_EGG_SLEEVE_ID,
              label: 'Custom Egg Image',
              collection: 'This device',
              src: customEggSrc,
            },
          ]
        : []),
      ...(customSrc
        ? [
            {
              id: CUSTOM_CARD_SLEEVE_ID,
              label: 'Custom Image',
              collection: 'Uploaded card sleeve',
              src: customSrc,
            },
          ]
        : []),
    ];
    return filterSleeves([DEFAULT_EGG_SLEEVE, ...custom, ...CARD_SLEEVES], query);
  }, [customEggSrc, customSrc, query]);

  return (
    <div data-testid="egg-sleeve-picker">
      <SleeveGrid
        sleeves={allSleeves}
        selectedId={selectedId}
        onSelect={setEggSleeveId}
        leading={upload.tile}
        isEgg
      />
      {upload.controls}
    </div>
  );
}

export interface SleevePickerProps {
  initialTab?: 'card' | 'egg';
}

/** Tabbed sleeve customizer for both Main Card Sleeves and Digi-Egg Sleeves. */
export function SleevePicker({ initialTab = 'card' }: SleevePickerProps) {
  const [tab, setTab] = useState<'card' | 'egg'>(initialTab);
  const [query, setQuery] = useState('');
  const activeCardSleeve = useCardSleeve();
  const activeEggSleeve = useEggSleeve();

  useEffect(() => {
    setTab(initialTab);
  }, [initialTab]);

  return (
    <div className="flex flex-col gap-4" data-testid="sleeve-picker">
      {/* Live active preview strip */}
      <div className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-[var(--line-1)] bg-[var(--surface-raised)] px-3.5 py-2.5">
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2">
            <span className="w-8 aspect-[5/7] rounded overflow-hidden border border-white/20 bg-slate-950 shrink-0">
              <SleeveThumbnail sleeve={activeCardSleeve} />
            </span>
            <div>
              <div className="text-[10px] uppercase tracking-wider text-[var(--ink-2)] font-semibold">
                Main Sleeve
              </div>
              <div className="text-xs font-semibold text-[var(--ink-0)]">
                {activeCardSleeve.label}
              </div>
            </div>
          </div>

          <div className="h-7 w-px bg-[var(--line-0)]" aria-hidden="true" />

          <div className="flex items-center gap-2">
            <span className="w-8 aspect-[5/7] rounded overflow-hidden border border-white/20 bg-slate-950 shrink-0">
              <SleeveThumbnail sleeve={activeEggSleeve} isEgg />
            </span>
            <div>
              <div className="text-[10px] uppercase tracking-wider text-[var(--ink-2)] font-semibold">
                Digi-Egg Sleeve
              </div>
              <div className="text-xs font-semibold text-[var(--ink-0)]">
                {activeEggSleeve.label}
              </div>
            </div>
          </div>
        </div>

        <input
          type="search"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search sleeves…"
          aria-label="Search sleeves"
          className="px-2.5 py-1 rounded-md border border-[var(--line-1)] bg-[var(--surface-sunken)] text-xs text-[var(--ink-0)] placeholder:text-[var(--ink-2)] focus:outline-none focus:border-[var(--accent)] w-44"
        />
      </div>

      {/* Category tabs */}
      <div className="flex items-center gap-2" role="tablist" aria-label="Sleeve categories">
        <button
          type="button"
          role="tab"
          aria-selected={tab === 'card'}
          onClick={() => setTab('card')}
          className={`px-3.5 py-1.5 rounded-md text-xs font-semibold border transition-colors cursor-pointer ${
            tab === 'card'
              ? 'border-[var(--accent)] bg-[var(--surface-raised)] text-[var(--ink-0)]'
              : 'border-[var(--line-0)] bg-[var(--surface-sunken)] text-[var(--ink-2)] hover:text-[var(--ink-0)]'
          }`}
        >
          Main Card Sleeves
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={tab === 'egg'}
          onClick={() => setTab('egg')}
          className={`px-3.5 py-1.5 rounded-md text-xs font-semibold border transition-colors cursor-pointer ${
            tab === 'egg'
              ? 'border-[var(--accent)] bg-[var(--surface-raised)] text-[var(--ink-0)]'
              : 'border-[var(--line-0)] bg-[var(--surface-sunken)] text-[var(--ink-2)] hover:text-[var(--ink-0)]'
          }`}
        >
          Digi-Egg Sleeves
        </button>
      </div>

      <div className="max-h-[48vh] overflow-y-auto pr-1">
        {tab === 'card' ? (
          <CardSleevePicker query={query} />
        ) : (
          <EggSleevePicker query={query} />
        )}
      </div>
    </div>
  );
}

export interface SleevePickerModalProps {
  isOpen: boolean;
  onClose: () => void;
  initialTab?: 'card' | 'egg';
}

/**
 * Centered portal modal wrapping {@link SleevePicker} for use inside the match
 * ArenaHud or Settings screens. Renders into `document.body` so fixed
 * positioning stays crisp even inside scaled/transformed board containers.
 */
export function SleevePickerModal({
  isOpen,
  onClose,
  initialTab = 'card',
}: SleevePickerModalProps) {
  useEffect(() => {
    if (!isOpen) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        e.stopImmediatePropagation();
        onClose();
      }
    };
    document.addEventListener('keydown', onKeyDown, true);
    return () => document.removeEventListener('keydown', onKeyDown, true);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  const content = (
    <div
      className="fixed inset-0 z-[1100] flex items-center justify-center p-4 bg-black/65"
      role="presentation"
      data-testid="sleeve-picker-modal"
      onClick={onClose}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Card & Digi-Egg Sleeves"
        className="relative w-full max-w-3xl max-h-[88vh] flex flex-col rounded-xl border border-[var(--line-1)] bg-[var(--surface)] text-[var(--ink-0)] shadow-2xl overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between px-5 py-3.5 border-b border-[var(--line-1)] bg-[var(--surface-raised)]">
          <div>
            <h2
              className="text-base font-bold text-[var(--ink-0)]"
              style={{ fontFamily: 'var(--font-display)' }}
            >
              Card &amp; Digi-Egg Sleeves
            </h2>
            <p className="text-xs text-[var(--ink-2)]">
              Customize card backs for your deck, hand, security, and Digi-Egg pile.
            </p>
          </div>
          <button
            type="button"
            aria-label="Close sleeve picker"
            onClick={onClose}
            className="w-7 h-7 grid place-items-center rounded border border-[var(--line-1)] bg-[var(--surface)] text-[var(--ink-1)] hover:text-[var(--ink-0)] hover:border-[var(--accent)] cursor-pointer"
          >
            ✕
          </button>
        </div>
        <div className="p-5 overflow-y-auto">
          <SleevePicker initialTab={initialTab} />
        </div>
      </div>
    </div>
  );

  return typeof document !== 'undefined'
    ? createPortal(content, document.body)
    : content;
}
