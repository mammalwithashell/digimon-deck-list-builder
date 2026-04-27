import { useEffect, useState } from 'react';
import { useLocation, useNavigate, useParams } from 'react-router-dom';
import { CardSearchPanel } from '@/components/deckbuilder/CardSearchPanel';
import { DeckListPanel } from '@/components/deckbuilder/DeckListPanel';
import { DeckStats } from '@/components/deckbuilder/DeckStats';
import { DeckSelector } from '@/components/deckbuilder/DeckSelector';
import { ValidationPanel } from '@/components/deckbuilder/ValidationPanel';
import { ImportExport } from '@/components/deckbuilder/ImportExport';
import { CardDetail } from '@/components/shared/CardDetail';
import { useDeckBuilderStore } from '@/stores/deckBuilderStore';
import { useAuthStore } from '@/stores/authStore';
import { getCardById } from '@/api/digimonCardApi';
import * as deckApi from '@/api/deckApi';
import * as deckStore from '@/storage/deckStore';
import type { DeckEntry } from '@/types/deck';

// Desktop saves decks to the local Tauri-backed store; web keeps hitting
// the hosted `/decks` API. The tested-cards allowlist + raw-validate calls
// already branch inside `deckApi` itself, so they stay on `deckApi`.
const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';
const decks = IS_DESKTOP ? deckStore : deckApi;

function groupCardIds(ids: string[], altArts: boolean[] = []): DeckEntry[] {
  const counts = new Map<string, { cardId: string; isAltArt: boolean; count: number }>();
  ids.forEach((cardId, i) => {
    const isAltArt = !!altArts[i];
    const key = `${cardId}|${isAltArt ? '1' : '0'}`;
    const existing = counts.get(key);
    if (existing) existing.count += 1;
    else counts.set(key, { cardId, isAltArt, count: 1 });
  });
  return Array.from(counts.values());
}

