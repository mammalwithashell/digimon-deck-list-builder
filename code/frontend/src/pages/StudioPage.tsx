import { useEffect, useMemo, useState } from 'react';
import { GameBoard } from '@/components/board/GameBoard';
import { SelectionPanel } from '@/components/game/SelectionPanel';
import { CardOverlay } from '@/components/game/CardOverlay';
import { CardDetailOverlay } from '@/components/game/CardDetailOverlay';
import { StudioLibraryView } from '@/components/studio/StudioLibraryView';
import {
  CardSearchDrawer,
  type PlacementTarget,
} from '@/components/studio/CardSearchDrawer';
import { StackInspector } from '@/components/studio/StackInspector';
import {
  ZoneEditorModal,
  type EditableZoneKind,
} from '@/components/studio/ZoneEditorModal';
import { AgentInspectorPanel } from '@/components/studio/AgentInspectorPanel';
import { ReplayStepperView } from '@/components/studio/ReplayStepperView';
import { PuzzleSaveExportModal } from '@/components/studio/PuzzleSaveExportModal';
import type { StudioBoardPayload, StudioFieldStack, StudioZoneSpec } from '@/api/studioTransport';
import { PLAYOUT_SPEED_MS, useStudioStore } from '@/stores/studioStore';
import { ACTION } from '@/utils/constants';
import type { PermanentInfo } from '@/types/game';
import './StudioPage.css';

