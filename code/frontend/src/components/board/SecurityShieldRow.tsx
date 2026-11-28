import { useEffect, useMemo, useRef, useState } from 'react';
import { useDroppable } from '@dnd-kit/core';
import { Card } from '@/components/shared/Card';
import { ATTACK_TARGET_SECURITY } from '@/utils/constants';

export interface SecurityShieldRowProps {
  count: number;
  isOpponent: boolean;
  /** Public card IDs (null for face-down security, string for face-up security). */
  securityIds?: (string | null)[];
  /** Per-slot face-up flags emitted by the engine (`securityFaceUp`). */
  securityFaceUp?: boolean[];
  /** True when the player has selected or is dragging an attacker that can swing into opponent security. */
  isAttackTarget?: boolean;
  onAttackSecurity?: () => void;
  onInspectCard?: (cardId: string) => void;
}

const MAX_VISIBLE_SHIELDS = 6;

interface FaceUpEntry {
  index: number;
  cardId: string;
}

/**
 * Security Shield dock placed cleanly in the right resource lane beside Deck & Trash.
 * Renders overlapping miniature card backs (using the player's selected sleeve or
 * standard back for opponent) and renders face-up security cards directly as miniature
 * interactive cards with an 👁 indicator and [Security] badge so players can see &
 * inspect them immediately.
 */
