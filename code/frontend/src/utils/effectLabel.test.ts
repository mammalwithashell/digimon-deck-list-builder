import { describe, expect, it } from 'vitest';
import {
  shortEffectTag,
  activatableEffectLabel,
  effectNameCounts,
  isActivatableEffect,
} from './effectLabel';
import type { DecodedAction } from '@/types/game';

function action(over: Partial<DecodedAction> = {}): DecodedAction {
  return {
    actionId: 1012,
    playerId: 0,
    phase: 'Main',
    kind: 'field_effect',
    label: '',
    sourceZone: 'battle',
    sourceIndex: 1,
    targetZone: null,
    targetIndex: null,
    cardId: 'ST6-13',
    cardName: 'CresGarurumon',
    effectName:
      '[Main] Digi-Burst 2: play 1 purple level 3 Digimon from trash free',
    ...over,
  };
}

describe('shortEffectTag', () => {
  it('strips a leading timing tag and keeps the headline before ":"', () => {
    expect(
      shortEffectTag(
        '[Main] Digi-Burst 2: play 1 purple level 3 Digimon from trash free',
      ),
    ).toBe('Digi-Burst 2');
  });

  it('strips a run of multiple tags', () => {
    expect(shortEffectTag('[On Play][When Digivolving] Draw 1')).toBe('Draw 1');
  });

  it('returns null when only tags remain', () => {
    expect(shortEffectTag('<Blocker>')).toBeNull();
  });

  it('returns null for empty/null input', () => {
    expect(shortEffectTag(null)).toBeNull();
    expect(shortEffectTag('')).toBeNull();
    expect(shortEffectTag('   ')).toBeNull();
  });

  it('truncates very long tags with an ellipsis', () => {
    const tag = shortEffectTag(
      'A very long effect name without any colon at all here yes indeed',
    );
    expect(tag).not.toBeNull();
    expect(tag!.length).toBeLessThanOrEqual(28);
    expect(tag!.endsWith('…')).toBe(true);
  });
});

describe('activatableEffectLabel', () => {
  it('formats "{card}: {tag}" without a slot for a unique name', () => {
    expect(activatableEffectLabel(action(), false)).toBe(
      'CresGarurumon: Digi-Burst 2',
    );
  });

  it('appends "(slot N)" when disambiguating duplicate names', () => {
    expect(activatableEffectLabel(action({ sourceIndex: 4 }), true)).toBe(
      'CresGarurumon (slot 4): Digi-Burst 2',
    );
  });

  it('falls back to card name when there is no usable effect tag', () => {
    expect(
      activatableEffectLabel(action({ effectName: '<Blocker>' }), false),
    ).toBe('CresGarurumon');
  });

  it('falls back to card name when effectName is null', () => {
    expect(activatableEffectLabel(action({ effectName: null }), false)).toBe(
      'CresGarurumon',
    );
  });
});

describe('effectNameCounts', () => {
  it('counts duplicate card names across the surfaced set', () => {
    const counts = effectNameCounts([
      action({ actionId: 1012, cardName: 'CresGarurumon' }),
      action({ actionId: 1042, cardName: 'CresGarurumon' }),
      action({ actionId: 1052, cardName: 'Garurumon' }),
    ]);
    expect(counts.get('CresGarurumon')).toBe(2);
    expect(counts.get('Garurumon')).toBe(1);
  });
});

describe('isActivatableEffect', () => {
  it('accepts field/hand/trash effect kinds and rejects others', () => {
    expect(isActivatableEffect(action({ kind: 'field_effect' }))).toBe(true);
    expect(isActivatableEffect(action({ kind: 'hand_effect' }))).toBe(true);
    expect(isActivatableEffect(action({ kind: 'trash_effect' }))).toBe(true);
    expect(isActivatableEffect(action({ kind: 'play' }))).toBe(false);
    expect(isActivatableEffect(action({ kind: 'attack' }))).toBe(false);
  });
});
