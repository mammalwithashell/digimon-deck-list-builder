import { useEffect, useState } from 'react';
import { GameBoard } from '@/components/board/GameBoard';
import { CardOverlay } from '@/components/game/CardOverlay';
import { CardDetailOverlay } from '@/components/game/CardDetailOverlay';
import { PLAYOUT_SPEED_MS, useStudioStore, type PlayoutSpeed } from '@/stores/studioStore';
import type { PermanentInfo } from '@/types/game';

export function ReplayStepperView() {
  const {
    replay,
    flipped,
    toggleFlipped,
    loadReplay,
    stepReplay,
    stepBackReplay,
    seekReplay,
    forkCurrentReplay,
    renameActiveReplay,
    updateActiveReplayTags,
    setView,
  } = useStudioStore();

  const [uploadError, setUploadError] = useState<string | null>(null);
  const [playing, setPlaying] = useState(false);
  const [speed, setSpeed] = useState<PlayoutSpeed>('normal');
  const [editingTitle, setEditingTitle] = useState(false);
  const [titleDraft, setTitleDraft] = useState('');
  const [tagDraft, setTagDraft] = useState('');
  const [inspectedPermanent, setInspectedPermanent] = useState<PermanentInfo | null>(null);
  const [inspectedCardId, setInspectedCardId] = useState<string | null>(null);

  // Stop auto-play when replay finishes, pauses on divergence, or changes
  useEffect(() => {
    if (!playing || !replay || replay.is_complete || replay.is_paused || replay.is_game_over) {
      if (playing && (!replay || replay.is_complete || replay.is_paused || replay.is_game_over)) {
        setPlaying(false);
      }
      return;
    }
    const timer = window.setTimeout(() => {
      void stepReplay();
    }, PLAYOUT_SPEED_MS[speed]);
    return () => window.clearTimeout(timer);
  }, [playing, speed, replay, stepReplay]);

  const handleFileUpload = async (file: File) => {
    setUploadError(null);
    setPlaying(false);
    try {
      const text = await file.text();
      const isJsonl = file.name.toLowerCase().endsWith('.jsonl');
      await loadReplay({
        source: isJsonl ? 'dcgo_jsonl' : 'native_json',
        ref: file.name,
        content: text,
        verify: true,
      });
    } catch (err) {
      setUploadError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleSlotInspect = (isOpp: boolean, slotIdx: number) => {
    if (!replay) return;
    const p = isOpp ? replay.state.player2 : replay.state.player1;
    const perm = p.battleArea[slotIdx] ?? null;
    setInspectedPermanent(perm);
  };

  const handleBreedingInspect = (isOpp: boolean) => {
    if (!replay) return;
    const p = isOpp ? replay.state.player2 : replay.state.player1;
    setInspectedPermanent(p.breedingArea ?? null);
  };

  const stepLog = replay?.step_log ?? [];

  return (
    <div style={{ display: 'flex', flexDirection: 'column', flex: 1, minHeight: 0 }} data-testid="studio-replay-view">
      {/* Top Replay Bar */}
      <div className="studio-header">
        <div style={{ display: 'flex', flexWrap: 'wrap', alignItems: 'center', gap: 8 }}>
          <button
            type="button"
            className="studio-btn"
            onClick={() => {
              setPlaying(false);
              setView('library');
            }}
          >
            ← Library
          </button>

          {replay && (
            <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
              {editingTitle ? (
                <>
                  <input
                    className="studio-input"
                    value={titleDraft}
                    onChange={(e) => setTitleDraft(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') {
                        void renameActiveReplay(titleDraft);
                        setEditingTitle(false);
                      } else if (e.key === 'Escape') {
                        setEditingTitle(false);
                      }
                    }}
                    autoFocus
                  />
                  <button
                    type="button"
                    className="studio-btn studio-btn--sm studio-btn--primary"
                    onClick={() => {
                      void renameActiveReplay(titleDraft);
                      setEditingTitle(false);
                    }}
                  >
                    Save
                  </button>
                </>
              ) : (
                <>
                  <span
                    data-testid="studio-replay-title"
                    style={{ fontWeight: 700, fontSize: 13, color: 'var(--ink-0)' }}
                  >
                    {replay.title || String(replay.source?.ref ?? 'Loaded Replay')}
                  </span>
                  <button
                    type="button"
                    className="studio-btn studio-btn--sm"
                    onClick={() => {
                      setTitleDraft(replay.title || String(replay.source?.ref ?? ''));
                      setEditingTitle(true);
                    }}
                  >
                    Rename
                  </button>
                </>
              )}
            </div>
          )}

          <label className="studio-btn" style={{ cursor: 'pointer' }}>
            Upload JSON / DCGO JSONL
            <input
              type="file"
              accept=".json,.jsonl"
              style={{ display: 'none' }}
              onChange={(e) => {
                const f = e.target.files?.[0];
                if (f) void handleFileUpload(f);
              }}
            />
          </label>
        </div>

        <div style={{ display: 'flex', flexWrap: 'wrap', alignItems: 'center', gap: 6 }}>
          {replay && (
            <>
              <button
                type="button"
                data-testid="studio-replay-play-pause"
                className={`studio-btn studio-btn--sm ${playing ? 'studio-btn--signal' : 'studio-btn--primary'}`}
                disabled={replay.is_complete || replay.is_paused || replay.is_game_over}
                onClick={() => setPlaying((p) => !p)}
              >
                {playing ? '⏸ Pause' : '▶ Play'}
              </button>
              <button
                type="button"
                className="studio-btn studio-btn--sm"
                disabled={replay.current_step === 0}
                onClick={() => {
                  setPlaying(false);
                  void stepBackReplay();
                }}
              >
                ← Prev
              </button>
              <button
                type="button"
                className="studio-btn studio-btn--sm"
                disabled={replay.is_complete || replay.is_paused}
                onClick={() => {
                  setPlaying(false);
                  void stepReplay();
                }}
              >
                Next →
              </button>
              <div className="studio-seg">
                {(['slow', 'normal', 'fast'] as PlayoutSpeed[]).map((sp) => (
                  <button
                    key={sp}
                    type="button"
                    className={`studio-seg__btn ${speed === sp ? 'is-active' : ''}`}
                    style={{ padding: '2px 6px' }}
                    onClick={() => setSpeed(sp)}
                  >
                    {sp}
                  </button>
                ))}
              </div>
              <input
                type="range"
                min={0}
                max={replay.total_steps}
                value={replay.current_step}
                onChange={(e) => {
                  setPlaying(false);
                  void seekReplay(Number(e.target.value));
                }}
                style={{ width: 140 }}
                aria-label="Replay step scrubber"
              />
              <span style={{ fontFamily: 'var(--font-mono)', fontSize: 11, color: 'var(--ink-1)' }}>
                {replay.current_step}/{replay.total_steps}
              </span>
            </>
          )}
          <button type="button" className="studio-btn studio-btn--sm" onClick={toggleFlipped}>
            Flip View
          </button>
          {replay && (
            <button
              type="button"
              data-testid="studio-fork-replay"
              className="studio-btn studio-btn--sm studio-btn--primary"
              onClick={() => {
                setPlaying(false);
                void forkCurrentReplay();
              }}
            >
              Fork Into Studio →
            </button>
          )}
        </div>
      </div>

      {replay && (
        <div className="studio-tag-bar" data-testid="studio-replay-tags">
          <span style={{ fontSize: 11, color: 'var(--ink-3)', fontFamily: 'var(--font-mono)' }}>TAGS</span>
          {(replay.auto_tags ?? []).map((t) => {
            const cls =
              t === 'diverged'
                ? 'studio-chip studio-chip--danger'
                : t === 'clean' || t === 'no-concede'
                  ? 'studio-chip studio-chip--ok'
                  : t === 'error' || t === 'concede' || t === 'unfinished'
                    ? 'studio-chip studio-chip--warn'
                    : 'studio-chip studio-chip--auto';
            return (
              <span key={`auto-${t}`} className={cls} title="Auto-tag" style={{ cursor: 'default' }}>
                #{t}
              </span>
            );
          })}
          {(replay.user_tags ?? []).map((t) => (
            <span key={`user-${t}`} className="studio-chip" style={{ cursor: 'default' }}>
              #{t}
              <button
                type="button"
                aria-label={`Remove tag ${t}`}
                onClick={() =>
                  void updateActiveReplayTags((replay.user_tags ?? []).filter((x) => x !== t))
                }
                style={{
                  background: 'none',
                  border: 'none',
                  color: 'inherit',
                  cursor: 'pointer',
                  padding: '0 2px',
                  fontSize: 12,
                  lineHeight: 1,
                }}
              >
                ×
              </button>
            </span>
          ))}
          <form
            style={{ display: 'inline-flex', alignItems: 'center', gap: 4 }}
            onSubmit={(e) => {
              e.preventDefault();
              const next = tagDraft.trim().toLowerCase();
              if (!next) return;
              void updateActiveReplayTags([...(replay.user_tags ?? []), next]);
              setTagDraft('');
            }}
          >
            <input
              className="studio-input"
              style={{ width: 120, padding: '2px 6px', fontSize: 11 }}
              placeholder="+ add tag..."
              aria-label="Add replay tag"
              value={tagDraft}
              onChange={(e) => setTagDraft(e.target.value)}
            />
            <button type="submit" className="studio-btn studio-btn--sm">
              + Tag
            </button>
          </form>
        </div>
      )}

      {uploadError && <div className="studio-banner studio-banner--error">{uploadError}</div>}
      {replay?.truncation_warning && (
        <div className="studio-banner studio-banner--warn">
          Raw DCGO conversion note: {replay.truncation_warning}
        </div>
      )}
      {replay && replay.divergences.length > 0 && (
        <div className="studio-banner studio-banner--error">
          Divergence detected ({replay.divergences.length}): {JSON.stringify(replay.divergences[0])}
        </div>
      )}

      {/* Main split: Live Board + Action Log side panel */}
      <div className="studio-body">
        <div className="studio-stage">
          {replay ? (
            <>
              <GameBoard
                dualSeat
                flipped={flipped}
                onSlotInspect={handleSlotInspect}
                onBreedingInspect={handleBreedingInspect}
                onHandCardInspect={(cid) => setInspectedCardId(cid)}
              />
              <CardOverlay
                permanent={inspectedPermanent}
                onClose={() => setInspectedPermanent(null)}
                onInspectCard={(cid) => setInspectedCardId(cid)}
              />
              <CardDetailOverlay
                cardId={inspectedCardId}
                onClose={() => setInspectedCardId(null)}
              />
            </>
          ) : (
            <div style={{ display: 'flex', height: '100%', alignItems: 'center', justifyContent: 'center', color: 'var(--ink-2)', fontSize: 13 }}>
              Select a replay from the Library or upload a native JSON / DCGO JSONL file above.
            </div>
          )}
        </div>

        {replay && (
          <aside className="studio-rail-right" data-testid="studio-replay-action-log">
            <div className="studio-panel" style={{ flex: 1, minHeight: 0 }}>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                <span className="studio-panel__title">Action Log ({stepLog.length})</span>
                <span style={{ fontSize: 10, color: 'var(--ink-3)', fontFamily: 'var(--font-mono)' }}>
                  Right-click card/stack to inspect
                </span>
              </div>
              <div style={{ flex: 1, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: 4 }}>
                {stepLog.length === 0 && (
                  <div style={{ color: 'var(--ink-3)', fontSize: 11, padding: '8px 0' }}>
                    Step or press Play to watch actions appear here.
                  </div>
                )}
                {stepLog.map((entry, idx) => {
                  const isCurrent = idx + 1 === replay.current_step;
                  return (
                    <button
                      key={`${entry.step_number}-${idx}`}
                      type="button"
                      className={`studio-row ${isCurrent ? 'is-selected' : ''}`}
                      style={{
                        width: '100%',
                        textAlign: 'left',
                        cursor: 'pointer',
                        flexDirection: 'column',
                        alignItems: 'stretch',
                        gap: 2,
                      }}
                      onClick={() => {
                        setPlaying(false);
                        void seekReplay(idx + 1);
                      }}
                    >
                      <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 11 }}>
                        <span
                          style={{
                            fontWeight: 700,
                            color: entry.player_id === 1 ? 'var(--player-2)' : 'var(--opp-2)',
                          }}
                        >
                          #{idx + 1} · P{entry.player_id ?? '?'} · {entry.label}
                        </span>
                        <span style={{ fontFamily: 'var(--font-mono)', fontSize: 10, color: 'var(--ink-2)' }}>
                          Mem {entry.memory_after}
                        </span>
                      </div>
                      <div style={{ fontFamily: 'var(--font-mono)', fontSize: 10, color: 'var(--ink-3)' }}>
                        T{entry.turn_number} · {entry.phase_before} → {entry.phase_after}
                      </div>
                    </button>
                  );
                })}
              </div>
            </div>
          </aside>
        )}
      </div>
    </div>
  );
}
