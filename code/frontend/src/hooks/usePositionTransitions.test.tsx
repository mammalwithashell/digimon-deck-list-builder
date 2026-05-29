import '@testing-library/jest-dom/vitest';
import { act, fireEvent, render } from '@testing-library/react';
import { useState } from 'react';
import { describe, expect, it } from 'vitest';
import { usePositionTransitions } from './usePositionTransitions';

/**
 * Test harness component: renders a list of divs whose keys can be
 * reordered via a control. The hook should apply inverse translates
 * to elements that moved.
 */
function Harness({ initialKeys }: { initialKeys: string[] }) {
  const [keys, setKeys] = useState(initialKeys);
  const { registerNode } = usePositionTransitions(keys, { durationMs: 200 });

  return (
    <div>
      {keys.map((k) => (
        <div key={k} ref={registerNode(k)} data-testid={`item-${k}`}>
          {k}
        </div>
      ))}
      <button
        type="button"
        data-testid="remove-middle"
        onClick={() => setKeys((cur) => cur.filter((k) => k !== 'b'))}
      >
        remove b
      </button>
    </div>
  );
}

describe('usePositionTransitions', () => {
  it('renders without error for a stable set of keys', () => {
    const { getByTestId } = render(<Harness initialKeys={['a', 'b', 'c']} />);
    expect(getByTestId('item-a')).toBeInTheDocument();
    expect(getByTestId('item-b')).toBeInTheDocument();
    expect(getByTestId('item-c')).toBeInTheDocument();
  });

  it('forgets a key when the corresponding element is removed', () => {
    const { getByTestId, queryByTestId } = render(
      <Harness initialKeys={['a', 'b', 'c']} />,
    );
    act(() => {
      fireEvent.click(getByTestId('remove-middle'));
    });
    expect(queryByTestId('item-b')).not.toBeInTheDocument();
    // The surviving elements should still be in the DOM.
    expect(getByTestId('item-a')).toBeInTheDocument();
    expect(getByTestId('item-c')).toBeInTheDocument();
  });

  it('does not animate on the very first render (no prior rect)', () => {
    const { getByTestId } = render(<Harness initialKeys={['a']} />);
    const node = getByTestId('item-a');
    // No transform on first paint — FLIP only kicks in for *subsequent*
    // renders where the element's rect changed.
    expect(node.style.transform).toBe('');
  });

  // Note: jsdom doesn't compute layout, so getBoundingClientRect always
  // returns zero rects in tests. Therefore the "actually animate when
  // an element moves" path can only be exercised in a real browser. The
  // tests above cover the contract that the hook is wired correctly and
  // doesn't crash on mount/unmount/key changes.
});
