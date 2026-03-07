import { useState, useEffect, useRef, useCallback } from 'react';
import { useGameStore } from '@/stores/gameStore';
import { Card } from '@/components/shared/Card';
import type { GameEvent } from '@/types/game';

/** How long each reveal phase displays before auto-advancing */
const REVEAL_DURATION_MS = 2000;
const BATTLE_DURATION_MS = 1800;

interface SecurityCheck {
  reveal: GameEvent;
  battle: GameEvent | null;
}

/**
 * Full-screen overlay that shows security cards as they're revealed.
 * Queues multiple reveals (e.g. Security Attack +1) and shows them sequentially.
 * Each check shows: card reveal → DP comparison (if Digimon) → next.
 */
export function SecurityRevealOverlay() {
  const events = useGameStore((s) => s.events);
  const [queue, setQueue] = useState<SecurityCheck[]>([]);
  const [phase, setPhase] = useState<'reveal' | 'battle'>('reveal');
  const lastProcessedSeq = useRef(-1);

  // Build queue from new events
  useEffect(() => {
    const newReveals = events.filter(
      (e) => e.type === 'security_reveal' && e.seq > lastProcessedSeq.current
    );
    if (newReveals.length === 0) return;

    const newChecks: SecurityCheck[] = newReveals.map((reveal) => {
      // Find matching battle event (same source card, seq > reveal.seq)
      const battle = events.find(
        (e) =>
          e.type === 'security_battle' &&
          e.seq > reveal.seq &&
          e.meta.defender_name === reveal.meta.revealed_card_name
      ) ?? null;
      return { reveal, battle };
    });

    lastProcessedSeq.current = newReveals[newReveals.length - 1]!.seq;
    setQueue((prev) => [...prev, ...newChecks]);
    setPhase('reveal');
  }, [events]);

  // Auto-advance timer
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
    const duration = phase === 'battle' ? BATTLE_DURATION_MS : REVEAL_DURATION_MS;
    const timer = setTimeout(advance, duration);
    return () => clearTimeout(timer);
  }, [queue, phase, advance]);

  const current = queue[0];
  if (!current) return null;

  const cardId = (current.reveal.meta.revealed_card_id as string) || current.reveal.source_card_id;
  const cardName = (current.reveal.meta.revealed_card_name as string) || 'Unknown';
  const securityRemaining = current.reveal.meta.security_remaining as number;
  const securityEffect = current.reveal.meta.security_effect_text as string;
  const cardType = current.reveal.meta.card_type as string;
  const baseDp = current.reveal.meta.base_dp as number | null;

  if (!cardId) return null;

  return (
    <div
      className="absolute inset-0 z-50 flex items-center justify-center bg-black/60 animate-fade-in cursor-pointer"
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
        />
      ) : current.battle ? (
        <BattlePhase battle={current.battle} securityCardId={cardId} securityCardName={cardName} />
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
}: {
  cardId: string;
  cardName: string;
  cardType: string;
  baseDp: number | null;
  securityRemaining: number;
  securityEffect: string;
  checkCount: number;
}) {
  return (
    <div className="flex flex-col items-center gap-3 animate-security-reveal">
      <div className="text-amber-400 text-sm font-bold tracking-wider uppercase">
        Security Check!
      </div>
      <Card cardId={cardId} cardName={cardName} size="xl" />
      <div className="text-gray-300 text-xs font-medium">{cardName}</div>
      {cardType === 'Digimon' && baseDp != null && (
        <div className="text-blue-300 text-[11px]">DP {baseDp.toLocaleString()}</div>
      )}
      {securityEffect ? (
        <div className="max-w-xs text-center px-3 py-1.5 bg-amber-900/40 border border-amber-700/50 rounded text-amber-200 text-[11px] leading-snug">
          {securityEffect}
        </div>
      ) : (
        <div className="text-gray-600 text-[10px] italic">No security effect</div>
      )}
      <div className="text-gray-500 text-[10px]">
        {securityRemaining} security remaining
        {checkCount > 1 && ` · ${checkCount} checks queued`}
        {' — click to continue'}
      </div>
    </div>
  );
}

function BattlePhase({
  battle,
  securityCardId,
  securityCardName,
}: {
  battle: GameEvent;
  securityCardId: string;
  securityCardName: string;
}) {
  const attackerDp = battle.meta.attacker_dp as number;
  const defenderDp = battle.meta.defender_dp as number;
  const attackerName = (battle.meta.attacker_name as string) || 'Attacker';
  const result = battle.meta.result as string;
  const jamming = battle.meta.jamming as boolean;
  const attackerCardId = battle.source_card_id;

  const attackerWins = result === 'attacker_survives' && !jamming;
  const defenderWins = result === 'attacker_deleted';

  return (
    <div className="flex flex-col items-center gap-4 animate-security-reveal">
      <div className="text-amber-400 text-sm font-bold tracking-wider uppercase">
        Security Battle!
      </div>
      <div className="flex items-center gap-6">
        {/* Attacker side */}
        <div className="flex flex-col items-center gap-2">
          {attackerCardId && <Card cardId={attackerCardId} cardName={attackerName} size="lg" />}
          <div className="text-xs text-gray-300">{attackerName}</div>
          <div
            className={`text-lg font-bold ${attackerWins ? 'text-green-400' : defenderWins ? 'text-red-400' : 'text-yellow-300'}`}
          >
            DP {attackerDp.toLocaleString()}
          </div>
        </div>

        {/* VS divider */}
        <div className="flex flex-col items-center gap-1">
          <div className="text-2xl font-black text-gray-400">VS</div>
          {jamming && (
            <div className="px-2 py-0.5 bg-purple-900/60 border border-purple-500/50 rounded text-purple-300 text-[10px] font-bold uppercase">
              Jamming!
            </div>
          )}
        </div>

        {/* Defender (security) side */}
        <div className="flex flex-col items-center gap-2">
          <Card cardId={securityCardId} cardName={securityCardName} size="lg" />
          <div className="text-xs text-gray-300">{securityCardName}</div>
          <div
            className={`text-lg font-bold ${defenderWins ? 'text-green-400' : attackerWins ? 'text-red-400' : 'text-yellow-300'}`}
          >
            DP {defenderDp.toLocaleString()}
          </div>
        </div>
      </div>

      {/* Result banner */}
      <div
        className={`px-4 py-1.5 rounded-full text-sm font-bold uppercase tracking-wide ${
          defenderWins
            ? 'bg-red-900/50 text-red-300 border border-red-500/40'
            : jamming
              ? 'bg-purple-900/50 text-purple-300 border border-purple-500/40'
              : 'bg-green-900/50 text-green-300 border border-green-500/40'
        }`}
      >
        {defenderWins
          ? 'Attacker Deleted!'
          : jamming
            ? 'Jamming — Attacker Survives!'
            : 'Attacker Wins!'}
      </div>

      <div className="text-gray-500 text-[10px]">click to continue</div>
    </div>
  );
}
