import { useCallback, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import * as library from '@/api/deckLibraryAdapter';
import { createLabGame, normalizeSeedInput } from '@/api/gameApi';
import { isInTauriRuntime } from '@/api/engineRuntime';
import { InBetweenShell } from '@/features/play/InBetweenShell';
import {
  mainSlotLabel,
  moveCard,
  seatProblem,
  shuffled,
  toEngineDeck,
  toStaged,
  type SeatStaging,
  type StagedCard,
} from '@/features/play/labStaging';
import type { DeckSummary } from '@/types/deck';
import { getCardImageUrl } from '@/utils/cardImages';
import './LabSetupPage.css';

// Remember the last staging so "tweak one card, run the combo again" is a
// two-click loop. Per-viewer convenience only — losing it is harmless.
const STAGING_KEY = 'digimon.labStaging.v1';

interface SavedStaging {
  seats: [SavedSeat, SavedSeat];
  firstPlayer: 0 | 1;
  seed: string;
}

interface SavedSeat {
  deckId: string | null;
  deckName: string;
  main: string[];
  eggs: string[];
}

const emptySeat = (): SeatStaging => ({ deckId: null, deckName: '', main: [], eggs: [] });

function loadSaved(): SavedStaging | null {
  try {
    const raw = localStorage.getItem(STAGING_KEY);
    return raw ? (JSON.parse(raw) as SavedStaging) : null;
  } catch {
    return null;
  }
}

type Seats = [SeatStaging, SeatStaging];

function saveStaging(seats: Seats, firstPlayer: 0 | 1, seed: string) {
  const toSaved = (s: SeatStaging): SavedSeat => ({
    deckId: s.deckId,
    deckName: s.deckName,
    main: s.main.map((c) => c.cardId),
    eggs: s.eggs.map((c) => c.cardId),
  });
  try {
    localStorage.setItem(
      STAGING_KEY,
      JSON.stringify({ seats: [toSaved(seats[0]), toSaved(seats[1])], firstPlayer, seed }),
    );
  } catch {
    // Storage unavailable — staging just won't be remembered.
  }
}

function fromSaved(s: SavedSeat): SeatStaging {
  return { deckId: s.deckId, deckName: s.deckName, main: toStaged(s.main), eggs: toStaged(s.eggs) };
}

export function LabSetupPage() {
  const navigate = useNavigate();
  const saved = loadSaved();
  const [decks, setDecks] = useState<DeckSummary[]>([]);
  const [seats, setSeats] = useState<Seats>(() =>
    saved ? [fromSaved(saved.seats[0]), fromSaved(saved.seats[1])] : [emptySeat(), emptySeat()],
  );
  const [firstPlayer, setFirstPlayer] = useState<0 | 1>(saved?.firstPlayer ?? 0);
  const [seedInput, setSeedInput] = useState(saved?.seed ?? '');
  const [error, setError] = useState('');
  const [launching, setLaunching] = useState(false);

  useEffect(() => {
    library.listDecks().then(setDecks).catch(() => setDecks([]));
  }, []);

  const updateSeat = useCallback((seat: number, fn: (s: SeatStaging) => SeatStaging) => {
    setSeats((prev) => (seat === 0 ? [fn(prev[0]), prev[1]] : [prev[0], fn(prev[1])]));
  }, []);

  const loadDeck = useCallback(
    async (seat: number, deckId: string) => {
      if (!deckId) {
        updateSeat(seat, () => emptySeat());
        return;
      }
      try {
        const deck = await library.getDeck(deckId);
        updateSeat(seat, () => ({
          deckId: deck.id,
          deckName: deck.name,
          main: toStaged(deck.main_deck),
          eggs: toStaged(deck.egg_deck),
        }));
      } catch (err) {
        setError(`Could not load deck: ${(err as Error).message}`);
      }
    },
    [updateSeat],
  );

  const handleStart = async () => {
    setError('');
    for (const [i, seat] of seats.entries()) {
      const problem = seatProblem(seat);
      if (problem) {
        setError(`Seat ${i + 1}: ${problem}`);
        return;
      }
    }
    let seed: string | null;
    try {
      seed = normalizeSeedInput(seedInput);
    } catch (err) {
      setError((err as Error).message);
      return;
    }
    setLaunching(true);
    try {
      saveStaging(seats, firstPlayer, seedInput);
      const response = await createLabGame({
        deck1: toEngineDeck(seats[0]),
        deck2: toEngineDeck(seats[1]),
        firstPlayer,
        seed,
      });
      navigate(`/game/${response.game_id}`);
    } catch (err) {
      setError(String((err as Error).message ?? err));
    } finally {
      setLaunching(false);
    }
  };

  if (!isInTauriRuntime()) {
    return (
      <InBetweenShell title="LAB" stepLabel="LAB" crumbs={[{ label: 'PLAY', href: '/play' }, { label: 'LAB' }]}>
        <main className="lab-main">
          <p className="lab-error">Lab games run on the local engine and are only available in the desktop app.</p>
        </main>
      </InBetweenShell>
    );
  }

  return (
    <InBetweenShell
      title="LAB"
      stepLabel="LAB"
      crumbs={[{ label: 'PLAY', href: '/play' }, { label: 'LAB SETUP' }]}
      rightSlot={<span>YOU CONTROL BOTH SEATS</span>}
    >
      <main className="lab-main">
        <header className="lab-header">
          <h1>COMBO LAB</h1>
          <p>
            Stage both decks top-first. No shuffle and no mulligan: slots 1–5 are the opening hand,
            6–10 are security (SEC 1 is checked first), and the rest are drawn in order. The first
            player skips their turn-1 draw. The board flips to whichever seat has to act.
          </p>
        </header>

        <div className="lab-seats">
          {seats.map((seat, i) => (
            <SeatColumn
              key={i}
              seatIndex={i}
              seat={seat}
              decks={decks}
              onPickDeck={(id) => loadDeck(i, id)}
              onChange={(fn) => updateSeat(i, fn)}
            />
          ))}
        </div>

        <footer className="lab-footer">
          <fieldset className="lab-first">
            <legend>FIRST PLAYER</legend>
            {[0, 1].map((p) => (
              <label key={p}>
                <input
                  type="radio"
                  name="lab-first-player"
                  checked={firstPlayer === p}
                  onChange={() => setFirstPlayer(p as 0 | 1)}
                />
                Seat {p + 1}
              </label>
            ))}
          </fieldset>
          <label className="lab-seed">
            SEED (in-game randomness)
            <input
              value={seedInput}
              onChange={(e) => setSeedInput(e.target.value)}
              placeholder="random"
            />
          </label>
          {error && <span className="lab-error" role="alert">{error}</span>}
          <button type="button" className="lab-start" onClick={handleStart} disabled={launching}>
            {launching ? 'STARTING…' : 'START LAB GAME'}
          </button>
        </footer>
      </main>
    </InBetweenShell>
  );
}

interface SeatColumnProps {
  seatIndex: number;
  seat: SeatStaging;
  decks: DeckSummary[];
  onPickDeck: (deckId: string) => void;
  onChange: (fn: (s: SeatStaging) => SeatStaging) => void;
}

function SeatColumn({ seatIndex, seat, decks, onPickDeck, onChange }: SeatColumnProps) {
  return (
    <section className="lab-seat" aria-label={`Seat ${seatIndex + 1}`}>
      <div className="lab-seat-head">
        <h2>SEAT {seatIndex + 1}</h2>
        <select
          value={seat.deckId ?? ''}
          onChange={(e) => onPickDeck(e.target.value)}
          aria-label={`Seat ${seatIndex + 1} deck`}
        >
          <option value="">— choose a deck —</option>
          {/* A remembered deck that is no longer in the library still shows. */}
          {seat.deckId && !decks.some((d) => d.id === seat.deckId) && (
            <option value={seat.deckId}>{seat.deckName || seat.deckId}</option>
          )}
          {decks.map((d) => (
            <option key={d.id} value={d.id}>
              {d.name}
            </option>
          ))}
        </select>
        <div className="lab-seat-tools">
          <button
            type="button"
            onClick={() => onChange((s) => ({ ...s, main: shuffled(s.main), eggs: shuffled(s.eggs) }))}
            disabled={!seat.deckId}
          >
            SHUFFLE
          </button>
          <button type="button" onClick={() => seat.deckId && onPickDeck(seat.deckId)} disabled={!seat.deckId}>
            RESET
          </button>
        </div>
      </div>

      <h3>EGG DECK ({seat.eggs.length})</h3>
      <OrderedList
        cards={seat.eggs}
        label={(i) => (i === 0 ? 'HATCH 1' : `HATCH ${i + 1}`)}
        zoneOf={() => 'egg'}
        onReorder={(eggs) => onChange((s) => ({ ...s, eggs }))}
      />

      <h3>MAIN DECK ({seat.main.length}) — TOP FIRST</h3>
      <OrderedList
        cards={seat.main}
        label={(i) => mainSlotLabel(i).text}
        zoneOf={(i) => mainSlotLabel(i).zone}
        onReorder={(main) => onChange((s) => ({ ...s, main }))}
      />
    </section>
  );
}

interface OrderedListProps {
  cards: StagedCard[];
  label: (index: number) => string;
  zoneOf: (index: number) => string;
  onReorder: (cards: StagedCard[]) => void;
}

function OrderedList({ cards, label, zoneOf, onReorder }: OrderedListProps) {
  const [dragFrom, setDragFrom] = useState<number | null>(null);
  const [dragOver, setDragOver] = useState<number | null>(null);

  if (cards.length === 0) return <p className="lab-empty">Empty</p>;

  const move = (from: number, to: number) => onReorder(moveCard(cards, from, to));

  return (
    <ol className="lab-list">
      {cards.map((card, i) => (
        <li
          key={card.uid}
          className={`lab-row lab-row--${zoneOf(i)}${dragOver === i ? ' lab-row--over' : ''}`}
          draggable
          onDragStart={(e) => {
            setDragFrom(i);
            e.dataTransfer.effectAllowed = 'move';
          }}
          onDragOver={(e) => {
            e.preventDefault();
            setDragOver(i);
          }}
          onDragLeave={() => setDragOver((cur) => (cur === i ? null : cur))}
          onDrop={(e) => {
            e.preventDefault();
            if (dragFrom != null) move(dragFrom, i);
            setDragFrom(null);
            setDragOver(null);
          }}
          onDragEnd={() => {
            setDragFrom(null);
            setDragOver(null);
          }}
        >
          <span className="lab-slot">{i + 1}</span>
          <span className="lab-zone">{label(i)}</span>
          <img src={getCardImageUrl(card.cardId)} alt="" loading="lazy" className="lab-thumb" />
          <span className="lab-card-id">{card.cardId}</span>
          <span className="lab-row-tools">
            <button type="button" title="Move to top" onClick={() => move(i, 0)} disabled={i === 0}>
              ⤒
            </button>
            <button type="button" title="Move up" onClick={() => move(i, i - 1)} disabled={i === 0}>
              ↑
            </button>
            <button
              type="button"
              title="Move down"
              onClick={() => move(i, i + 1)}
              disabled={i === cards.length - 1}
            >
              ↓
            </button>
            <button
              type="button"
              title="Move to bottom"
              onClick={() => move(i, cards.length - 1)}
              disabled={i === cards.length - 1}
            >
              ⤓
            </button>
          </span>
        </li>
      ))}
    </ol>
  );
}
