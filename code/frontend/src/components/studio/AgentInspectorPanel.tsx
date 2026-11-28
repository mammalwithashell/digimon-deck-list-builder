import { useState } from 'react';
import type { SeatConfig, SeatControllerType, StepEvalSnapshot } from '@/api/studioTransport';
import { useStudioStore, type PlayoutSpeed } from '@/stores/studioStore';

export function AgentInspectorPanel() {
  const {
    session,
    models,
    playoutRunning,
    playoutSpeed,
    selectedStepIndex,
    liveEval,
    inspectBusy,
    updateSeats,
    setPlayoutRunning,
    setPlayoutSpeed,
    stepModel,
    rewindTo,
    inspectCurrentPosition,
  } = useStudioStore();

  const [inspectModelId, setInspectModelId] = useState<string>(models[0]?.id ?? '');
  const [inspectSims, setInspectSims] = useState<number>(0);
  const [inspectWorlds, setInspectWorlds] = useState<number>(2);
  const [inspectSeed, setInspectSeed] = useState<number>(0);

  if (!session) return null;

  const activeSeatKey = String(session.current_player);
  const activeSeat = session.seats[activeSeatKey];
  const isModelTurn = activeSeat && activeSeat.type !== 'human';

  const selectedStep =
    selectedStepIndex != null ? session.timeline[selectedStepIndex] ?? null : null;
  const displayedEval: StepEvalSnapshot | null = selectedStep?.eval ?? liveEval;

  const handleSeatChange = (pid: '1' | '2', patch: Partial<SeatConfig>) => {
    const cur = session.seats[pid] ?? {
      type: 'human',
      model_id: models[0]?.id ?? null,
      simulations: 0,
      worlds: 1,
      regime: 'open_list',
      seed: 0,
    };
    void updateSeats({
      ...session.seats,
      [pid]: { ...cur, ...patch },
    });
  };

  const handleInspectNow = () => {
    const mid = inspectModelId || models[0]?.id || '';
    if (!mid) return;
    void inspectCurrentPosition({
      model_id: mid,
      simulations: inspectSims,
      worlds: inspectWorlds,
      regime: 'open_list',
      seed: inspectSeed,
    });
  };

  return (
    <div className="studio-panel" data-testid="studio-agent-inspector">
      <div className="studio-panel__title">Pilot Models & Step Controller</div>

      {/* Seat Assignments */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
        {(['1', '2'] as const).map((pid) => {
          const seat = session.seats[pid] ?? {
            type: 'human',
            simulations: 0,
            worlds: 1,
            regime: 'open_list',
            seed: 0,
          };
          const isTurn = session.current_player === Number(pid);
          return (
            <div
              key={pid}
              className="studio-panel"
              style={{
                background: 'var(--surface-sunken)',
                borderColor: isTurn ? 'var(--accent)' : 'var(--line-0)',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 6 }}>
                <span style={{ fontWeight: 700, fontSize: 12, color: pid === '1' ? 'var(--player-2)' : 'var(--opp-2)' }}>
                  Player {pid} {isTurn ? '●' : ''}
                </span>
                <select
                  className="studio-select"
                  value={seat.type}
                  onChange={(e) =>
                    handleSeatChange(pid, {
                      type: e.target.value as SeatControllerType,
                      model_id: seat.model_id ?? models[0]?.id ?? null,
                    })
                  }
                  aria-label={`Player ${pid} controller`}
                >
                  <option value="human">Human</option>
                  <option value="model">Pilot Model (.onnx)</option>
                  <option value="greedy">Greedy</option>
                  <option value="random">Random</option>
                </select>
              </div>
              {seat.type === 'model' && (
                <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
                  <select
                    className="studio-select"
                    value={seat.model_id ?? ''}
                    onChange={(e) => handleSeatChange(pid, { model_id: e.target.value })}
                  >
                    {models.length === 0 && <option value="">No local .onnx models found</option>}
                    {models.map((m) => (
                      <option key={m.id} value={m.id}>
                        {m.id}
                      </option>
                    ))}
                  </select>
                  <div style={{ display: 'flex', gap: 8, fontSize: 11 }}>
                    <label style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                      Sims:
                      <input
                        type="number"
                        min={0}
                        max={2048}
                        className="studio-input"
                        style={{ width: 62 }}
                        value={seat.simulations}
                        onChange={(e) =>
                          handleSeatChange(pid, { simulations: Math.max(0, Number(e.target.value) || 0) })
                        }
                      />
                    </label>
                    <label style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                      Worlds:
                      <input
                        type="number"
                        min={1}
                        max={32}
                        className="studio-input"
                        style={{ width: 48 }}
                        value={seat.worlds}
                        onChange={(e) =>
                          handleSeatChange(pid, { worlds: Math.max(1, Number(e.target.value) || 1) })
                        }
                      />
                    </label>
                  </div>
                </div>
              )}
            </div>
          );
        })}
      </div>

      {/* Step Controller */}
      <div className="studio-panel" style={{ background: 'var(--surface-sunken)' }}>
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 5 }}>
          <button
            type="button"
            data-testid="studio-play-pause"
            className={`studio-btn studio-btn--sm ${playoutRunning ? 'studio-btn--signal' : 'studio-btn--primary'}`}
            disabled={session.is_game_over || (!isModelTurn && !playoutRunning)}
            onClick={() => setPlayoutRunning(!playoutRunning)}
          >
            {playoutRunning ? '⏸ Pause' : '▶ Play'}
          </button>
          <button
            type="button"
            data-testid="studio-step-model"
            className="studio-btn studio-btn--sm"
            disabled={session.is_game_over || !isModelTurn || inspectBusy}
            onClick={() => void stepModel()}
          >
            {inspectBusy ? 'Thinking…' : 'Step Next →'}
          </button>
          <button
            type="button"
            className="studio-btn studio-btn--sm"
            disabled={session.timeline.length === 0}
            onClick={() => void rewindTo(Math.max(0, session.timeline.length - 1))}
          >
            ← Back
          </button>
          <button
            type="button"
            className="studio-btn studio-btn--sm"
            disabled={session.timeline.length === 0}
            onClick={() => void rewindTo(0)}
          >
            Reset
          </button>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 11, color: 'var(--ink-2)' }}>
          <span>Speed:</span>
          <div className="studio-seg">
            {(['slow', 'normal', 'fast'] as PlayoutSpeed[]).map((sp) => (
              <button
                key={sp}
                type="button"
                className={`studio-seg__btn ${playoutSpeed === sp ? 'is-active' : ''}`}
                style={{ padding: '2px 7px' }}
                onClick={() => setPlayoutSpeed(sp)}
              >
                {sp}
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* Ad-hoc Single-Position Inspection */}
      <div className="studio-panel" style={{ background: 'var(--surface-sunken)' }}>
        <div className="studio-panel__title">Inspect Current Board</div>
        <select
          className="studio-select"
          value={inspectModelId || models[0]?.id || ''}
          onChange={(e) => setInspectModelId(e.target.value)}
        >
          {models.length === 0 && <option value="">No local .onnx models</option>}
          {models.map((m) => (
            <option key={m.id} value={m.id}>
              {m.id}
            </option>
          ))}
        </select>
        <div style={{ display: 'flex', gap: 6, fontSize: 11 }}>
          <label style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
            Sims:
            <input
              type="number"
              min={0}
              className="studio-input"
              style={{ width: 54 }}
              value={inspectSims}
              onChange={(e) => setInspectSims(Math.max(0, Number(e.target.value) || 0))}
            />
          </label>
          <label style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
            Worlds:
            <input
              type="number"
              min={1}
              className="studio-input"
              style={{ width: 44 }}
              value={inspectWorlds}
              onChange={(e) => setInspectWorlds(Math.max(1, Number(e.target.value) || 1))}
            />
          </label>
          <label style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
            Seed:
            <input
              type="number"
              className="studio-input"
              style={{ width: 44 }}
              value={inspectSeed}
              onChange={(e) => setInspectSeed(Number(e.target.value) || 0)}
            />
          </label>
        </div>
        <button
          type="button"
          data-testid="studio-inspect-now"
          className="studio-btn studio-btn--primary"
          disabled={inspectBusy || models.length === 0}
          onClick={handleInspectNow}
        >
          {inspectBusy ? 'Evaluating…' : 'Inspect Policy & Search'}
        </button>
      </div>

      {/* Valuation Readout */}
      {displayedEval && (
        <div className="studio-panel" style={{ borderColor: 'var(--line-strong)' }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <span className="studio-panel__title" style={{ color: 'var(--accent)' }}>
              {selectedStep?.eval ? `Step #${selectedStep.step_index} Eval` : 'Live Board Eval'} (P{displayedEval.viewer})
            </span>
            <span className="studio-chip">{displayedEval.mode}</span>
          </div>

          <div className="studio-metric-grid">
            <div className="studio-metric">
              <div className="studio-metric__label">POLICY VALUE</div>
              <div className="studio-metric__val">
                {displayedEval.policy_value >= 0 ? '+' : ''}
                {displayedEval.policy_value.toFixed(3)}
              </div>
            </div>
            <div className="studio-metric">
              <div className="studio-metric__label">
                SEARCH ROOT {displayedEval.simulations_run > 0 ? `(${displayedEval.simulations_run})` : ''}
              </div>
              <div className="studio-metric__val">
                {displayedEval.search_value != null
                  ? `${displayedEval.search_value >= 0 ? '+' : ''}${displayedEval.search_value.toFixed(3)}`
                  : '—'}
              </div>
            </div>
          </div>

          {displayedEval.root_plans.length > 0 && (
            <>
              <div className="studio-panel__title">Search Plans (Visits / Q / Raw Prior)</div>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
                {displayedEval.root_plans.slice(0, 6).map((p, idx) => (
                  <div
                    key={idx}
                    className={`studio-row ${p.actions[0] === displayedEval.chosen_action_id ? 'is-selected' : ''}`}
                    style={{ fontSize: 11 }}
                  >
                    <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }} title={p.labels.join(' → ')}>
                      {p.labels.join(' → ')}
                    </span>
                    <span style={{ fontFamily: 'var(--font-mono)', fontSize: 10, color: 'var(--ink-1)', flexShrink: 0 }}>
                      {(p.share * 100).toFixed(0)}% ({p.visits}) · Q {p.q_value >= 0 ? '+' : ''}
                      {p.q_value.toFixed(2)} · P {(p.prior_prob * 100).toFixed(0)}%
                    </span>
                  </div>
                ))}
              </div>
            </>
          )}

          <div className="studio-panel__title">Policy Head Top Actions</div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            {displayedEval.policy_actions.slice(0, 6).map((a) => (
              <div key={a.action_id} className="studio-row" style={{ fontSize: 11 }}>
                <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }} title={a.label}>
                  [{a.action_id}] {a.label}
                </span>
                <span style={{ fontFamily: 'var(--font-mono)', color: 'var(--accent)', flexShrink: 0 }}>
                  {(a.prob * 100).toFixed(1)}%
                </span>
              </div>
            ))}
          </div>

          {displayedEval.expectation_results.length > 0 && (
            <>
              <div className="studio-panel__title">Expectation Verdicts</div>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
                {displayedEval.expectation_results.map((r, i) => (
                  <div
                    key={i}
                    className="studio-row"
                    style={{
                      fontSize: 11,
                      borderColor: r.passed ? 'var(--good)' : 'var(--danger)',
                      color: r.passed ? 'var(--good)' : 'var(--danger)',
                    }}
                  >
                    <span>
                      {r.passed ? '✓' : '✗'} {r.kind} {JSON.stringify(r.plan)}
                    </span>
                    <span style={{ fontFamily: 'var(--font-mono)', fontSize: 10 }}>{r.detail}</span>
                  </div>
                ))}
              </div>
            </>
          )}
        </div>
      )}
    </div>
  );
}
