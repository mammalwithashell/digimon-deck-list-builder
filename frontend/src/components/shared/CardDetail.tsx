import { useEffect, useState } from 'react';
import { Card } from './Card';
import { getCardById } from '@/api/digimonCardApi';
import { useDeckBuilderStore } from '@/stores/deckBuilderStore';
import type { DigimonCardData } from '@/types/cards';

export function CardDetail() {
  const { selectedCardId, searchResults, addCardToDeck } = useDeckBuilderStore();
  const [card, setCard] = useState<DigimonCardData | null>(null);

  useEffect(() => {
    if (!selectedCardId) {
      setCard(null);
      return;
    }
    // Try search results first
    const cached = searchResults.find((c) => c.cardnumber === selectedCardId);
    if (cached) {
      setCard(cached);
      return;
    }
    // Fetch from API
    getCardById(selectedCardId).then(setCard).catch(() => setCard(null));
  }, [selectedCardId, searchResults]);

  if (!card) {
    return (
      <div className="text-center text-gray-500 text-sm p-4">
        Select a card to view details
      </div>
    );
  }

  return (
    <div className="flex gap-3 p-2">
      <Card cardId={card.cardnumber} cardName={card.name} size="xl" />
      <div className="flex-1 space-y-2 min-w-0">
        <h3 className="text-sm font-bold text-gray-100">{card.name}</h3>
        <div className="text-xs text-gray-400">{card.cardnumber}</div>

        <div className="grid grid-cols-2 gap-x-3 gap-y-1 text-xs">
          {card.type && <Detail label="Type" value={card.type} />}
          {card.color && <Detail label="Color" value={card.color} />}
          {card.level && <Detail label="Level" value={card.level} />}
          {card.dp && <Detail label="DP" value={card.dp} />}
          {card.play_cost && <Detail label="Cost" value={card.play_cost} />}
          {card.evolution_cost && <Detail label="Evo Cost" value={card.evolution_cost} />}
          {card.stage && <Detail label="Form" value={card.stage} />}
          {card.attribute && <Detail label="Attribute" value={card.attribute} />}
          {card.digi_type && <Detail label="Digi-Type" value={card.digi_type} />}
          {card.cardrarity && <Detail label="Rarity" value={card.cardrarity} />}
        </div>

        {card.maineffect && (
          <div>
            <div className="text-[10px] text-gray-500 uppercase">Effect</div>
            <div className="text-xs text-gray-300 leading-relaxed">{card.maineffect}</div>
          </div>
        )}

        {card.soureeffect && (
          <div>
            <div className="text-[10px] text-gray-500 uppercase">Inherited Effect</div>
            <div className="text-xs text-gray-300 leading-relaxed">{card.soureeffect}</div>
          </div>
        )}

        <button
          onClick={() => addCardToDeck(card.cardnumber, card)}
          className="mt-2 px-3 py-1 bg-green-600 hover:bg-green-500 text-white text-sm rounded"
        >
          Add to Deck
        </button>
      </div>
    </div>
  );
}

function Detail({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <span className="text-gray-500">{label}: </span>
      <span className="text-gray-300">{value}</span>
    </div>
  );
}
