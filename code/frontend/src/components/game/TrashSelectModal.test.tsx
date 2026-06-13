import '@testing-library/jest-dom/vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { TrashSelectModal } from './TrashSelectModal';
import { SELECTION } from '@/utils/constants';
import type { PendingSelection } from '@/types/game';

const T = SELECTION.TRASH_START; // engine trash base (1150)

function mask(...legal: number[]) {
  const m = new Array(2192).fill(0);
  for (const i of legal) m[i] = 1;
  return m;
}

function baseProps(over: Partial<Parameters<typeof TrashSelectModal>[0]> = {}) {
  return {
    pendingSelection: null,
    actionMask: mask(),
    ownTrashIds: [],
    opponentTrashIds: [],
    localPlayer: 1,
    onAction: vi.fn(),
    ...over,
  } satisfies Parameters<typeof TrashSelectModal>[0];
}

function singleSel(over: Partial<PendingSelection> = {}): PendingSelection {
  return {
    phase: 0,
    selectingPlayer: 1,
    validIndices: [T, T + 1],
    isOptional: false,
    prompt: 'Play 1 from your trash',
    kind: 'Trash',
    ...over,
  } as PendingSelection;
}

function multiSel(kind: string, validIndices: number[], over: Partial<PendingSelection> = {}): PendingSelection {
  return {
    phase: 0,
    selectingPlayer: 1,
    validIndices,
    isOptional: false,
    prompt: 'Return up to N from your trash',
    kind,
    ...over,
  } as PendingSelection;
}

describe('TrashSelectModal — single select', () => {
  it('dispatches the engine action id for the clicked card', () => {
    const onAction = vi.fn();
    render(
      <TrashSelectModal
        {...baseProps({
          pendingSelection: singleSel(),
          actionMask: mask(T, T + 1),
          ownTrashIds: ['ST1-02', 'ST1-03'],
          onAction,
        })}
      />,
    );
    fireEvent.click(screen.getByTitle('ST1-03'));
    expect(onAction).toHaveBeenCalledWith(T + 1);
  });

  it("renders the OPPONENT's trash when zoneOwner is the opponent", () => {
    const onAction = vi.fn();
    render(
      <TrashSelectModal
        {...baseProps({
          pendingSelection: singleSel({ zoneOwner: 2 }),
          actionMask: mask(T, T + 1),
          ownTrashIds: ['ST1-02', 'ST1-03'],
          opponentTrashIds: ['EX11-012', 'BT21-029'],
          onAction,
        })}
      />,
    );
    expect(screen.getByTitle('EX11-012')).toBeInTheDocument();
    expect(screen.queryByTitle('ST1-02')).not.toBeInTheDocument();
    fireEvent.click(screen.getByTitle('EX11-012'));
    expect(onAction).toHaveBeenCalledWith(T);
  });

  it('shows a decline control for an optional selection', () => {
    const onAction = vi.fn();
    render(
      <TrashSelectModal
        {...baseProps({
          pendingSelection: singleSel({ isOptional: true }),
          actionMask: mask(T, SELECTION.DECLINE),
          ownTrashIds: ['ST1-02'],
          onAction,
        })}
      />,
    );
    fireEvent.click(screen.getByTestId('trash-select-decline'));
    expect(onAction).toHaveBeenCalledWith(SELECTION.DECLINE);
  });
});

