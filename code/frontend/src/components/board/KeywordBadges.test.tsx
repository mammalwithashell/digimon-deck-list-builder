import '@testing-library/jest-dom/vitest';
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { KeywordBadges, getKeywordMeta } from './KeywordBadges';

describe('KeywordBadges', () => {
  it('renders null when keywords list is empty', () => {
    const { container } = render(<KeywordBadges keywords={[]} />);
    expect(container.firstChild).toBeNull();
  });

  it('renders full untruncated keyword names with semantic icons and hover descriptions', () => {
    render(
      <KeywordBadges
        keywords={['armor_purge', 'blast_digivolve', 'material_save']}
        variant="inspector"
      />,
    );

    const armorPurge = screen.getByTestId('keyword-badge-armor_purge');
    const blastDigi = screen.getByTestId('keyword-badge-blast_digivolve');
    const matSave = screen.getByTestId('keyword-badge-material_save');

    expect(armorPurge).toHaveTextContent('Armor Purge');
    expect(blastDigi).toHaveTextContent('Blast Digivolve');
    expect(matSave).toHaveTextContent('Material Save');

    // Semantic SVG icon rendered inside each badge
    expect(armorPurge.querySelector('svg')).toBeInTheDocument();
    expect(blastDigi.querySelector('svg')).toBeInTheDocument();
    expect(matSave.querySelector('svg')).toBeInTheDocument();

    // Full rules description in tooltip
    expect(armorPurge).toHaveAttribute(
      'title',
      expect.stringContaining('trash the top card of this Digimon'),
    );
  });

  it('distinguishes innate vs gained keywords cleanly', () => {
    render(
      <KeywordBadges
        keywords={['blocker', 'rush', 'piercing']}
        innateKeywords={['blocker']}
        gainedKeywords={['rush', 'piercing']}
      />,
    );

    const blocker = screen.getByTestId('keyword-badge-blocker');
    const rush = screen.getByTestId('keyword-badge-rush');
    const piercing = screen.getByTestId('keyword-badge-piercing');

    expect(blocker).toHaveAttribute('data-keyword-origin', 'innate');
    expect(blocker).toHaveClass('hs-kw-badge--innate');
    expect(blocker).toHaveAttribute('title', expect.stringContaining('(Innate)'));

    expect(rush).toHaveAttribute('data-keyword-origin', 'gained');
    expect(rush).toHaveClass('hs-kw-badge--gained');
    expect(rush).toHaveAttribute('title', expect.stringContaining('(Granted by effect)'));

    expect(piercing).toHaveAttribute('data-keyword-origin', 'gained');
    expect(piercing).toHaveClass('hs-kw-badge--gained');
  });

  it('collapses overflow keywords into +N chip in slot mode while showing all in inspector mode', () => {
    const keywords = ['blocker', 'rush', 'raid', 'piercing', 'reboot'];
    const { rerender } = render(
      <KeywordBadges keywords={keywords} gainedKeywords={['piercing']} variant="slot" />,
    );

    expect(screen.getByTestId('keyword-badge-blocker')).toBeInTheDocument();
    expect(screen.getByTestId('keyword-badge-rush')).toBeInTheDocument();
    expect(screen.getByTestId('keyword-badge-raid')).toBeInTheDocument();
    expect(screen.queryByTestId('keyword-badge-piercing')).not.toBeInTheDocument();

    const overflow = screen.getByTestId('keyword-overflow-badge');
    expect(overflow).toHaveTextContent('+2');
    expect(overflow).toHaveAttribute('title', expect.stringContaining('Piercing (Granted)'));
    expect(overflow).toHaveAttribute('title', expect.stringContaining('Reboot (Innate)'));

    rerender(
      <KeywordBadges keywords={keywords} gainedKeywords={['piercing']} variant="inspector" />,
    );

    expect(screen.getByTestId('keyword-badge-piercing')).toBeInTheDocument();
    expect(screen.getByTestId('keyword-badge-reboot')).toBeInTheDocument();
    expect(screen.queryByTestId('keyword-overflow-badge')).not.toBeInTheDocument();
  });

  it('humanizes unknown future keywords cleanly without truncating', () => {
    const meta = getKeywordMeta('super_mega_link');
    expect(meta.label).toBe('Super Mega Link');
    render(<KeywordBadges keywords={['super_mega_link']} />);
    expect(screen.getByTestId('keyword-badge-super_mega_link')).toHaveTextContent('Super Mega Link');
  });
});
