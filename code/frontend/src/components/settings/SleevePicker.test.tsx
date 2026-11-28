import '@testing-library/jest-dom/vitest';
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { DndContext } from '@dnd-kit/core';
import { Card } from '@/components/shared/Card';
import { DeckPile } from '@/components/board/DeckPile';
import { EggDeck } from '@/components/board/EggDeck';
import { HandZone } from '@/components/board/HandZone';
import { PlayerHalf } from '@/components/board/PlayerHalf';
import type { PlayerState } from '@/types/game';
import {
  CardSleevePicker,
  EggSleevePicker,
  SleevePicker,
  SleevePickerModal,
} from '@/components/settings/SleevePicker';
import {
  CLASSIC_SLEEVE_ID,
  DEFAULT_CARD_SLEEVE,
  DEFAULT_EGG_SLEEVE,
  clearCustomCardSleeve,
  clearCustomEggSleeve,
  getCardSleeveId,
  getCustomCardSleeveSrc,
  getCustomEggSleeveSrc,
  getEffectiveCardSleeveId,
  getEffectiveEggSleeveId,
  getEggSleeveId,
  setCardSleeveId,
  setCustomCardSleeve,
  setCustomEggSleeve,
  setDeckEggSleeveId,
  setDeckSleeveId,
  setEggSleeveId,
  sleeveUploadHelper,
} from '@/stores/sleeveStore';

