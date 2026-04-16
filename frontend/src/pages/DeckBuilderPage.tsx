import { useState } from 'react';
import { CardSearchPanel } from '@/components/deckbuilder/CardSearchPanel';
import { DeckListPanel } from '@/components/deckbuilder/DeckListPanel';
import { DeckStats } from '@/components/deckbuilder/DeckStats';
import { DeckSelector } from '@/components/deckbuilder/DeckSelector';
import { ValidationPanel } from '@/components/deckbuilder/ValidationPanel';
import { ImportExport } from '@/components/deckbuilder/ImportExport';
import { CardDetail } from '@/components/shared/CardDetail';
import { useDeckBuilderStore } from '@/stores/deckBuilderStore';
import * as deckApi from '@/api/deckApi';

export function DeckBuilderPage() {
  const {
    deckName,
    setDeckName,
    deckId,
    setDeckId,
    mainDeck,
    eggDeck,
    isDirty,
    setIsDirty,
    setValidationResult,
    selectedCardId,
  } = useDeckBuilderStore();

  const [showImport, setShowImport] = useState(false);
  const [saving, setSaving] = useState(false);

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

      if (deckId) {
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
