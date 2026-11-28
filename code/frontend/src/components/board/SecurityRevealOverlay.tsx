import '@/components/game/decisions/decisions.css';
import { useState, useEffect, useRef, useCallback } from 'react';
import { useGameStore } from '@/stores/gameStore';
import { useUiStore } from '@/stores/uiStore';
import { Card } from '@/components/shared/Card';
import { scaleDuration } from '@/pacing/pacingConfig';
import type { GameEvent } from '@/types/game';

/** Base display durations scaled by user botSpeed & motion settings. */
const BASE_REVEAL_MS = 1600;
const BASE_BATTLE_MS = 1500;

interface SecurityCheck {
  reveal: GameEvent;
  battle: GameEvent | null;
}

interface SecurityRevealOverlayProps {
  onInspectCard?: (cardId: string) => void;
}

/**
 * Upgraded Security Check & Battle Clash overlay using softened unified
 * design tokens and speed/motion-aware pacing.
 */
export function SecurityRevealOverlay({ onInspectCard }: SecurityRevealOverlayProps = {}) {
  const events = useGameStore((s) => s.events);
  const botSpeed = useUiStore((s) => s.botSpeed);
  const motion = useUiStore((s) => s.motion);
  const [queue, setQueue] = useState<SecurityCheck[]>([]);
  const [phase, setPhase] = useState<'reveal' | 'battle'>('reveal');
  const lastProcessedSeq = useRef(-1);

  useEffect(() => {
    const newReveals = events.filter(
      (e) => e.type === 'security_reveal' && e.seq > lastProcessedSeq.current,
    );
    if (newReveals.length === 0) return;

    lastProcessedSeq.current = newReveals[newReveals.length - 1]!.seq;
    if (botSpeed === 'instant' || motion === 'off') {
      return;
    }

    const newChecks: SecurityCheck[] = newReveals.map((reveal) => {
      const battle =
        events.find(
          (e) =>
            e.type === 'security_battle' &&
            e.seq > reveal.seq &&
            e.meta.defender_name === reveal.meta.revealed_card_name,
        ) ?? null;
      return { reveal, battle };
    });

    setQueue((prev) => [...prev, ...newChecks]);
    setPhase('reveal');
  }, [events, botSpeed, motion]);

  const advance = useCallback(() => {
    if (phase === 'reveal' && queue[0]?.battle) {
      setPhase('battle');
    } else {
      setQueue((prev) => prev.slice(1));
      setPhase('reveal');
    }
  }, [phase, queue]);

  useEffect(() => {
    if (queue.length === 0) return;
    const baseMs = phase === 'battle' ? BASE_BATTLE_MS : BASE_REVEAL_MS;
    const duration = Math.max(320, scaleDuration(baseMs, botSpeed, motion));
    const timer = setTimeout(advance, duration);
    return () => clearTimeout(timer);
  }, [queue, phase, advance, botSpeed, motion]);

  const current = queue[0];
  if (!current) return null;

  const cardId =
    (current.reveal.meta.revealed_card_id as string) || current.reveal.source_card_id;
  const cardName = (current.reveal.meta.revealed_card_name as string) || 'Unknown';
  const securityRemaining = current.reveal.meta.security_remaining as number;
  const securityEffect = current.reveal.meta.security_effect_text as string;
  const cardType = current.reveal.meta.card_type as string;
  const baseDp = current.reveal.meta.base_dp as number | null;

  if (!cardId) return null;

  return (
    <div
      data-testid="security-reveal-overlay"
      className="hs-sec-backdrop"
      onClick={advance}
    >
      {phase === 'reveal' ? (
        <RevealPhase
          cardId={cardId}
          cardName={cardName}
          cardType={cardType}
          baseDp={baseDp}
          securityRemaining={securityRemaining}
          securityEffect={securityEffect}
          checkCount={queue.length}
          onInspectCard={onInspectCard}
        />
      ) : current.battle ? (
        <BattlePhase
          battle={current.battle}
          securityCardId={cardId}
          securityCardName={cardName}
          onInspectCard={onInspectCard}
        />
      ) : null}
    </div>
  );
}

