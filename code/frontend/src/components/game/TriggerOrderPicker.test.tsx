import '@testing-library/jest-dom/vitest';
import { fireEvent, render, screen, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { TriggerOrderPicker } from './TriggerOrderPicker';
import type { TriggerChoice } from '@/types/game';

const choices: TriggerChoice[] = ['a', 'b', 'c'].map((id, index) => ({
  triggerId: id, actionId: 30 + index, cardId: `TEST-${id}`, cardName: `Card ${id}`, effectText: `Effect ${id}`,
  timing: 'WhenAttacking', sourceLabel: `Battle area ${index + 1}`, isOptional: index === 1, isInherited: false, activationPrompt: false,
}));
function setup() {
  const onSubmit = vi.fn();
  render(<TriggerOrderPicker choices={choices} validIndices={[30, 31, 32]} actionMask={new Array(2192).fill(1)} onSubmit={onSubmit} />);
  return onSubmit;
}

describe('trigger resolution picker', () => {
  it('submits a partial click order and optional activation preset only after confirmation', () => {
    const submit = setup();
    fireEvent.click(screen.getByRole('button', { name: 'Order Card b: Effect b' }));
    fireEvent.click(screen.getByRole('button', { name: 'Order Card a: Effect a' }));
    fireEvent.click(within(screen.getByRole('group', { name: 'Activate Card b' })).getByText('No'));
    expect(submit).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'Resolve in this order' }));
    expect(submit).toHaveBeenCalledWith({ order: ['b', 'a'], offered: ['a', 'b', 'c'], optionalAnswers: { b: false } });
  });
  it('reorders by keyboard and submits the full visible order', () => {
    const submit = setup();
    fireEvent.keyDown(screen.getByRole('button', { name: 'Reorder item 1' }), { key: 'ArrowDown' });
    fireEvent.click(screen.getByRole('button', { name: 'Resolve in this order' }));
    expect(submit.mock.calls[0]![0].order).toEqual(['b', 'a', 'c']);
  });
  it('reorders by dragging handles', () => {
    const submit = setup();
    fireEvent.dragStart(screen.getByRole('button', { name: 'Reorder item 3' }));
    fireEvent.drop(screen.getAllByRole('listitem')[0]!);
    fireEvent.click(screen.getByRole('button', { name: 'Resolve in this order' }));
    expect(submit.mock.calls[0]![0].order).toEqual(['c', 'a', 'b']);
  });
  it('leaves body-level optional choices interactive when no activation gate is advertised', () => {
    render(<TriggerOrderPicker choices={[{ ...choices[1]!, activationPresetAvailable: false }]} validIndices={[31]} actionMask={new Array(2192).fill(1)} onSubmit={vi.fn()} />);
    expect(screen.queryByRole('group', { name: 'Activate Card b' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'No to all optional effects' })).not.toBeInTheDocument();
  });

});
