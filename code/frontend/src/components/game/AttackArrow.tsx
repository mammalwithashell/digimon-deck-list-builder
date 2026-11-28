import { useEffect, useState, useCallback } from 'react';
import { ATTACK_TARGET_SECURITY } from '@/utils/constants';
import { useUiStore } from '@/stores/uiStore';
import type { PendingAttack } from '@/types/game';

interface AttackArrowProps {
  pendingAttack: PendingAttack | null;
  selectedAttacker: number | null;
  /** True when the current attacker belongs to the opponent (top half). */
  attackerIsOpponent?: boolean;
  /** Changes whenever permanents enter/leave/move so arrow coordinates re-measure after FLIP. */
  boardLayoutKey?: string;
  /** Ref to the container element the arrow is drawn relative to */
  containerRef: React.RefObject<HTMLDivElement | null>;
}

interface Point {
  x: number;
  y: number;
}

/**
 * Convert an element's viewport bounding box into the container's *unscaled*
 * CSS coordinate space. Crucial inside `<CanvasScaler>` where an ancestor has
 * `transform: scale(s)`: `getBoundingClientRect()` returns scaled viewport
 * pixels, whereas `<svg className="absolute inset-0 w-full h-full">` draws in
 * unscaled internal canvas pixels (`1920x1080`). Dividing by `rect/client`
 * restores exact 1:1 alignment at every window size & aspect ratio.
 */
function getElementCenter(el: Element | null, container: HTMLElement): Point | null {
  if (!el) return null;
  const rect = el.getBoundingClientRect();
  const containerRect = container.getBoundingClientRect();
  const scaleX =
    container.clientWidth > 0 && containerRect.width > 0
      ? containerRect.width / container.clientWidth
      : 1;
  const scaleY =
    container.clientHeight > 0 && containerRect.height > 0
      ? containerRect.height / container.clientHeight
      : 1;
  return {
    x: (rect.left + rect.width / 2 - containerRect.left) / scaleX,
    y: (rect.top + rect.height / 2 - containerRect.top) / scaleY,
  };
}

function getSlotCenter(
  slotIndex: number,
  isOpponent: boolean,
  container: HTMLElement,
): Point | null {
  const prefix = isOpponent ? 'opponent' : 'player';
  const el = container.querySelector(`[data-slot-id="${prefix}-${slotIndex}"]`);
  return getElementCenter(el, container);
}

function getSecurityCenter(isOpponent: boolean, container: HTMLElement): Point | null {
  const selector = isOpponent
    ? '[data-testid="security-attack-target"], [data-testid="opponent-security-row"]'
    : '[data-testid="own-security-row"]';
  return getElementCenter(container.querySelector(selector), container);
}

export function AttackArrow({
  pendingAttack,
  selectedAttacker,
  attackerIsOpponent = false,
  boardLayoutKey = '',
  containerRef,
}: AttackArrowProps) {
  const canvasScale = useUiStore((s) => s.canvasScale);
  const [line, setLine] = useState<{ from: Point; to: Point } | null>(null);

  const updateLine = useCallback(() => {
    const container = containerRef.current;
    if (!container || !pendingAttack) {
      setLine(null);
      return;
    }

    if (pendingAttack.attackerSlot < 0) {
      setLine(null);
      return;
    }

    const from = getSlotCenter(pendingAttack.attackerSlot, attackerIsOpponent, container);
    const defenderIsOpponent = !attackerIsOpponent;
    const to =
      pendingAttack.targetSlot === ATTACK_TARGET_SECURITY
        ? getSecurityCenter(defenderIsOpponent, container)
        : pendingAttack.targetSlot >= 0
          ? getSlotCenter(pendingAttack.targetSlot, defenderIsOpponent, container)
          : null;

    if (from && to) {
      setLine({ from, to });
      return;
    }

    setLine(null);
  }, [pendingAttack, attackerIsOpponent, containerRef]);

  useEffect(() => {
    updateLine();
    const raf = window.requestAnimationFrame(updateLine);
    // Re-measure once FLIP transitions settle (~260ms) so redirects lock dead-center
    const settleTimer = window.setTimeout(updateLine, 260);
    window.addEventListener('resize', updateLine);
    return () => {
      window.cancelAnimationFrame(raf);
      window.clearTimeout(settleTimer);
      window.removeEventListener('resize', updateLine);
    };
  }, [updateLine, boardLayoutKey, canvasScale]);

  if (!line && selectedAttacker === null) return null;
  if (!line) return null;

  const dx = line.to.x - line.from.x;
  const dy = line.to.y - line.from.y;
  const dist = Math.hypot(dx, dy) || 1;
  // Subtle perpendicular curve so long cross-board redirects stay crisp and readable
  const bend = Math.min(48, dist * 0.14) * (dx >= 0 ? -1 : 1);
  const ctrlX = (line.from.x + line.to.x) / 2 - (dy / dist) * bend;
  const ctrlY = (line.from.y + line.to.y) / 2 + (dx / dist) * bend;

  // Tangent angle at the curve endpoint (t = 1) is the vector from control point -> tip
  const endAngle = Math.atan2(line.to.y - ctrlY, line.to.x - ctrlX);
  const headLen = 14;
  const headAngle = Math.PI / 6;
  const tipX = line.to.x;
  const tipY = line.to.y;
  const left = {
    x: tipX - headLen * Math.cos(endAngle - headAngle),
    y: tipY - headLen * Math.sin(endAngle - headAngle),
  };
  const right = {
    x: tipX - headLen * Math.cos(endAngle + headAngle),
    y: tipY - headLen * Math.sin(endAngle + headAngle),
  };

  return (
    <svg
      className="absolute inset-0 w-full h-full pointer-events-none z-30"
      data-testid="attack-arrow"
      data-attacker-side={attackerIsOpponent ? 'opponent' : 'player'}
      data-target-slot={pendingAttack?.targetSlot}
    >
      <defs>
        <filter id="arrow-glow">
          <feGaussianBlur stdDeviation="3" result="blur" />
          <feMerge>
            <feMergeNode in="blur" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
      </defs>

      {/* Origin anchor dot */}
      <circle
        cx={line.from.x}
        cy={line.from.y}
        r={5}
        fill="#ef4444"
        filter="url(#arrow-glow)"
        opacity={0.95}
      />

      {/* Curved attack / redirect trajectory */}
      <path
        d={`M ${line.from.x} ${line.from.y} Q ${ctrlX} ${ctrlY} ${tipX} ${tipY}`}
        fill="none"
        stroke="#ef4444"
        strokeWidth={3.5}
        strokeLinecap="round"
        filter="url(#arrow-glow)"
        opacity={0.92}
      />

      {/* Target reticle ring */}
      <circle
        cx={tipX}
        cy={tipY}
        r={11}
        fill="none"
        stroke="#ef4444"
        strokeWidth={2}
        opacity={0.75}
      />

      {/* Arrowhead */}
      <polygon
        points={`${tipX},${tipY} ${left.x},${left.y} ${right.x},${right.y}`}
        fill="#ef4444"
        filter="url(#arrow-glow)"
        opacity={0.95}
      />
    </svg>
  );
}
