import json
import os
from typing import Dict, Any, Optional
from python_impl.engine.core.entity_base import CEntity_Base
from python_impl.engine.core.card_source import CardSource
from python_impl.engine.data.enums import CardColor, CardKind

class CardDatabase:
    _instance = None

    def __new__(cls):
        if cls._instance is None:
            cls._instance = super(CardDatabase, cls).__new__(cls)
            cls._instance.cards: Dict[str, CEntity_Base] = {}
        return cls._instance

    def load_cards(self, filepath: str):
        if not os.path.exists(filepath):
            print(f"Error: Cards file {filepath} not found.")
            return

        with open(filepath, 'r') as f:
            data = json.load(f)

        for card_data in data:
            entity = CEntity_Base()
            entity.card_id = card_data.get("card_id", "")
            entity.card_name_eng = card_data.get("card_name_eng", "")

            kind_str = card_data.get("card_kind", "Digimon")
            if kind_str in CardKind.__members__:
                entity.card_kind = CardKind[kind_str]
            else:
                print(f"Warning: Unknown CardKind {kind_str} for {entity.card_id}")

            colors = card_data.get("card_colors", [])
            for c in colors:
                if c in CardColor.__members__:
                    entity.card_colors.append(CardColor[c])

            entity.level = card_data.get("level", 0)
            entity.play_cost = card_data.get("play_cost", 0)
            entity.dp = card_data.get("dp", 0)

            self.cards[entity.card_id] = entity

        print(f"Loaded {len(self.cards)} cards from {filepath}")

    def get_card_entity(self, card_id: str) -> Optional[CEntity_Base]:
        return self.cards.get(card_id)

    def create_card_source(self, card_id: str, owner=None) -> Optional[CardSource]:
        entity = self.get_card_entity(card_id)
        if not entity:
            print(f"Card ID {card_id} not found.")
            return None

        card_source = CardSource()
        card_source.set_base_data(entity, owner)
        return card_source
