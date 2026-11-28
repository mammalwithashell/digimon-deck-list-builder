import type { StudioZoneSpec } from '@/api/studioTransport';
import { useStudioStore } from '@/stores/studioStore';

export type EditableZoneKind = 'hand' | 'security' | 'trash' | 'deck_top' | 'egg_deck';

interface ZoneEditorModalProps {
  player: 1 | 2;
  zone: EditableZoneKind;
  spec: StudioZoneSpec;
  onUpdate: (next: StudioZoneSpec) => void;
  onClose: () => void;
}

export function ZoneEditorModal({
  player,
  zone,
  spec,
  onUpdate,
  onClose,
}: ZoneEditorModalProps) {
  const cardCatalog = useStudioStore((s) => s.cardCatalog);
  const nameOf = (cid: string) => cardCatalog.find((c) => c.card_id === cid)?.name ?? cid;

  const rawList: Array<{ card_id: string; face_up?: boolean }> = (() => {
    if (zone === 'security') {
      const fu = new Set(spec.face_up_security ?? []);
      return (spec.security ?? []).map((item, idx) =>
        typeof item === 'string'
          ? { card_id: item, face_up: fu.has(idx) }
          : { card_id: item.card_id, face_up: Boolean(item.face_up) || fu.has(idx) },
      );
    }
    const arr = (spec[zone] as string[] | undefined) ?? [];
    return arr.map((card_id) => ({ card_id }));
  })();

  const commit = (next: Array<{ card_id: string; face_up?: boolean }>) => {
    if (zone === 'security') {
      const faceUpIndices = next
        .map((c, i) => (c.face_up ? i : -1))
        .filter((i) => i >= 0);
      onUpdate({
        ...spec,
        security: next.map((c) => c.card_id),
        face_up_security: faceUpIndices,
      });
    } else {
      onUpdate({ ...spec, [zone]: next.map((c) => c.card_id) });
    }
  };

  const move = (idx: number, dir: -1 | 1) => {
    const target = idx + dir;
    if (target < 0 || target >= rawList.length) return;
    const copy = [...rawList];
    const [item] = copy.splice(idx, 1);
    if (item !== undefined) {
      copy.splice(target, 0, item);
      commit(copy);
    }
  };

  const remove = (idx: number) => commit(rawList.filter((_, i) => i !== idx));

  const toggleFaceUp = (idx: number) => {
    commit(rawList.map((c, i) => (i === idx ? { ...c, face_up: !c.face_up } : c)));
  };

  return (
    <div className="studio-modal-backdrop">
      <div className="studio-modal" data-testid="studio-zone-editor">
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <span className="studio-panel__title">
            P{player} — {zone.replace('_', ' ').toUpperCase()} ({rawList.length})
          </span>
          <button type="button" className="studio-btn studio-btn--sm" onClick={onClose}>
            ×
          </button>
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: 4, maxHeight: 360, overflowY: 'auto' }}>
          {rawList.length === 0 && (
            <div style={{ padding: 20, textAlign: 'center', color: 'var(--ink-3)', fontSize: 12 }}>
              Zone is empty
            </div>
          )}
          {rawList.map((item, i) => (
            <div key={`${item.card_id}-${i}`} className="studio-row">
              <span style={{ fontSize: 12 }}>
                {i + 1}. <strong>{nameOf(item.card_id)}</strong>{' '}
                <span style={{ fontFamily: 'var(--font-mono)', fontSize: 11, color: 'var(--ink-2)' }}>
                  {item.card_id}
                </span>
              </span>
              <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                {zone === 'security' && (
                  <button
                    type="button"
                    className={`studio-btn studio-btn--sm ${item.face_up ? 'studio-btn--signal' : ''}`}
                    onClick={() => toggleFaceUp(i)}
                  >
                    {item.face_up ? 'Face-Up' : 'Face-Down'}
                  </button>
                )}
                <button type="button" className="studio-btn studio-btn--sm" onClick={() => move(i, -1)}>↑</button>
                <button type="button" className="studio-btn studio-btn--sm" onClick={() => move(i, 1)}>↓</button>
                <button type="button" className="studio-btn studio-btn--sm studio-btn--danger" onClick={() => remove(i)}>×</button>
              </div>
            </div>
          ))}
        </div>

        <div style={{ display: 'flex', justifyContent: 'space-between', marginTop: 4 }}>
          <button type="button" className="studio-btn studio-btn--danger" onClick={() => commit([])}>
            Clear Zone
          </button>
          <button type="button" className="studio-btn studio-btn--primary" onClick={onClose}>
            Done
          </button>
        </div>
      </div>
    </div>
  );
}
