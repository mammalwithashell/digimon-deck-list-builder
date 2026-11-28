import { useRef, useState } from 'react';
import { Card } from '@/components/shared/Card';
import { formatEffect } from '@/utils/formatCardText';

export interface BoardDecisionRailProps {
  eyebrow: string;
  title: string;
  sourceCardId?: string | null;
  clauseText?: string | null;
  metaText?: string | null;
  tone?: 'default' | 'combat' | 'target';
  onInspectCard?: (cardId: string) => void;
  children: React.ReactNode;
}

/**
 * Aegis-style draggable docked side rail for prompts answered directly on the
 * board or via compact yes/no choices — keeps the playmat visible and clickable.
 */
export function BoardDecisionRail({
  eyebrow,
  title,
  sourceCardId,
  clauseText,
  metaText,
  tone = 'default',
  onInspectCard,
  children,
}: BoardDecisionRailProps) {
  const [offset, setOffset] = useState<{ x: number; y: number }>({ x: 0, y: 0 });
  const dragOrigin = useRef<{ startX: number; startY: number; baseX: number; baseY: number } | null>(
    null,
  );

  const onPointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    dragOrigin.current = {
      startX: e.clientX,
      startY: e.clientY,
      baseX: offset.x,
      baseY: offset.y,
    };
    e.currentTarget.setPointerCapture(e.pointerId);
  };

  const onPointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    if (!dragOrigin.current) return;
    setOffset({
      x: dragOrigin.current.baseX + (e.clientX - dragOrigin.current.startX),
      y: dragOrigin.current.baseY + (e.clientY - dragOrigin.current.startY),
    });
  };

  const onPointerUp = () => {
    dragOrigin.current = null;
  };

  return (
    <section
      data-testid="board-decision-rail"
      role="region"
      aria-label={title}
      className={`hs-decision-rail${tone !== 'default' ? ` hs-decision-rail--${tone}` : ''}`}
      style={
        {
          '--rail-dx': `${offset.x}px`,
          '--rail-dy': `${offset.y}px`,
        } as React.CSSProperties
      }
    >
      <div
        className="hs-decision-rail__header"
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        title="Drag to reposition"
      >
        <div>
          <div className="hs-decision-rail__eyebrow">{eyebrow}</div>
          <h3 className="hs-decision-rail__title">{title}</h3>
        </div>
        <span className="text-[11px] text-[var(--ink-3)]" aria-hidden="true">
          ⋮⋮
        </span>
      </div>

      {(sourceCardId || clauseText || metaText) && (
        <div className="hs-decision-rail__body">
          {sourceCardId && (
            <div
              className="hs-decision-rail__art"
              onClick={() => onInspectCard?.(sourceCardId)}
              onContextMenu={(e) => {
                e.preventDefault();
                onInspectCard?.(sourceCardId);
              }}
              title="Click to inspect card"
            >
              <Card cardId={sourceCardId} size="sm" />
            </div>
          )}
          <div className="hs-decision-rail__clause">
            {clauseText && <div className="card-effect-text">{formatEffect(clauseText)}</div>}
            {metaText && <div className="hs-decision-rail__meta">{metaText}</div>}
          </div>
        </div>
      )}

      <div className="hs-decision-rail__actions">{children}</div>
    </section>
  );
}
