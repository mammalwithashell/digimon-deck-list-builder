import type { StudioFieldStack } from '@/api/studioTransport';
import { useStudioStore } from '@/stores/studioStore';

interface StackInspectorProps {
  player: 1 | 2;
  slotIndex: number;
  stack: StudioFieldStack;
  onUpdate: (next: StudioFieldStack) => void;
  onDelete: () => void;
  onClose: () => void;
}

export function StackInspector({
  player,
  slotIndex,
  stack,
  onUpdate,
  onDelete,
  onClose,
}: StackInspectorProps) {
  const cardCatalog = useStudioStore((s) => s.cardCatalog);
  const nameOf = (cid: string) => cardCatalog.find((c) => c.card_id === cid)?.name ?? cid;

  const moveCard = (idx: number, dir: -1 | 1) => {
    const nextIdx = idx + dir;
    if (nextIdx < 0 || nextIdx >= stack.stack.length) return;
    const copy = [...stack.stack];
    const [item] = copy.splice(idx, 1);
    if (item !== undefined) {
      copy.splice(nextIdx, 0, item);
      onUpdate({ ...stack, stack: copy });
    }
  };

  const removeCard = (idx: number) => {
    const copy = stack.stack.filter((_, i) => i !== idx);
    if (copy.length === 0) {
      onDelete();
    } else {
      onUpdate({ ...stack, stack: copy });
    }
  };

  const removeLink = (idx: number) => {
    const nextLinks = (stack.linked_cards ?? []).filter((_, i) => i !== idx);
    onUpdate({ ...stack, linked_cards: nextLinks });
  };

  return (
    <div
      className="studio-panel"
      style={{ borderColor: 'var(--accent)' }}
      data-testid="studio-stack-inspector"
    >
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <span className="studio-panel__title" style={{ color: 'var(--accent)' }}>
          P{player} Battle Slot #{slotIndex}
        </span>
        <button type="button" className="studio-btn studio-btn--sm" onClick={onClose}>
          ×
        </button>
      </div>

      <div style={{ display: 'flex', alignItems: 'center', gap: 12, fontSize: 12 }}>
        <label style={{ display: 'flex', alignItems: 'center', gap: 5 }}>
          <input
            type="checkbox"
            checked={stack.is_suspended}
            onChange={(e) => onUpdate({ ...stack, is_suspended: e.target.checked })}
          />
          Suspended
        </label>
        <label style={{ display: 'flex', alignItems: 'center', gap: 5 }}>
          Turn Played:
          <input
            type="number"
            min={0}
            className="studio-input"
            style={{ width: 56 }}
            value={stack.turn_played}
            onChange={(e) => onUpdate({ ...stack, turn_played: Math.max(0, Number(e.target.value) || 0) })}
          />
        </label>
      </div>

      <div className="studio-panel__title">Stack (Bottom → Top)</div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
        {stack.stack.map((cid, i) => (
          <div key={`${cid}-${i}`} className="studio-row">
            <span style={{ fontSize: 11, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
              {i === stack.stack.length - 1 ? '★ ' : `${i + 1}. `}
              <strong>{nameOf(cid)}</strong>{' '}
              <span style={{ fontFamily: 'var(--font-mono)', color: 'var(--ink-2)' }}>{cid}</span>
            </span>
            <div style={{ display: 'flex', gap: 3 }}>
              <button type="button" className="studio-btn studio-btn--sm" onClick={() => moveCard(i, -1)} title="Move down">↓</button>
              <button type="button" className="studio-btn studio-btn--sm" onClick={() => moveCard(i, 1)} title="Move up">↑</button>
              <button type="button" className="studio-btn studio-btn--sm studio-btn--danger" onClick={() => removeCard(i)}>×</button>
            </div>
          </div>
        ))}
      </div>

      {(stack.linked_cards?.length ?? 0) > 0 && (
        <>
          <div className="studio-panel__title">Linked Cards</div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            {(stack.linked_cards ?? []).map((cid, i) => (
              <div key={`${cid}-link-${i}`} className="studio-row">
                <span style={{ fontSize: 11 }}>
                  🔗 <strong>{nameOf(cid)}</strong>{' '}
                  <span style={{ fontFamily: 'var(--font-mono)', color: 'var(--ink-2)' }}>{cid}</span>
                </span>
                <button type="button" className="studio-btn studio-btn--sm studio-btn--danger" onClick={() => removeLink(i)}>×</button>
              </div>
            ))}
          </div>
        </>
      )}

      <button type="button" className="studio-btn studio-btn--danger" onClick={onDelete}>
        Remove Permanent
      </button>
    </div>
  );
}
