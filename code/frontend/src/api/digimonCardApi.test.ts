import { describe, expect, it } from 'vitest';
import { mapApiCard } from './digimonCardApi';

const LINK_REQUIREMENTS = 'Link Requirements [Link] [Appmon] trait: Cost 1\r\n'
  + '(Plug this card from the hand or battle area sideways into the specified Digimon in the battle area.)';

describe('mapApiCard', () => {
  it("files a link card's source_effect as its link effect, not as an inherited effect", () => {
    const card = mapApiCard({
      id: 'BT21-041',
      name: 'Calendamon',
      source_effect: "All of your opponent's Security Digimon get -3000 DP.",
      link_requirements: LINK_REQUIREMENTS,
      link_dp: 2000,
    });
    expect(card.soureeffect).toBe('');
    expect(card.linkeffect).toBe("All of your opponent's Security Digimon get -3000 DP.");
  });

  it('drops a source_effect that only copies the link requirement', () => {
    const card = mapApiCard({
      id: 'BT21-059',
      name: 'Timemon',
      source_effect: LINK_REQUIREMENTS,
      link_requirements: LINK_REQUIREMENTS,
      link_dp: 3000,
    });
    expect(card.soureeffect).toBe('');
    expect(card.linkeffect).toBe('');
  });

  it('keeps an ordinary inherited effect', () => {
    const card = mapApiCard({
      id: 'ST1-03',
      name: 'Agumon',
      source_effect: '[Your Turn] This Digimon gets +1000 DP.',
      link_requirements: '',
      link_dp: null,
    });
    expect(card.soureeffect).toBe('[Your Turn] This Digimon gets +1000 DP.');
    expect(card.linkeffect).toBeUndefined();
  });
});
