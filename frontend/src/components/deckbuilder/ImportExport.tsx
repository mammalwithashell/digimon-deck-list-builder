import { useState } from 'react';
import { Modal } from '@/components/shared/Modal';
import { useDeckBuilderStore } from '@/stores/deckBuilderStore';
import * as deckApi from '@/api/deckApi';
import { getCardById } from '@/api/digimonCardApi';

interface ImportExportProps {
  isOpen: boolean;
  onClose: () => void;
}

export function ImportExport({ isOpen, onClose }: ImportExportProps) {
  const [text, setText] = useState('');
  const [error, setError] = useState('');
  const [warnings, setWarnings] = useState<string[]>([]);
  const [importSuccess, setImportSuccess] = useState(false);
  const [isImporting, setIsImporting] = useState(false);
  const { mainDeck, eggDeck, loadDeck, deckName, setIsDirty } = useDeckBuilderStore();

  const handleImport = async () => {
    if (!text.trim()) return;
    setIsImporting(true);
    setError('');
    setWarnings([]);
    setImportSuccess(false);
    try {
      const parsed = await deckApi.parseDeck(text.trim());

      // Fetch card data for imported cards to get names/types
      const allIds = [...new Set([...parsed.main_deck, ...parsed.egg_deck])];
      const cardDataMap = new Map<string, Awaited<ReturnType<typeof getCardById>>>();
      await Promise.allSettled(
        allIds.map(async (id) => {
          const data = await getCardById(id);
          if (data) cardDataMap.set(id, data);
        }),
      );

      // Count duplicates
      const mainCounts = new Map<string, number>();
      for (const id of parsed.main_deck) {
        mainCounts.set(id, (mainCounts.get(id) ?? 0) + 1);
      }
      const eggCounts = new Map<string, number>();
      for (const id of parsed.egg_deck) {
        eggCounts.set(id, (eggCounts.get(id) ?? 0) + 1);
      }

      // Imported text decks don't carry alt-art info; treat every slot
      // as base art.  Users can re-pick alt-art printings in the grid.
      const mainEntries = Array.from(mainCounts.entries()).map(([cardId, count]) => ({
        cardId,
        isAltArt: false,
        count,
        cardData: cardDataMap.get(cardId) ?? undefined,
      }));
      const eggEntries = Array.from(eggCounts.entries()).map(([cardId, count]) => ({
        cardId,
        isAltArt: false,
        count,
        cardData: cardDataMap.get(cardId) ?? undefined,
      }));

      loadDeck(null, deckName, mainEntries, eggEntries);
      setIsDirty(true); // Imported deck hasn't been saved yet

      // Show warnings if any, otherwise close immediately
      if (parsed.warnings?.length) {
        setWarnings(parsed.warnings);
        setImportSuccess(true);
      } else {
        onClose();
      }
    } catch {
      setError('Failed to parse deck. Check the format and try again.');
    } finally {
      setIsImporting(false);
    }
  };

  const handleExport = () => {
    const lines: string[] = [];
    for (const entry of eggDeck) {
      lines.push(`${entry.count} ${entry.cardId} ${entry.cardData?.name ?? ''}`);
    }
    for (const entry of mainDeck) {
      lines.push(`${entry.count} ${entry.cardId} ${entry.cardData?.name ?? ''}`);
    }
    const content = lines.join('\n');
    navigator.clipboard.writeText(content);
    setText(content);
  };

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="Import / Export Deck">
      <div className="space-y-3">
        <textarea
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder="Paste deck list here (TTS JSON or text format)..."
          className="w-full h-48 px-3 py-2 bg-gray-700 border border-gray-600 rounded text-sm text-gray-100 font-mono resize-none focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
        {error && <p className="text-red-400 text-sm">{error}</p>}
        {importSuccess && warnings.length > 0 && (
          <div className="space-y-1">
            <p className="text-green-400 text-sm font-medium">Deck imported successfully!</p>
            <div className="text-amber-400 text-xs space-y-0.5">
              {warnings.map((w, i) => (
                <p key={i}>{w}</p>
              ))}
            </div>
          </div>
        )}
        <div className="flex gap-2">
          {importSuccess ? (
            <button
              onClick={onClose}
              className="px-4 py-2 bg-green-600 hover:bg-green-500 text-white rounded text-sm"
            >
              Done
            </button>
          ) : (
            <>
              <button
                onClick={handleImport}
                disabled={isImporting || !text.trim()}
                className="px-4 py-2 bg-blue-600 hover:bg-blue-500 disabled:bg-blue-800 text-white rounded text-sm"
              >
                {isImporting ? 'Importing...' : 'Import'}
              </button>
              <button
                onClick={handleExport}
                className="px-4 py-2 bg-gray-600 hover:bg-gray-500 text-white rounded text-sm"
              >
                Export to Clipboard
              </button>
            </>
          )}
        </div>
      </div>
    </Modal>
  );
}