export function SecurityShieldRow({
  count,
  isOpponent,
  securityIds = [],
  securityFaceUp = [],
  isAttackTarget = false,
  onAttackSecurity,
  onInspectCard,
}: SecurityShieldRowProps) {
  const prevCount = useRef(count);
  const [breaking, setBreaking] = useState(false);
  const [faceUpOpen, setFaceUpOpen] = useState(false);

  const { isOver, setNodeRef } = useDroppable({
    id: isOpponent ? 'enemy-security' : 'own-security',
    data: { type: 'enemy-security', slotIndex: ATTACK_TARGET_SECURITY },
    disabled: !isOpponent || !isAttackTarget,
  });

  useEffect(() => {
    if (count < prevCount.current) {
      setBreaking(true);
      const timer = setTimeout(() => setBreaking(false), 620);
      prevCount.current = count;
      return () => clearTimeout(timer);
    }
    prevCount.current = count;
  }, [count]);

  const faceUpEntries = useMemo<FaceUpEntry[]>(() => {
    const out: FaceUpEntry[] = [];
    for (let i = 0; i < count; i++) {
      const cid = securityIds[i];
      if (cid) {
        out.push({ index: i, cardId: cid });
      }
    }
    return out;
  }, [count, securityIds]);

  useEffect(() => {
    if (faceUpEntries.length === 0) setFaceUpOpen(false);
  }, [faceUpEntries.length]);

  const visibleCount = Math.min(count, MAX_VISIBLE_SHIELDS);
  const overflow = Math.max(0, count - MAX_VISIBLE_SHIELDS);
  const clickable = isOpponent && isAttackTarget && Boolean(onAttackSecurity);
  const targetCue = isOpponent && isAttackTarget;

  const handleEyeClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (faceUpEntries.length === 1 && onInspectCard) {
      onInspectCard(faceUpEntries[0]!.cardId);
      return;
    }
    setFaceUpOpen((v) => !v);
  };

  const inner = (
    <>
      <div className="hs-shields__header">
        <span className="hs-shields__title">SECURITY</span>
        <span
          className="hs-shields__count"
          aria-label={`${isOpponent ? 'Opponent' : 'Your'} security: ${count}`}
        >
          {count}
        </span>
      </div>

      <div
        className={`hs-shields__row${targetCue ? ' is-target-mat' : ''}`}
        data-testid={isOpponent ? 'opp-security-stack' : 'own-security-stack'}
      >
        {visibleCount > 0 ? (
          Array.from({ length: visibleCount }).map((_, idx) => {
            const cid = securityIds[idx] ?? null;
            const isFaceUp = Boolean(securityFaceUp[idx] || cid);
            const zIndex = visibleCount - idx;
            const isTop = idx === 0;
            const slotClass = `hs-shield-card${isTop ? ' is-top' : ''}${
              breaking && isTop ? ' is-breaking' : ''
            }`;

            if (isFaceUp && cid) {
              return (
                <button
                  key={`${idx}-${cid}`}
                  type="button"
                  data-testid={`security-faceup-card-${idx}`}
                  className={`${slotClass} hs-shield-card--faceup`}
                  style={{ zIndex }}
                  title={`Face-up security #${idx + 1}: ${cid} [Security] — Click to inspect`}
                  aria-label={`Inspect face-up security #${idx + 1}: ${cid}`}
                  onClick={(e) => {
                    if (clickable) return;
                    e.stopPropagation();
                    onInspectCard?.(cid);
                  }}
                  onContextMenu={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    onInspectCard?.(cid);
                  }}
                >
                  <Card
                    cardId={cid}
                    size="sm"
                    className="hs-shield-card__media"
                  />
                  <span className="hs-shield-card__eye" aria-hidden="true">
                    👁
                  </span>
                  <span className="hs-shield-card__badge">[Security]</span>
                </button>
              );
            }

            return (
              <span
                key={`back-${idx}`}
                data-testid={`security-facedown-card-${idx}`}
                className={`${slotClass} hs-shield-card--back${isFaceUp ? ' is-face-up' : ''}`}
                style={{ zIndex }}
                aria-hidden="true"
              >
                <Card
                  cardId={`security-back-${idx}`}
                  size="sm"
                  faceDown
                  useDefaultSleeve={isOpponent}
                  className="hs-shield-card__media pointer-events-none"
                />
                {isFaceUp && (
                  <span className="hs-shield-card__eye" aria-hidden="true">
                    👁
                  </span>
                )}
              </span>
            );
          })
        ) : (
          <span className={`hs-shields__empty${targetCue ? ' is-direct-target' : ''}`}>
            {targetCue ? 'Direct Attack!' : 'Broken (0)'}
          </span>
        )}
        {overflow > 0 && <span className="hs-shields__overflow">+{overflow}</span>}
      </div>

      {faceUpEntries.length > 0 && (
        <button
          type="button"
          data-testid={isOpponent ? 'opp-security-eye' : 'own-security-eye'}
          className="hs-shields__eye-btn"
          onClick={handleEyeClick}
          title={
            faceUpEntries.length === 1
              ? `Inspect face-up security (${faceUpEntries[0]!.cardId})`
              : `View ${faceUpEntries.length} face-up security cards`
          }
        >
          <span aria-hidden="true">👁</span>
          <span>
            {faceUpEntries.length === 1
              ? `Face-Up: ${faceUpEntries[0]!.cardId}`
              : `${faceUpEntries.length} Face-Up`}
          </span>
        </button>
      )}

      {targetCue && (
        <span className="hs-shields__attack-cta" data-testid="security-attack-cta">
          {isOver ? 'Drop to Attack!' : count > 0 ? 'Attack Security' : 'Attack Player'}
        </span>
      )}
    </>
  );

  const baseClass = `relative hs-shields ${isOpponent ? 'hs-shields--opp' : 'hs-shields--you'}${
    targetCue ? ' is-attack-target' : ''
  }${isOver && targetCue ? ' is-drop-over' : ''}`;

  return (
    <div
      ref={setNodeRef}
      className={baseClass}
      data-testid={
        clickable
          ? 'security-attack-target'
          : isOpponent
            ? 'opponent-security-row'
            : 'own-security-row'
      }
      role={clickable ? 'button' : undefined}
      tabIndex={clickable ? 0 : undefined}
      onClick={clickable ? onAttackSecurity : undefined}
      onKeyDown={
        clickable
          ? (e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                onAttackSecurity?.();
              }
            }
          : undefined
      }
    >
      {inner}

      {faceUpOpen && faceUpEntries.length > 0 && (
        <div
          className="hs-shields__faceup-popover"
          data-testid="faceup-security-popover"
          onClick={(e) => e.stopPropagation()}
        >
          <div className="flex items-center justify-between gap-2">
            <span className="hs-shields__title">FACE-UP SECURITY</span>
            <button
              type="button"
              onClick={() => setFaceUpOpen(false)}
              className="text-xs text-[var(--ink-2)] hover:text-[var(--ink-0)]"
            >
              ✕
            </button>
          </div>
          <div className="hs-shields__faceup-cards">
            {faceUpEntries.map((entry) => (
              <button
                key={`${entry.index}-${entry.cardId}`}
                type="button"
                className="flex flex-col items-center gap-1 cursor-pointer hover:scale-105 transition-transform"
                onClick={() => {
                  onInspectCard?.(entry.cardId);
                  setFaceUpOpen(false);
                }}
              >
                <Card cardId={entry.cardId} size="sm" />
                <span className="hs-zone-pill">#{entry.index + 1}</span>
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
