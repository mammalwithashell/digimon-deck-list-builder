import { useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Window } from '@/design/components/Window';
import { InBetweenShell } from '@/features/play/InBetweenShell';
import {
  getPlayFormat,
  loadPlayFormats,
  type OpponentMode,
  type PlayFormat,
} from '@/features/play/formatCatalog';
import { usePlayFlowStore } from '@/features/play/playFlowStore';
import './ModeSelectPage.css';

const OPPONENTS: Array<{ id: OpponentMode; name: string; sub: string; meta: string }> = [
  { id: 'quick', name: 'QUICK MATCH', sub: 'Auto-paired ladder', meta: 'MATCHMAKING' },
  { id: 'room', name: 'ROOM MATCH', sub: 'Private code and friends', meta: 'PRIVATE CODE' },
  { id: 'bot', name: 'BOT MATCH', sub: 'CPU practice', meta: 'LOCAL ENGINE' },
  { id: 'ai_starter', name: 'AI STARTER DECK', sub: 'Pick a starter, face the AI', meta: 'AI OPPONENT' },
];

export function ModeSelectPage() {
  const navigate = useNavigate();
  const [formats, setFormats] = useState<PlayFormat[]>([]);
  const { formatId, opponentMode, selectFormat, selectOpponentMode, clearLaunchState } =
    usePlayFlowStore();
  const fallback = getPlayFormat(formatId);
  const visibleFormats = (formats.length > 0 ? formats : [fallback]).filter(
    (format) => format.id !== 'starter',
  );
  const selected = useMemo(
    () => visibleFormats.find((format) => format.id === formatId) ?? fallback,
    [fallback, formatId, visibleFormats],
  );

  useEffect(() => {
    clearLaunchState();
    loadPlayFormats().then(setFormats).catch(() => setFormats([]));
  }, [clearLaunchState]);

  return (
    <InBetweenShell
      title="CHOOSE FORMAT"
      stepLabel="01"
      crumbs={[{ label: 'HOME', href: '/' }, { label: 'PLAY' }]}
      rightSlot={<span>STEP 1 OF 3</span>}
    >
      <main className="mode-select-main">
        <header className="mode-select-header">
          <div className="mode-select-kicker">// SELECT A RULESET TO ENTER THE AMPHITHEATER</div>
          <h1>
            CHOOSE YOUR
            <br />
            <em>FORMAT.</em>
          </h1>
          <p>{visibleFormats.length} RULESETS - DIFFERENT BANLISTS - DIFFERENT DECK SHAPES - ONE THEATER</p>
        </header>

        <Window title="OPPONENT" className="mode-window">
          <section className="mode-opponent-strip" aria-label="Opponent">
            {OPPONENTS.map((opponent) => (
              <button
                key={opponent.id}
                type="button"
                className={opponentMode === opponent.id ? 'on' : ''}
                onClick={() => selectOpponentMode(opponent.id)}
              >
                <span className="name">{opponent.name}</span>
                <span className="sub">{opponent.sub}</span>
                <span className="meta">{opponent.meta}</span>
              </button>
            ))}
          </section>
        </Window>

        {opponentMode !== 'ai_starter' && (
          <Window title="FORMAT" className="mode-window">
            <section className="mode-grid" aria-label="Formats">
              {visibleFormats.map((format, index) => (
                <button
                  key={format.id}
                  type="button"
                  aria-label={format.name}
                  className={`mode-card ${format.id === formatId ? 'selected' : ''}`}
                  onClick={() => selectFormat(format.id)}
                  disabled={!format.enabled}
                >
                  <span className="num">
                    {String(index + 1).padStart(2, '0')} / {String(visibleFormats.length).padStart(2, '0')}
                  </span>
                  <span className="tag">{format.enabled ? '// READY' : '// LOCKED'}</span>
                  <span className="sub">{format.tagline}</span>
                  <span className="name">{format.name}</span>
                  <span className="desc">{format.enabled ? format.description : format.disabledReason}</span>
                  <span className="stats">
                    <b>{format.deckLabel}</b>
                    <i>POPULATION {format.populationPct}%</i>
                  </span>
                </button>
              ))}
            </section>
          </Window>
        )}

        <div className="mode-action-bar">
          <span>
            {opponentMode === 'ai_starter' ? 'STARTER DECKS' : selected.name} /{' '}
            {opponentMode.toUpperCase()}
          </span>
          <button
            type="button"
            onClick={() =>
              navigate(
                opponentMode === 'room'
                  ? '/play/room'
                  : opponentMode === 'ai_starter'
                    ? '/play/ai-starter'
                    : '/play/deck',
              )
            }
            disabled={opponentMode !== 'ai_starter' && !selected.enabled}
          >
            ENTER FORMAT
          </button>
        </div>
      </main>
    </InBetweenShell>
  );
}