export function StudioPage() {
  const {
    view,
    mode,
    flipped,
    error,
    statusMessage,
    session,
    playoutRunning,
    playoutSpeed,
    selectedStepIndex,
    expectations,
    assertions,
    assertionResults,
    puzzleTitle,
    loadBootstrap,
    setView,
    setMode,
    toggleFlipped,
    clearError,
    updateBoard,
    stepHuman,
    stepModel,
    rewindTo,
    snapshotAsStart,
    saveNamedLine,
    deleteNamedLine,
    loadNamedLine,
    addExpectation,
    removeExpectation,
    addAssertion,
    removeAssertion,
    evaluateCurrentAssertions,
    setSelectedStepIndex,
  } = useStudioStore();

  const [targetPlayer, setTargetPlayer] = useState<1 | 2>(1);
  const [placementTarget, setPlacementTarget] = useState<PlacementTarget>('field_new');
  const [selectedStack, setSelectedStack] = useState<{ player: 1 | 2; slotIndex: number } | null>(null);
  const [zoneModal, setZoneModal] = useState<{ player: 1 | 2; zone: EditableZoneKind } | null>(null);
  const [showSaveModal, setShowSaveModal] = useState(false);
  const [lineName, setLineName] = useState('');
  const [lineRole, setLineRole] = useState<'target' | 'avoid' | 'alternative'>('target');
  const [newAssertKind, setNewAssertKind] = useState<string>('action_legal');
  const [newAssertVal, setNewAssertVal] = useState<string>('');
  const [newAssertPlayer, setNewAssertPlayer] = useState<1 | 2>(1);
  const [inspectedPermanent, setInspectedPermanent] = useState<PermanentInfo | null>(null);
  const [inspectedCardId, setInspectedCardId] = useState<string | null>(null);

  useEffect(() => {
    void loadBootstrap();
  }, [loadBootstrap]);

  useEffect(() => {
    if (!playoutRunning || view !== 'workspace' || mode !== 'play' || !session || session.is_game_over) {
      return;
    }
    const activeSeat = session.seats[String(session.current_player)];
    if (!activeSeat || activeSeat.type === 'human') return;
    const timer = window.setTimeout(() => {
      void stepModel();
    }, PLAYOUT_SPEED_MS[playoutSpeed]);
    return () => window.clearTimeout(timer);
  }, [playoutRunning, playoutSpeed, view, mode, session, stepModel]);

  const legalActions = useMemo(() => {
    if (!session) return [] as number[];
    const out: number[] = [];
    session.action_mask.forEach((v, i) => {
      if (v > 0.5) out.push(i);
    });
    return out;
  }, [session]);

  const playableHandBySeat = useMemo(() => {
    const p1 = new Set<number>();
    const p2 = new Set<number>();
    if (!session || mode !== 'play') return { p1, p2 };
    const seat = session.seats[String(session.current_player)];
    if (seat && seat.type !== 'human') return { p1, p2 };
    const target = session.current_player === 1 ? p1 : p2;
    for (const a of legalActions) {
      if (a >= 0 && a < 30) target.add(a);
    }
    return { p1, p2 };
  }, [session, mode, legalActions]);

  const ensureZoneSpec = (board: StudioBoardPayload, pid: 1 | 2): StudioZoneSpec => {
    const key = String(pid);
    const existing = board.zones[key];
    if (existing) return existing;
    const uiP = pid === 1 ? session?.state.player1 : session?.state.player2;
    return {
      hand: [...(uiP?.handIds ?? [])],
      trash: [...(uiP?.trashIds ?? [])],
      field: (uiP?.battleArea ?? []).map((perm) => ({
        stack: perm.sources.map((s) => s.cardId),
        is_suspended: perm.isSuspended,
        turn_played: perm.turnPlayed ?? 0,
      })),
      breeding: uiP?.breedingArea ? uiP.breedingArea.sources.map((s) => s.cardId) : [],
    };
  };

  const commitZoneUpdate = (pid: 1 | 2, nextZone: StudioZoneSpec) => {
    if (!session) return;
    const nextBoard: StudioBoardPayload = {
      ...session.board,
      zones: {
        ...session.board.zones,
        [String(pid)]: nextZone,
      },
    };
    void updateBoard(nextBoard);
  };

  const handlePlaceCard = (cardId: string) => {
    if (!session) return;
    const z = { ...ensureZoneSpec(session.board, targetPlayer) };
    const field = [...(z.field ?? [])];

    if (placementTarget === 'field_new') {
      field.push({ stack: [cardId], is_suspended: false, turn_played: 0, linked_cards: [] });
      z.field = field;
    } else if (
      (placementTarget === 'field_stack_top' ||
        placementTarget === 'field_stack_bottom' ||
        placementTarget === 'field_link') &&
      selectedStack &&
      selectedStack.player === targetPlayer
    ) {
      const existing = field[selectedStack.slotIndex];
      if (!existing) return;
      const target: StudioFieldStack = { ...existing, stack: [...existing.stack] };
      if (placementTarget === 'field_stack_top') {
        target.stack = [...target.stack, cardId];
      } else if (placementTarget === 'field_stack_bottom') {
        target.stack = [cardId, ...target.stack];
      } else {
        target.linked_cards = [...(target.linked_cards ?? []), cardId];
      }
      field[selectedStack.slotIndex] = target;
      z.field = field;
    } else if (placementTarget === 'breeding') {
      z.breeding = [...(z.breeding ?? []), cardId];
    } else if (placementTarget === 'hand') {
      z.hand = [...(z.hand ?? []), cardId];
    } else if (placementTarget === 'security') {
      z.security = [cardId, ...(z.security ?? [])];
    } else if (placementTarget === 'trash') {
      z.trash = [...(z.trash ?? []), cardId];
    } else if (placementTarget === 'deck_top') {
      z.deck_top = [cardId, ...(z.deck_top ?? [])];
    } else if (placementTarget === 'egg_deck') {
      z.egg_deck = [cardId, ...(z.egg_deck ?? [])];
    }
    commitZoneUpdate(targetPlayer, z);
  };

  const handleUpdateStack = (pid: 1 | 2, slotIndex: number, next: StudioFieldStack) => {
    if (!session) return;
    const z = { ...ensureZoneSpec(session.board, pid) };
    const field = [...(z.field ?? [])];
    field[slotIndex] = next;
    z.field = field;
    commitZoneUpdate(pid, z);
  };

  const handleDeleteStack = (pid: 1 | 2, slotIndex: number) => {
    if (!session) return;
    const z = { ...ensureZoneSpec(session.board, pid) };
    z.field = (z.field ?? []).filter((_, i) => i !== slotIndex);
    setSelectedStack(null);
    commitZoneUpdate(pid, z);
  };

  const activeSelectedStackObj =
    session && selectedStack
      ? ensureZoneSpec(session.board, selectedStack.player).field?.[selectedStack.slotIndex] ?? null
      : null;

  return (
    <div className="studio-root" data-testid="studio-page">
      <header className="studio-header">
        <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
          <span className="studio-brand">// BOARD STUDIO</span>
          <div className="studio-seg">
            <button
              type="button"
              className={`studio-seg__btn ${view === 'library' ? 'is-active' : ''}`}
              onClick={() => setView('library')}
            >
              LIBRARY
            </button>
            <button
              type="button"
              className={`studio-seg__btn ${view === 'workspace' ? 'is-active' : ''}`}
              disabled={!session}
              onClick={() => setView('workspace')}
            >
              WORKSPACE
            </button>
            <button
              type="button"
              className={`studio-seg__btn ${view === 'replay' ? 'is-active' : ''}`}
              onClick={() => setView('replay')}
            >
              REPLAY STEPPER
            </button>
          </div>
        </div>

        {view === 'workspace' && session && (
          <div style={{ display: 'flex', flexWrap: 'wrap', alignItems: 'center', gap: 10 }}>
            <span
              data-testid="studio-workspace-title"
              style={{ fontWeight: 700, fontSize: 13, color: 'var(--ink-0)' }}
            >
              {puzzleTitle}
            </span>
            <div className="studio-seg">
              <button
                type="button"
                data-testid="studio-mode-compose"
                className={`studio-seg__btn ${mode === 'compose' ? 'is-active' : ''}`}
                onClick={() => setMode('compose')}
              >
                COMPOSE
              </button>
              <button
                type="button"
                data-testid="studio-mode-play"
                className={`studio-seg__btn ${mode === 'play' ? 'is-active' : ''}`}
                onClick={() => setMode('play')}
              >
                PLAY & PLAYOUT
              </button>
            </div>
            <button type="button" className="studio-btn" onClick={toggleFlipped}>
              Flip View
            </button>
            <button
              type="button"
              data-testid="studio-open-save"
              className="studio-btn studio-btn--primary"
              onClick={() => setShowSaveModal(true)}
            >
              Save / Export
            </button>
          </div>
        )}
      </header>

      {error && (
        <div className="studio-banner studio-banner--error">
          <span>{error}</span>
          <button type="button" className="studio-btn studio-btn--sm" onClick={clearError}>×</button>
        </div>
      )}
      {statusMessage && (
        <div className="studio-banner studio-banner--ok">
          <span>{statusMessage}</span>
          <button type="button" className="studio-btn studio-btn--sm" onClick={clearError}>×</button>
        </div>
      )}
      {session?.validation_error && (
        <div className="studio-banner studio-banner--warn">
          Engine validation warning: {session.validation_error}
        </div>
      )}

      <div className="studio-body">
        {view === 'library' && <StudioLibraryView />}
        {view === 'replay' && <ReplayStepperView />}
        {view === 'workspace' && session && (
          <>
            <aside className="studio-rail-left">
              {mode === 'compose' ? (
                <>
                  <div className="studio-panel">
                    <div className="studio-panel__title">Scalar State</div>
                    <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 6, fontSize: 11 }}>
                      <label style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
                        Memory
                        <input
                          type="number"
                          min={-10}
                          max={10}
                          className="studio-input"
                          value={session.board.memory}
                          onChange={(e) =>
                            void updateBoard({ ...session.board, memory: Number(e.target.value) || 0 })
                          }
                        />
                      </label>
                      <label style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
                        Turn
                        <input
                          type="number"
                          min={1}
                          className="studio-input"
                          value={session.board.turn}
                          onChange={(e) =>
                            void updateBoard({ ...session.board, turn: Math.max(1, Number(e.target.value) || 1) })
                          }
                        />
                      </label>
                      <label style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
                        Active Player
                        <select
                          className="studio-select"
                          value={session.board.first_player}
                          onChange={(e) =>
                            void updateBoard({
                              ...session.board,
                              first_player: Number(e.target.value) === 2 ? 2 : 1,
                            })
                          }
                        >
                          <option value={1}>Player 1</option>
                          <option value={2}>Player 2</option>
                        </select>
                      </label>
                      <label style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
                        Phase
                        <select
                          className="studio-select"
                          value={session.board.phase}
                          onChange={(e) => void updateBoard({ ...session.board, phase: e.target.value })}
                        >
                          <option value="Breeding">Breeding</option>
                          <option value="Main">Main</option>
                          <option value="EndTurn">EndTurn</option>
                        </select>
                      </label>
                    </div>
                    <div className="studio-pill-group">
                      {(['hand', 'security', 'trash', 'deck_top', 'egg_deck'] as EditableZoneKind[]).map((z) => (
                        <button
                          key={z}
                          type="button"
                          className="studio-btn studio-btn--sm"
                          onClick={() => setZoneModal({ player: targetPlayer, zone: z })}
                        >
                          P{targetPlayer} {z.replace('_', ' ')}
                        </button>
                      ))}
                    </div>
                  </div>

                  {selectedStack && activeSelectedStackObj && (
                    <StackInspector
                      player={selectedStack.player}
                      slotIndex={selectedStack.slotIndex}
                      stack={activeSelectedStackObj}
                      onUpdate={(next) =>
                        handleUpdateStack(selectedStack.player, selectedStack.slotIndex, next)
                      }
                      onDelete={() =>
                        handleDeleteStack(selectedStack.player, selectedStack.slotIndex)
                      }
                      onClose={() => setSelectedStack(null)}
                    />
                  )}

                  <CardSearchDrawer
                    targetPlayer={targetPlayer}
                    onTargetPlayerChange={setTargetPlayer}
                    placementTarget={placementTarget}
                    onPlacementTargetChange={setPlacementTarget}
                    selectedSlotIndex={selectedStack?.slotIndex ?? null}
                    onPlaceCard={handlePlaceCard}
                  />
                </>
              ) : (
                <>
                  <div className="studio-panel" data-testid="studio-legal-actions" style={{ borderColor: 'var(--accent)' }}>
                    <div className="studio-panel__title" style={{ color: 'var(--accent)' }}>
                      Legal Actions — P{session.current_player} ({(session.legal_actions ?? legalActions).length})
                    </div>
                    <div style={{ display: 'flex', flexDirection: 'column', gap: 4, maxHeight: 240, overflowY: 'auto' }}>
                      {(session.legal_actions && session.legal_actions.length > 0
                        ? session.legal_actions
                        : legalActions.map((aid) => ({ action_id: aid, label: `Action ${aid}` }))
                      ).map((act) => (
                        <button
                          key={act.action_id}
                          type="button"
                          className="studio-card-item"
                          style={{ fontSize: 11, padding: '5px 7px' }}
                          disabled={session.is_game_over}
                          onClick={() => void stepHuman(act.action_id)}
                          title={act.label}
                        >
                          <span style={{ fontFamily: 'var(--font-mono)', fontSize: 10, color: 'var(--accent)' }}>
                            [{act.action_id}]
                          </span>
                          <span style={{ color: 'var(--ink-0)', lineHeight: 1.3 }}>{act.label}</span>
                        </button>
                      ))}
                      {legalActions.length === 0 && (
                        <span style={{ fontSize: 11, color: 'var(--ink-3)' }}>No legal actions</span>
                      )}
                    </div>
                  </div>

                  <div className="studio-panel">
                    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                      <span className="studio-panel__title">
                        Recorded Timeline ({session.timeline.length})
                      </span>
                      <button
                        type="button"
                        className="studio-btn studio-btn--sm studio-btn--signal"
                        onClick={() => void snapshotAsStart()}
                      >
                        Snapshot Start
                      </button>
                    </div>
                    <div style={{ maxHeight: 160, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: 4 }}>
                      {session.timeline.length === 0 && (
                        <div style={{ color: 'var(--ink-3)', fontSize: 11, padding: '6px 0' }}>
                          No moves recorded yet.
                        </div>
                      )}
                      {session.timeline.map((st) => (
                        <div
                          key={st.step_index}
                          className={`studio-row ${selectedStepIndex === st.step_index ? 'is-selected' : ''}`}
                          style={{ cursor: 'pointer', fontSize: 11 }}
                          onClick={() => setSelectedStepIndex(st.step_index)}
                        >
                          <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                            #{st.step_index + 1} P{st.actor}: {st.label}
                          </span>
                          <button
                            type="button"
                            className="studio-btn studio-btn--sm"
                            onClick={(e) => {
                              e.stopPropagation();
                              void rewindTo(st.step_index);
                            }}
                          >
                            Rewind
                          </button>
                        </div>
                      ))}
                    </div>
                    <div style={{ display: 'flex', gap: 4 }}>
                      <button
                        type="button"
                        className="studio-btn studio-btn--sm"
                        style={{ flex: 1 }}
                        onClick={() => void stepHuman(ACTION.PASS)}
                        disabled={!legalActions.includes(ACTION.PASS)}
                      >
                        Pass (62)
                      </button>
                      <button
                        type="button"
                        className="studio-btn studio-btn--sm"
                        onClick={() => void stepHuman(ACTION.HATCH)}
                        disabled={!legalActions.includes(ACTION.HATCH)}
                      >
                        Hatch
                      </button>
                      <button
                        type="button"
                        className="studio-btn studio-btn--sm"
                        onClick={() => void stepHuman(ACTION.MOVE)}
                        disabled={!legalActions.includes(ACTION.MOVE)}
                      >
                        Move
                      </button>
                    </div>
                  </div>

                  <div className="studio-panel">
                    <div className="studio-panel__title">Named Lines ({session.lines.length})</div>
                    <div style={{ display: 'flex', gap: 4 }}>
                      <input
                        className="studio-input"
                        style={{ flex: 1, minWidth: 0 }}
                        placeholder="Line name..."
                        value={lineName}
                        onChange={(e) => setLineName(e.target.value)}
                      />
                      <select
                        className="studio-select"
                        value={lineRole}
                        onChange={(e) => setLineRole(e.target.value as 'target' | 'avoid' | 'alternative')}
                      >
                        <option value="target">target</option>
                        <option value="avoid">avoid</option>
                        <option value="alternative">alt</option>
                      </select>
                      <button
                        type="button"
                        className="studio-btn studio-btn--sm studio-btn--primary"
                        disabled={session.timeline.length === 0}
                        onClick={() => {
                          void saveNamedLine(lineName || `Line ${session.lines.length + 1}`, lineRole);
                          setLineName('');
                        }}
                      >
                        Save
                      </button>
                    </div>
                    <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
                      {session.lines.map((ln) => (
                        <div key={ln.id} className="studio-row" style={{ fontSize: 11 }}>
                          <button
                            type="button"
                            style={{ background: 'none', border: 'none', color: 'var(--ink-0)', cursor: 'pointer', textAlign: 'left', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}
                            onClick={() => void loadNamedLine(ln.id)}
                          >
                            [{ln.role}] {ln.name} ({ln.actions.length})
                          </button>
                          <div style={{ display: 'flex', gap: 4, flexShrink: 0 }}>
                            <button
                              type="button"
                              className="studio-btn studio-btn--sm"
                              title="Promote to exam expectation"
                              onClick={() =>
                                addExpectation(
                                  ln.role === 'avoid'
                                    ? { kind: 'avoid', plan: ln.actions, label: ln.name }
                                    : { kind: 'visits', plan: ln.actions, min_share: 0.1, label: ln.name },
                                )
                              }
                            >
                              +Exp
                            </button>
                            <button
                              type="button"
                              className="studio-btn studio-btn--sm studio-btn--danger"
                              onClick={() => void deleteNamedLine(ln.id)}
                            >
                              ×
                            </button>
                          </div>
                        </div>
                      ))}
                    </div>
                    {expectations.length > 0 && (
                      <div style={{ display: 'flex', flexDirection: 'column', gap: 4, borderTop: '1px solid var(--line-0)', paddingTop: 6 }}>
                        <div className="studio-panel__title">Expectations</div>
                        {expectations.map((ex, i) => (
                          <div key={i} className="studio-row" style={{ fontSize: 11 }}>
                            <span>{ex.kind}: {JSON.stringify(ex.plan)}</span>
                            <button type="button" className="studio-btn studio-btn--sm studio-btn--danger" onClick={() => removeExpectation(i)}>×</button>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>

                  <div className="studio-panel" data-testid="studio-assertions-panel">
                    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                      <span className="studio-panel__title">Engine Assertions ({assertions.length})</span>
                      <button
                        type="button"
                        className="studio-btn studio-btn--sm studio-btn--primary"
                        disabled={assertions.length === 0}
                        onClick={() => void evaluateCurrentAssertions()}
                      >
                        Run
                      </button>
                    </div>
                    <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap' }}>
                      <select
                        className="studio-select"
                        style={{ flex: 1, minWidth: 110 }}
                        value={newAssertKind}
                        onChange={(e) => setNewAssertKind(e.target.value)}
                      >
                        <option value="action_legal">action_legal</option>
                        <option value="memory_equals">memory_equals</option>
                        <option value="security_count">security_count</option>
                        <option value="hand_count">hand_count</option>
                        <option value="field_count">field_count</option>
                        <option value="trash_count">trash_count</option>
                        <option value="stack_top">stack_top</option>
                        <option value="winner_is">winner_is</option>
                      </select>
                      {['security_count', 'hand_count', 'field_count', 'trash_count', 'stack_top'].includes(newAssertKind) && (
                        <select
                          className="studio-select"
                          value={newAssertPlayer}
                          onChange={(e) => setNewAssertPlayer(Number(e.target.value) === 2 ? 2 : 1)}
                        >
                          <option value={1}>P1</option>
                          <option value={2}>P2</option>
                        </select>
                      )}
                      <input
                        className="studio-input"
                        style={{ width: 76 }}
                        placeholder={newAssertKind === 'stack_top' ? 'Card ID' : 'Value'}
                        value={newAssertVal}
                        onChange={(e) => setNewAssertVal(e.target.value)}
                      />
                      <button
                        type="button"
                        className="studio-btn studio-btn--sm"
                        onClick={() => {
                          const v = newAssertVal.trim();
                          if (!v) return;
                          if (newAssertKind === 'action_legal') {
                            addAssertion({ kind: 'action_legal', action_id: Number(v), expected: true });
                          } else if (newAssertKind === 'stack_top') {
                            addAssertion({ kind: 'stack_top', player: newAssertPlayer, field_index: 0, card_id: v.toUpperCase() });
                          } else if (['security_count', 'hand_count', 'field_count', 'trash_count'].includes(newAssertKind)) {
                            addAssertion({ kind: newAssertKind, player: newAssertPlayer, value: Number(v) });
                          } else {
                            addAssertion({ kind: newAssertKind, value: Number(v) });
                          }
                          setNewAssertVal('');
                        }}
                      >
                        + Add
                      </button>
                    </div>
                    <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
                      {assertions.map((a, i) => {
                        const res = assertionResults?.[i];
                        return (
                          <div
                            key={i}
                            className="studio-row"
                            style={{
                              fontSize: 11,
                              borderColor: res ? (res.passed ? 'var(--good)' : 'var(--danger)') : undefined,
                            }}
                          >
                            <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                              {res ? (res.passed ? '✓ ' : '✗ ') : ''}
                              {a.kind}
                              {a.player ? ` P${a.player}` : ''}
                              {a.action_id != null ? ` #${a.action_id}` : ''}
                              {a.card_id ? ` ${a.card_id}` : ''}
                              {a.value != null ? ` = ${a.value}` : ''}
                            </span>
                            <button
                              type="button"
                              className="studio-btn studio-btn--sm studio-btn--danger"
                              onClick={() => removeAssertion(i)}
                            >
                              ×
                            </button>
                          </div>
                        );
                      })}
                      {assertions.length === 0 && (
                        <span style={{ fontSize: 11, color: 'var(--ink-3)' }}>No engine assertions yet</span>
                      )}
                    </div>
                  </div>
                </>
              )}
            </aside>

            <main className="studio-stage">
              <GameBoard
                dualSeat
                flipped={flipped}
                playableHandIndices={playableHandBySeat.p1}
                playableOpponentHandIndices={playableHandBySeat.p2}
                onPlayCard={(idx) => {
                  if (mode === 'play' && session.current_player === 1) void stepHuman(idx);
                }}
                onOpponentPlayCard={(idx) => {
                  if (mode === 'play' && session.current_player === 2) void stepHuman(idx);
                }}
                onSlotClick={(isOpp, slotIdx) => {
                  const pid: 1 | 2 = isOpp ? 2 : 1;
                  if (mode === 'compose') {
                    setTargetPlayer(pid);
                    setSelectedStack({ player: pid, slotIndex: slotIdx });
                  } else if (mode === 'play' && session.pending_selection) {
                    const kind = String(session.pending_selection.kind ?? '');
                    const isSelf = pid === session.current_player;
                    let cand: number | null = null;
                    if (kind === 'OwnField' && isSelf) {
                      cand = 100 + slotIdx;
                    } else if (kind === 'OppField' && !isSelf) {
                      cand = 100 + slotIdx;
                    } else {
                      const primary = isSelf ? 100 + slotIdx : 114 + slotIdx;
                      const fallback = 100 + slotIdx;
                      cand =
                        (session.action_mask[primary] ?? 0) > 0.5
                          ? primary
                          : (session.action_mask[fallback] ?? 0) > 0.5
                            ? fallback
                            : null;
                    }
                    if (cand !== null && (session.action_mask[cand] ?? 0) > 0.5) {
                      void stepHuman(cand);
                    }
                  } else if (mode === 'play' && !session.pending_selection && pid === session.current_player) {
                    // Activate a legal [Main] field/tamer effect on this slot when clicked directly
                    const base = 1000 + slotIdx * 10;
                    const slotEffects = legalActions.filter((aid) => aid >= base && aid < base + 10);
                    if (slotEffects.length === 1 && slotEffects[0] != null) {
                      void stepHuman(slotEffects[0]);
                    }
                  }
                }}
                onSlotInspect={(isOpp, slotIdx) => {
                  const pid: 1 | 2 = isOpp ? 2 : 1;
                  if (mode === 'compose') {
                    setTargetPlayer(pid);
                    setSelectedStack({ player: pid, slotIndex: slotIdx });
                  } else {
                    const p = isOpp ? session.state.player2 : session.state.player1;
                    setInspectedPermanent(p.battleArea[slotIdx] ?? null);
                  }
                }}
                onBreedingInspect={(isOpp) => {
                  const p = isOpp ? session.state.player2 : session.state.player1;
                  setInspectedPermanent(p.breedingArea ?? null);
                }}
                onHandCardInspect={(cid) => setInspectedCardId(cid)}
                onOwnTrashClick={() => mode === 'compose' && setZoneModal({ player: 1, zone: 'trash' })}
                onOpponentTrashClick={() => mode === 'compose' && setZoneModal({ player: 2, zone: 'trash' })}
                onHatch={() => mode === 'play' && void stepHuman(ACTION.HATCH)}
                onMove={() => mode === 'play' && void stepHuman(ACTION.MOVE)}
                canHatch={mode === 'play' && legalActions.includes(ACTION.HATCH)}
                canMove={mode === 'play' && legalActions.includes(ACTION.MOVE)}
              />
              {mode === 'play' && session.pending_selection && (
                <SelectionPanel
                  currentPhase={session.state.currentPhase}
                  pendingSelection={session.pending_selection}
                  actionMask={session.action_mask}
                  handIds={
                    session.current_player === 1
                      ? session.state.player1.handIds
                      : session.state.player2.handIds
                  }
                  securityCount={
                    session.current_player === 1
                      ? session.state.player1.securityCount
                      : session.state.player2.securityCount
                  }
                  opponentSecurityCount={
                    session.current_player === 1
                      ? session.state.player2.securityCount
                      : session.state.player1.securityCount
                  }
                  battleArea={
                    session.current_player === 1
                      ? session.state.player1.battleArea
                      : session.state.player2.battleArea
                  }
                  opponentBattleArea={
                    session.current_player === 1
                      ? session.state.player2.battleArea
                      : session.state.player1.battleArea
                  }
                  trashIds={
                    session.current_player === 1
                      ? session.state.player1.trashIds
                      : session.state.player2.trashIds
                  }
                  opponentTrashIds={
                    session.current_player === 1
                      ? session.state.player2.trashIds
                      : session.state.player1.trashIds
                  }
                  localPlayer={session.current_player}
                  onAction={(aid: number) => void stepHuman(aid)}
                />
              )}
              <CardOverlay
                permanent={inspectedPermanent}
                onClose={() => setInspectedPermanent(null)}
                onInspectCard={(cid) => setInspectedCardId(cid)}
              />
              <CardDetailOverlay
                cardId={inspectedCardId}
                onClose={() => setInspectedCardId(null)}
              />
            </main>

            <aside className="studio-rail-right">
              <AgentInspectorPanel />
            </aside>
          </>
        )}
      </div>

      {zoneModal && session && (
        <ZoneEditorModal
          player={zoneModal.player}
          zone={zoneModal.zone}
          spec={ensureZoneSpec(session.board, zoneModal.player)}
          onUpdate={(next) => commitZoneUpdate(zoneModal.player, next)}
          onClose={() => setZoneModal(null)}
        />
      )}

      {showSaveModal && (
        <PuzzleSaveExportModal onClose={() => setShowSaveModal(false)} />
      )}
    </div>
  );
}