function RevealPhase({
  cardId,
  cardName,
  cardType,
  baseDp,
  securityRemaining,
  securityEffect,
  checkCount,
  onInspectCard,
}: {
  cardId: string;
  cardName: string;
  cardType: string;
  baseDp: number | null;
  securityRemaining: number;
  securityEffect: string;
  checkCount: number;
  onInspectCard?: (cardId: string) => void;
}) {
  return (
    <div className="hs-sec-panel animate-security-reveal">
      <div className="hs-sec-eyebrow">Security Check</div>
      <div
        onContextMenu={(e) => {
          e.preventDefault();
          e.stopPropagation();
          onInspectCard?.(cardId);
        }}
      >
        <Card cardId={cardId} cardName={cardName} size="xl" />
      </div>
      <div className="text-sm font-bold text-[var(--ink-0)]">{cardName}</div>
      {cardType === 'Digimon' && baseDp != null && (
        <span className="hs-zone-pill">DP {baseDp.toLocaleString()}</span>
      )}
      {securityEffect ? (
        <div className="hs-sec-effect-box">
          <div className="hs-sec-eyebrow mb-1">[Security] Effect</div>
          <div>{securityEffect}</div>
        </div>
      ) : (
        <div className="text-[11px] text-[var(--ink-2)] italic">
          No security effect — goes to trash after check
        </div>
      )}
      <div className="text-[11px] text-[var(--ink-2)]">
        {securityRemaining} security remaining
        {checkCount > 1 && ` · ${checkCount} checks queued`}
        {' · click to continue'}
      </div>
    </div>
  );
}

function BattlePhase({
  battle,
  securityCardId,
  securityCardName,
  onInspectCard,
}: {
  battle: GameEvent;
  securityCardId: string;
  securityCardName: string;
  onInspectCard?: (cardId: string) => void;
}) {
  const attackerDp = battle.meta.attacker_dp as number;
  const defenderDp = battle.meta.defender_dp as number;
  const attackerName = (battle.meta.attacker_name as string) || 'Attacker';
  const result = battle.meta.result as string;
  const jamming = Boolean(battle.meta.jamming);
  const attackerCardId = battle.source_card_id;

  const attackerWins = result === 'attacker_survives' && !jamming;
  const defenderWins = result === 'attacker_deleted';

  return (
    <div className="hs-sec-panel animate-security-reveal">
      <div className="hs-sec-eyebrow">Security Battle Clash</div>
      <div className="flex items-center gap-6">
        <div className="flex flex-col items-center gap-2">
          {attackerCardId && (
            <div
              onContextMenu={(e) => {
                e.preventDefault();
                e.stopPropagation();
                onInspectCard?.(attackerCardId);
              }}
            >
              <Card cardId={attackerCardId} cardName={attackerName} size="lg" />
            </div>
          )}
          <div className="text-xs font-semibold text-[var(--ink-1)]">{attackerName}</div>
          <div
            className="text-base font-extrabold"
            style={{
              fontFamily: 'var(--font-mono)',
              color: attackerWins
                ? 'var(--good)'
                : defenderWins
                  ? 'var(--danger)'
                  : 'var(--warn)',
            }}
          >
            DP {attackerDp.toLocaleString()}
          </div>
        </div>

        <div className="flex flex-col items-center gap-1.5">
          <div
            className="text-2xl font-black text-[var(--ink-2)]"
            style={{ fontFamily: 'var(--font-display)' }}
          >
            VS
          </div>
          {jamming && <span className="hs-zone-pill">Jamming</span>}
        </div>

        <div className="flex flex-col items-center gap-2">
          <div
            onContextMenu={(e) => {
              e.preventDefault();
              e.stopPropagation();
              onInspectCard?.(securityCardId);
            }}
          >
            <Card cardId={securityCardId} cardName={securityCardName} size="lg" />
          </div>
          <div className="text-xs font-semibold text-[var(--ink-1)]">{securityCardName}</div>
          <div
            className="text-base font-extrabold"
            style={{
              fontFamily: 'var(--font-mono)',
              color: defenderWins
                ? 'var(--good)'
                : attackerWins
                  ? 'var(--danger)'
                  : 'var(--warn)',
            }}
          >
            DP {defenderDp.toLocaleString()}
          </div>
        </div>
      </div>

      <div
        className={`hs-sec-verdict ${
          defenderWins
            ? 'hs-sec-verdict--lose'
            : jamming
              ? 'hs-sec-verdict--jamming'
              : 'hs-sec-verdict--win'
        }`}
      >
        {defenderWins
          ? 'Attacker Deleted'
          : jamming
            ? 'Jamming — Attacker Survives'
            : 'Attacker Wins'}
      </div>

      <div className="text-[11px] text-[var(--ink-2)]">click to continue</div>
    </div>
  );
}
