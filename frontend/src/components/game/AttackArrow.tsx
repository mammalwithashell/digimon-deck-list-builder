import { useEffect, useState, useCallback } from 'react';
import type { PendingAttack } from '@/types/game';

interface AttackArrowProps {
  pendingAttack: PendingAttack | null;
  selectedAttacker: number | null;
  /** Ref to the container element the arrow is drawn relative to */
  containerRef: React.RefObject<HTMLDivElement | null>;
}

interface Point {
  x: number;
  y: number;
}

function getSlotCenter(slotIndex: number, isOpponent: boolean, container: HTMLElement): Point | null {
  // Droppable slots use id like "field-slot-{i}" but those are data attributes on @dnd-kit.
  // We'll search for the slot by querying the DOM structure.
  const prefix = isOpponent ? 'opponent' : 'player';
  const el = container.querySelector(`[data-slot-id="${prefix}-${slotIndex}"]`);
  if (!el) return null;

  const rect = el.getBoundingClientRect();
  const containerRect = container.getBoundingClientRect();
  return {
    x: rect.left + rect.width / 2 - containerRect.left,
    y: rect.top + rect.height / 2 - containerRect.top,
  };
}

export function AttackArrow({ pendingAttack, selectedAttacker, containerRef }: AttackArrowProps) {
  const [line, setLine] = useState<{ from: Point; to: Point } | null>(null);

  const updateLine = useCallback(() => {
    const container = containerRef.current;
    if (!container) {
      setLine(null);
      return;
    }

    if (pendingAttack) {
      const from = getSlotCenter(pendingAttack.attackerSlot, false, container);
      const to =
        pendingAttack.targetSlot >= 0
          ? getSlotCenter(pendingAttack.targetSlot, true, container)
          : null;

      if (from && to) {
        setLine({ from, to });
        return;
      }
    }

    setLine(null);
  }, [pendingAttack, containerRef]);

  useEffect(() => {
    updateLine();
    // Recalculate on resize
    window.addEventListener('resize', updateLine);
    return () => window.removeEventListener('resize', updateLine);
  }, [updateLine]);

  // Also show a pulsing highlight on selected attacker (no arrow yet, just glow)
  if (!line && selectedAttacker === null) return null;
  if (!line) return null;

  const dx = line.to.x - line.from.x;
  const dy = line.to.y - line.from.y;
  const len = Math.sqrt(dx * dx + dy * dy);
  const angle = Math.atan2(dy, dx);

  // Arrowhead points
  const headLen = 12;
  const headAngle = Math.PI / 6;
  const tipX = line.to.x;
  const tipY = line.to.y;
  const left = {
    x: tipX - headLen * Math.cos(angle - headAngle),
    y: tipY - headLen * Math.sin(angle - headAngle),
  };
  const right = {
    x: tipX - headLen * Math.cos(angle + headAngle),
    y: tipY - headLen * Math.sin(angle + headAngle),
  };

  return (
    <svg
      className="absolute inset-0 w-full h-full pointer-events-none z-30"
      data-testid="attack-arrow"
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

      {/* Main line */}
      <line
        x1={line.from.x}
        y1={line.from.y}
        x2={line.to.x}
        y2={line.to.y}
        stroke="#ef4444"
        strokeWidth={3}
        strokeLinecap="round"
        filter="url(#arrow-glow)"
        opacity={0.9}
      />

      {/* Arrowhead */}
      <polygon
        points={`${tipX},${tipY} ${left.x},${left.y} ${right.x},${right.y}`}
        fill="#ef4444"
        filter="url(#arrow-glow)"
        opacity={0.9}
      />
    </svg>
  );
}