describe('sleeveStore and SleevePicker', () => {
  beforeEach(() => {
    clearCustomCardSleeve();
    clearCustomEggSleeve();
    localStorage.clear();
    setDeckSleeveId(undefined);
    setDeckEggSleeveId(undefined);
    setCardSleeveId(DEFAULT_CARD_SLEEVE.id);
    setEggSleeveId(DEFAULT_EGG_SLEEVE.id);
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it('persists main card sleeve selection and updates faceDown Card & DeckPile', () => {
    const { container } = render(
      <>
        <CardSleevePicker />
        <Card cardId="BT1-001" faceDown />
        <DeckPile count={42} />
      </>,
    );

    expect(container.querySelectorAll('img[src="/sleeves/digimon-standard.webp"]')).toHaveLength(
      3,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Alphamon, Official Card Sleeves' }));

    expect(getCardSleeveId()).toBe('alphamon');
    expect(localStorage.getItem('digimon.sleeve')).toBe('alphamon');
    expect(container.querySelector('img[src="/sleeves/alphamon.webp"]')).toBeTruthy();
    expect(screen.getByTestId('deck-pile-count')).toHaveTextContent('42');
    // Preserves sr-only DCG text for SelectionPanel compatibility
    expect(screen.getAllByText('DCG').length).toBeGreaterThanOrEqual(2);
  });

  it('ignores unknown sleeve ids and supports classic fallback + image error fallback', () => {
    setCardSleeveId('does-not-exist');
    expect(getCardSleeveId()).toBe(DEFAULT_CARD_SLEEVE.id);

    setCardSleeveId(CLASSIC_SLEEVE_ID);
    const { container } = render(<Card cardId="BT1-001" faceDown />);
    expect(screen.getByTestId('card-back-classic')).toBeInTheDocument();
    expect(container.querySelector('img')).toBeNull();

    // Switching to an image sleeve and simulating load error falls back to classic
    act(() => setCardSleeveId('omnimon'));
    const img = container.querySelector('img[src="/sleeves/omnimon.webp"]')!;
    expect(img).toBeTruthy();
    fireEvent.error(img);
    expect(screen.getByTestId('card-back-classic')).toBeInTheDocument();
  });

  it('keeps Digi-Egg sleeves independent from Main Card sleeves and renders EggDeck hatch state', () => {
    const onHatch = vi.fn();
    const { container } = render(
      <>
        <Card cardId="c1" faceDown />
        <EggDeck count={4} canHatch onClick={onHatch} />
      </>,
    );

    act(() => setCardSleeveId('omnimon'));
    expect(container.querySelector('img[src="/sleeves/omnimon.webp"]')).toBeTruthy();
    expect(container.querySelector('img[src="/sleeves/digimon-egg.webp"]')).toBeTruthy();

    act(() => setEggSleeveId('gold'));
    expect(container.querySelector('img[src="/sleeves/gold.webp"]')).toBeTruthy();
    expect(screen.getByTestId('egg-hatch-indicator')).toHaveTextContent('Hatch');
    expect(screen.getByTestId('egg-deck-count')).toHaveTextContent('4');

    fireEvent.click(screen.getByRole('button', { name: /Hatch Digi-Egg/i }));
    expect(onHatch).toHaveBeenCalledTimes(1);
  });

  it('supports deck-level sleeve overrides with clean fallback', () => {
    setCardSleeveId('omnimon');
    setEggSleeveId('gold');
    setDeckSleeveId('alphamon');
    setDeckEggSleeveId('silver');
    expect(getEffectiveCardSleeveId()).toBe('alphamon');
    expect(getEffectiveEggSleeveId()).toBe('silver');

    setDeckSleeveId('missing-sleeve');
    setDeckEggSleeveId('custom-egg');
    expect(getEffectiveCardSleeveId()).toBe('omnimon');
    expect(getEffectiveEggSleeveId()).toBe('gold');
  });

  it('uploads, replaces, resets, and removes custom card and egg sleeves', async () => {
    const first = 'data:image/webp;base64,Zmlyc3Q=';
    const second = 'data:image/webp;base64,c2Vjb25k';
    vi.spyOn(sleeveUploadHelper, 'toStorableDataUrl')
      .mockResolvedValueOnce(first)
      .mockResolvedValueOnce(second);

    const { container } = render(
      <>
        <CardSleevePicker />
        <Card cardId="c1" faceDown />
      </>,
    );

    const input = container.querySelector('input[type="file"]')!;
    fireEvent.change(input, {
      target: { files: [new File(['img'], 'sleeve.png', { type: 'image/png' })] },
    });
    await waitFor(() => expect(getCustomCardSleeveSrc()).toBe(first));
    expect(getCardSleeveId()).toBe('custom');
    expect(container.querySelector(`img[src="${first}"]`)).toBeTruthy();

    fireEvent.change(input, {
      target: { files: [new File(['img2'], 'sleeve2.png', { type: 'image/png' })] },
    });
    await waitFor(() => expect(getCustomCardSleeveSrc()).toBe(second));

    fireEvent.click(screen.getByRole('button', { name: 'Reset to default' }));
    expect(getCardSleeveId()).toBe(DEFAULT_CARD_SLEEVE.id);

    fireEvent.click(screen.getByRole('button', { name: 'Remove custom image' }));
    expect(getCustomCardSleeveSrc()).toBeUndefined();
  });

  it('rejects non-image uploads and reports conversion errors without changing selection', async () => {
    setCardSleeveId('omnimon');
    const spy = vi
      .spyOn(sleeveUploadHelper, 'toStorableDataUrl')
      .mockRejectedValue(new Error('decode error'));

    const { container } = render(<CardSleevePicker />);
    const input = container.querySelector('input[type="file"]')!;

    fireEvent.change(input, {
      target: { files: [new File(['txt'], 'notes.txt', { type: 'text/plain' })] },
    });
    expect(screen.getByRole('status')).toHaveTextContent('Please select an image file.');
    expect(spy).not.toHaveBeenCalled();

    fireEvent.change(input, {
      target: { files: [new File(['bad'], 'bad.png', { type: 'image/png' })] },
    });
    await waitFor(() =>
      expect(screen.getByRole('status')).toHaveTextContent(/Could not load or store/),
    );
    expect(getCardSleeveId()).toBe('omnimon');
  });

  it('uploads and removes a custom Digi-Egg sleeve independently', async () => {
    const eggImg = 'data:image/webp;base64,ZWdn';
    vi.spyOn(sleeveUploadHelper, 'toStorableDataUrl').mockResolvedValueOnce(eggImg);
    setCustomCardSleeve('data:image/webp;base64,Y2FyZA==');

    const { container } = render(
      <>
        <EggSleevePicker />
        <Card cardId="egg-1" faceDown isEgg />
      </>,
    );

    const input = container.querySelector('input[type="file"]')!;
    fireEvent.change(input, {
      target: { files: [new File(['egg'], 'egg.png', { type: 'image/png' })] },
    });
    await waitFor(() => expect(getCustomEggSleeveSrc()).toBe(eggImg));
    expect(getEggSleeveId()).toBe('custom-egg');
    expect(container.querySelector(`img[src="${eggImg}"]`)).toBeTruthy();

    fireEvent.click(screen.getByRole('button', { name: 'Remove custom image' }));
    expect(getEggSleeveId()).toBe(DEFAULT_EGG_SLEEVE.id);
    expect(getCustomCardSleeveSrc()).toBe('data:image/webp;base64,Y2FyZA==');
  });

  it('switches tabs, filters sleeves, and opens/closes SleevePickerModal', () => {
    const onClose = vi.fn();
    render(<SleevePickerModal isOpen onClose={onClose} />);

    expect(screen.getByRole('dialog', { name: /Card & Digi-Egg Sleeves/i })).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('Search sleeves'), { target: { value: 'Loogamon' } });
    expect(screen.getByRole('button', { name: /Loogamon/i })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Omnimon/i })).toBeNull();

    fireEvent.click(screen.getByRole('tab', { name: 'Digi-Egg Sleeves' }));
    expect(screen.getByTestId('egg-sleeve-picker')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Close sleeve picker' }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('supports SleevePicker standalone rendering and Escape to close modal', () => {
    const onClose = vi.fn();
    render(
      <>
        <SleevePicker initialTab="egg" />
        <SleevePickerModal isOpen onClose={onClose} />
      </>,
    );
    expect(screen.getByTestId('egg-sleeve-picker')).toBeInTheDocument();
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('preserves previous custom egg sleeve when localStorage rejects write and keeps opponent HandZone on standard sleeve', () => {
    const prev = 'data:image/webp;base64,b2xkLWVnZw==';
    setCustomEggSleeve(prev);
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('quota');
    });
    expect(() => setCustomEggSleeve('data:image/webp;base64,bmV3')).toThrow('quota');
    expect(getCustomEggSleeveSrc()).toBe(prev);

    act(() => setCardSleeveId('official-2023-loogamon'));
    const { container } = render(
      <DndContext>
        <DeckPile count={30} />
        <HandZone cardIds={['BT1-001', 'BT1-002']} isOpponent />
      </DndContext>,
    );
    expect(
      container.querySelectorAll('img[src="/sleeves/official-2023-loogamon.webp"]'),
    ).toHaveLength(1);
    expect(
      container.querySelectorAll('img[src="/sleeves/digimon-standard.webp"]'),
    ).toHaveLength(2);
  });

  it('PlayerHalf passes isOpponent so opponent DeckPile and EggDeck stay on standard backs', () => {
    act(() => {
      setCardSleeveId('omnimon');
      setEggSleeveId('gold');
    });
    const basePlayer: PlayerState = {
      id: 1,
      memory: 0,
      handCount: 5,
      handIds: [],
      handCards: [],
      securityCount: 2,
      securityIds: [null, null],
      deckCount: 40,
      eggDeckCount: 4,
      battleAreaCount: 0,
      battleArea: [],
      breedingArea: null,
      trashIds: [],
    };
    const { container } = render(
      <DndContext>
        <div data-testid="opp-half">
          <PlayerHalf player={{ ...basePlayer, id: 2 }} isOpponent />
        </div>
        <div data-testid="you-half">
          <PlayerHalf player={basePlayer} isOpponent={false} />
        </div>
      </DndContext>,
    );
    const oppHalf = container.querySelector('[data-testid="opp-half"]')!;
    const youHalf = container.querySelector('[data-testid="you-half"]')!;
    expect(oppHalf.querySelector('img[src="/sleeves/digimon-standard.webp"]')).toBeTruthy();
    expect(oppHalf.querySelector('img[src="/sleeves/digimon-egg.webp"]')).toBeTruthy();
    expect(oppHalf.querySelector('img[src="/sleeves/omnimon.webp"]')).toBeNull();
    expect(oppHalf.querySelector('img[src="/sleeves/gold.webp"]')).toBeNull();
    expect(youHalf.querySelectorAll('img[src="/sleeves/omnimon.webp"]').length).toBeGreaterThanOrEqual(
      2,
    );
    expect(youHalf.querySelector('img[src="/sleeves/gold.webp"]')).toBeTruthy();
  });
});