export function DeckBuilderPage() {
  const { id: routeDeckId } = useParams();
  const location = useLocation();
  const navigate = useNavigate();
  const {
    deckName,
    setDeckName,
    deckId,
    setDeckId,
    loadDeck,
    clearDeck,
    mainDeck,
    eggDeck,
    isDirty,
    setIsDirty,
    setValidationResult,
    selectedCardId,
    testedCardIds,
    setTestedCardIds,
  } = useDeckBuilderStore();

  const [showImport, setShowImport] = useState(false);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    const params = new URLSearchParams(location.search);
    if (location.pathname.endsWith('/new') || params.get('new') === '1') {
      clearDeck();
      setShowImport(false);
    } else if (params.get('import') === '1') {
      setShowImport(true);
    }
  }, [location.pathname, location.search, clearDeck]);

  // Alpha gate: fetch the tested-cards allowlist once per session.
  useEffect(() => {
    if (testedCardIds !== null) return;
    deckApi
      .listTestedCards()
      .then(setTestedCardIds)
      .catch(() => {
        // If the endpoint is unreachable, fall back to an empty set so
        // the user sees no cards rather than the unrestricted pool.
        setTestedCardIds([]);
    });
  }, [testedCardIds, setTestedCardIds]);

  useEffect(() => {
    if (!routeDeckId) return;
    let cancelled = false;

    async function loadRouteDeck() {
      try {
        const deck = await decks.getDeck(routeDeckId!);
        const mainEntries = groupCardIds(deck.main_deck, deck.main_deck_alt_arts);
        const eggEntries = groupCardIds(deck.egg_deck, deck.egg_deck_alt_arts);
        const allIds = [...new Set([...deck.main_deck, ...deck.egg_deck])];
        const cardDataMap = new Map<string, Awaited<ReturnType<typeof getCardById>>>();
        await Promise.allSettled(
          allIds.map(async (id) => {
            const data = await getCardById(id);
            if (data) cardDataMap.set(id, data);
          }),
        );
        for (const entry of [...mainEntries, ...eggEntries]) {
          const data = cardDataMap.get(entry.cardId);
          if (data) entry.cardData = data;
        }
        if (!cancelled) loadDeck(deck.id, deck.name, mainEntries, eggEntries);
      } catch {
        if (!cancelled) clearDeck();
      }
    }

    void loadRouteDeck();
    return () => {
      cancelled = true;
    };
  }, [routeDeckId, loadDeck, clearDeck]);

  const handleSave = async () => {
    setSaving(true);
    try {
      const mainIds = mainDeck.flatMap((e) => Array(e.count).fill(e.cardId) as string[]);
      const eggIds = eggDeck.flatMap((e) => Array(e.count).fill(e.cardId) as string[]);
      const mainAlts = mainDeck.flatMap(
        (e) => Array(e.count).fill(!!e.isAltArt) as boolean[],
      );
      const eggAlts = eggDeck.flatMap(
        (e) => Array(e.count).fill(!!e.isAltArt) as boolean[],
      );

      if (IS_DESKTOP) {
        // Single write path on desktop: `putDeck` creates when `id` is
        // empty and updates when present. Stamp the guest owner_id from
        // the bootstrap cache so decks get associated with the caller.
        const ownerId = useAuthStore.getState().user?.id ?? 'guest';
        const saved = await deckStore.putDeck({
          id: deckId ?? undefined,
          owner_id: ownerId,
          name: deckName,
          game_mode: 'standard',
          main_deck: mainIds,
          egg_deck: eggIds,
          main_deck_alt_arts: mainAlts,
          egg_deck_alt_arts: eggAlts,
        });
        setDeckId(saved.id);
        if (!routeDeckId) navigate(`/deckbuilder/${saved.id}`, { replace: true });
      } else if (deckId) {
        await deckApi.updateDeck(deckId, {
          name: deckName,
          main_deck: mainIds,
          egg_deck: eggIds,
          main_deck_alt_arts: mainAlts,
          egg_deck_alt_arts: eggAlts,
        });
      } else {
        const created = await deckApi.createDeck({
          name: deckName,
          main_deck: mainIds,
          egg_deck: eggIds,
          main_deck_alt_arts: mainAlts,
          egg_deck_alt_arts: eggAlts,
          game_mode: 'standard',
        });
        setDeckId(created.id);
        navigate(`/deckbuilder/${created.id}`, { replace: true });
      }
      setIsDirty(false);
    } catch {
      // Ignore
    } finally {
      setSaving(false);
    }
  };

  const handleValidate = async () => {
    const mainIds = mainDeck.flatMap((e) => Array(e.count).fill(e.cardId) as string[]);
    const eggIds = eggDeck.flatMap((e) => Array(e.count).fill(e.cardId) as string[]);
    try {
      const result = await deckApi.validateDeckRaw(mainIds, eggIds);
      setValidationResult(result);
    } catch {
      // Ignore
    }
  };

  return (
    <div className="h-[calc(100vh-56px)] flex flex-col">
      {/* Top bar */}
      <div className="flex items-center gap-3 px-4 py-2 bg-gray-800 border-b border-gray-700">
        <DeckSelector />
        <input
          type="text"
          value={deckName}
          onChange={(e) => setDeckName(e.target.value)}
          className="px-2 py-1 bg-gray-700 border border-gray-600 rounded text-sm text-gray-200 focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
        <button
          onClick={handleSave}
          disabled={saving || !isDirty}
          className="px-3 py-1 bg-blue-600 hover:bg-blue-500 disabled:bg-blue-800 disabled:opacity-50 text-white text-sm rounded"
        >
          {saving ? 'Saving...' : 'Save'}
        </button>
        <button
          onClick={handleValidate}
          className="px-3 py-1 bg-gray-600 hover:bg-gray-500 text-white text-sm rounded"
        >
          Validate
        </button>
        <button
          onClick={() => setShowImport(true)}
          className="px-3 py-1 bg-gray-600 hover:bg-gray-500 text-white text-sm rounded"
        >
          Import/Export
        </button>
      </div>

      {/* Main content */}
      <div className="flex-1 flex min-h-0">
        {/* Left: Search panel */}
        <div className="flex-[3] flex flex-col min-w-0 border-r border-gray-700">
          <div className="flex-1 overflow-y-auto p-3">
            <CardSearchPanel />
          </div>
          {/* Card detail below search */}
          {selectedCardId && (
            <div className="border-t border-gray-700 p-2 max-h-[280px] overflow-y-auto">
              <CardDetail />
            </div>
          )}
        </div>

        {/* Right: Deck panel */}
        <div className="flex-[1] flex flex-col min-w-[260px] max-w-[360px] p-3 gap-2">
          <DeckStats />
          <ValidationPanel />
          <div className="flex-1 overflow-y-auto">
            <DeckListPanel />
          </div>
        </div>
      </div>

      <ImportExport isOpen={showImport} onClose={() => setShowImport(false)} />
    </div>
  );
}
