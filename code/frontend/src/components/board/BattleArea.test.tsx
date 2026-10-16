import '@testing-library/jest-dom/vitest';
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { DndContext } from '@dnd-kit/core';
import { BattleArea } from './BattleArea';
import type { PermanentInfo } from '@/types/game';

const perm = { topCardId: 'BT1-010', topCardName: 'Agumon', dp: 2000, level: 3, isSuspended: false,
  sourceCount: 0, keywords: [], securityAttackModifier: 0, linkedCardIds: [], sources: [],
  mainEffectText: '', inheritedEffects: [], modifiers: [], turnPlayed: 1, colors: [0] } as unknown as PermanentInfo;

function renderArea(props: Partial<Parameters<typeof BattleArea>[0]> = {}) {
  return render(
    <DndContext>
      <BattleArea permanents={[perm, perm]} isOpponent={false} {...props} />
    </DndContext>,
  );
}

describe('BattleArea', () => {
  it('draws only filled slots at rest', () => {
    renderArea();
    expect(screen.getByTestId('field-slot-p1-0')).toBeInTheDocument();
    expect(screen.getByTestId('field-slot-p1-1')).toBeInTheDocument();
    expect(screen.queryByTestId('field-slot-p1-2')).not.toBeInTheDocument();
  });

  it('adds the first empty slot as a landing spot while a playable card is dragged', () => {
    renderArea({ isDraggingHandCard: true, canPlayDragged: true });
    expect(screen.getByTestId('field-slot-p1-2')).toHaveTextContent('Play here');
    expect(screen.queryByTestId('field-slot-p1-3')).not.toBeInTheDocument();
  });

  it('shows no landing spot when the dragged card cannot be played', () => {
    renderArea({ isDraggingHandCard: true, canPlayDragged: false });
    expect(screen.queryByTestId('field-slot-p1-2')).not.toBeInTheDocument();
  });
});