describe('TrashSelectModal — capped multi (deferred toggle)', () => {
  const kind = 'CountCappedMultiSelect { min: 1, max: 3, picked: 0, distinct: false }';

  it('toggles selection on/off and gates Done on the minimum, then dispatches picks + PASS', async () => {
    const onAction = vi.fn().mockResolvedValue(undefined);
    render(
      <TrashSelectModal
        {...baseProps({
          pendingSelection: multiSel(kind, [T, T + 1, T + 2, T + 3]),
          actionMask: mask(T, T + 1, T + 2, T + 3, SELECTION.DECLINE),
          ownTrashIds: ['A', 'B', 'C', 'D'],
          onAction,
        })}
      />,
    );
    const done = screen.getByTestId('trash-select-done');
    expect(done).toBeDisabled(); // 0 < min 1

    fireEvent.click(screen.getByTitle('A')); // pick A
    fireEvent.click(screen.getByTitle('B')); // pick B
    expect(screen.getByTestId('trash-select-counter')).toHaveTextContent('2 / 3');
    expect(done).toBeEnabled();

    fireEvent.click(screen.getByTitle('A')); // deselect A
    expect(screen.getByTestId('trash-select-counter')).toHaveTextContent('1 / 3');

    fireEvent.click(done);
    await waitFor(() => expect(onAction).toHaveBeenCalledWith(SELECTION.DECLINE));
    // Order: the single remaining pick (B = T+1), then the stop/PASS.
    expect(onAction.mock.calls.map((c) => c[0])).toEqual([T + 1, SELECTION.DECLINE]);
  });

  it('omits the trailing PASS when exactly max are selected', async () => {
    const onAction = vi.fn().mockResolvedValue(undefined);
    const k = 'CountCappedMultiSelect { min: 1, max: 2, picked: 0, distinct: false }';
    render(
      <TrashSelectModal
        {...baseProps({
          pendingSelection: multiSel(k, [T, T + 1, T + 2]),
          actionMask: mask(T, T + 1, T + 2, SELECTION.DECLINE),
          ownTrashIds: ['A', 'B', 'C'],
          onAction,
        })}
      />,
    );
    fireEvent.click(screen.getByTitle('A'));
    fireEvent.click(screen.getByTitle('B'));
    fireEvent.click(screen.getByTestId('trash-select-done'));
    await waitFor(() => expect(onAction).toHaveBeenCalledTimes(2));
    expect(onAction.mock.calls.map((c) => c[0])).toEqual([T, T + 1]); // no PASS
  });

  it('caps additions at max', () => {
    const k = 'CountCappedMultiSelect { min: 1, max: 2, picked: 0, distinct: false }';
    render(
      <TrashSelectModal
        {...baseProps({
          pendingSelection: multiSel(k, [T, T + 1, T + 2]),
          actionMask: mask(T, T + 1, T + 2),
          ownTrashIds: ['A', 'B', 'C'],
        })}
      />,
    );
    fireEvent.click(screen.getByTitle('A'));
    fireEvent.click(screen.getByTitle('B'));
    fireEvent.click(screen.getByTitle('C')); // over cap — ignored
    expect(screen.getByTestId('trash-select-counter')).toHaveTextContent('2 / 2');
  });
});

describe('TrashSelectModal — capped multi (distinct, immediate commit)', () => {
  const kind = 'CountCappedMultiSelect { min: 1, max: 2, picked: 0, distinct: true }';

  it('dispatches each pick immediately and renders consumed cards as non-toggleable', () => {
    const onAction = vi.fn();
    // Card A (T) already consumed: NOT legal and NOT in validIndices.
    render(
      <TrashSelectModal
        {...baseProps({
          pendingSelection: multiSel(kind, [T + 1], { kind }),
          actionMask: mask(T + 1, SELECTION.DECLINE),
          ownTrashIds: ['A', 'B'],
          onAction,
        })}
      />,
    );
    // Consumed card A renders selected.
    expect(screen.getByTestId(`trash-card-${T}`)).toHaveAttribute('data-selected', 'true');
    // Clicking it does nothing (not toggleable in immediate mode).
    fireEvent.click(screen.getByTitle('A'));
    expect(onAction).not.toHaveBeenCalled();
    // Clicking a live candidate commits immediately.
    fireEvent.click(screen.getByTitle('B'));
    expect(onAction).toHaveBeenCalledWith(T + 1);
  });

  it('Done dispatches PASS when decline is legal', () => {
    const onAction = vi.fn();
    render(
      <TrashSelectModal
        {...baseProps({
          pendingSelection: multiSel(kind, [T, T + 1], { kind }),
          actionMask: mask(T, T + 1, SELECTION.DECLINE),
          ownTrashIds: ['A', 'B'],
          onAction,
        })}
      />,
    );
    fireEvent.click(screen.getByTestId('trash-select-done'));
    expect(onAction).toHaveBeenCalledWith(SELECTION.DECLINE);
  });
});
