import { describe, expect, it } from 'vitest';
import { useMemo, useState } from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { RailProvider, useRailSection, useRailSectionSlot } from './RailContext';

function Slot() {
  const section = useRailSectionSlot();
  return <div data-testid="slot">{section}</div>;
}

function Contributor({ label }: { label: string }) {
  // Memoize the node as real callers (e.g. DeckLibraryPage) do, so it only
  // re-publishes when its content changes — passing a fresh node every render
  // would loop (publish → re-render → publish …).
  const node = useMemo(() => <span data-testid="contrib">{label}</span>, [label]);
  useRailSection(node);
  return null;
}

function Harness() {
  const [mounted, setMounted] = useState(true);
  const [label, setLabel] = useState('folders');
  return (
    <RailProvider>
      <button type="button" onClick={() => setMounted(false)}>
        unmount
      </button>
      <button type="button" onClick={() => setLabel('formats')}>
        relabel
      </button>
      <Slot />
      {mounted && <Contributor label={label} />}
    </RailProvider>
  );
}

describe('RailContext', () => {
  it('renders a contributed section into the shell slot', () => {
    render(<Harness />);
    expect(screen.getByTestId('contrib')).toHaveTextContent('folders');
  });

  it('republishes the section when the node changes', () => {
    render(<Harness />);
    fireEvent.click(screen.getByRole('button', { name: 'relabel' }));
    expect(screen.getByTestId('contrib')).toHaveTextContent('formats');
  });

  it('clears the section when the contributing page unmounts', () => {
    render(<Harness />);
    fireEvent.click(screen.getByRole('button', { name: 'unmount' }));
    expect(screen.queryByTestId('contrib')).not.toBeInTheDocument();
    expect(screen.getByTestId('slot')).toBeEmptyDOMElement();
  });
});
