import json
import os
import importlib
from typing import Dict, Optional, Any
from ..core.entity_base import CEntity_Base
from ..core.card_script import CardScript
from ..core.card_source import CardSource
from .enums import CardColor, CardKind, Rarity
from .evo_cost import EvoCost

class CardDatabase:
    _instance = None

    def __new__(cls):
        if cls._instance is None:
            cls._instance = super(CardDatabase, cls).__new__(cls)
            cls._instance.initialized = False
        return cls._instance

    def __init__(self):
        if self.initialized:
            return
        self.cards: Dict[str, CEntity_Base] = {}
        self.scripts: Dict[str, CardScript] = {}
        self.load_cards()
        self.initialized = True

    def load_cards(self):
        module_dir = os.path.dirname(__file__)
        json_path = os.path.join(module_dir, 'cards.json')

        if not os.path.exists(json_path):
            print(f"Warning: {json_path} not found.")
            return

        with open(json_path, 'r', encoding='utf-8') as f:
            data = json.load(f)

        for entry in data:
            entity = CEntity_Base()
            entity.card_id = entry.get('card_id', '')
            entity.card_index = entry.get('card_index', 0)
            entity.card_name_eng = entry.get('card_name_eng', '')
            entity.card_name_jpn = entry.get('card_name_jpn', '')
            entity.type_eng = entry.get('type_eng', [])
            entity.type_jpn = entry.get('type_jpn', [])
            entity.card_effect_class_name = entry.get('card_effect_class_name', '')
            entity.play_cost = entry.get('play_cost', 0)
            entity.dp = entry.get('dp', 0)
            entity.level = entry.get('level', 0)
            entity.max_count_in_deck = entry.get('max_count_in_deck', 4)
            entity.card_sprite_name = entry.get('card_sprite_name', '')
            entity.effect_description_eng = entry.get('effect_description_eng', '')
            entity.inherited_effect_description_eng = entry.get('inherited_effect_description_eng', '')
            entity.security_effect_description_eng = entry.get('security_effect_description_eng', '')

            # Evo costs
            for ec in entry.get('evo_costs', []):
                entity.evo_costs.append(EvoCost(
                    card_color=CardColor(ec.get('card_color', 0)),
                    level=ec.get('level', 0),
                    memory_cost=ec.get('memory_cost', 0),
                ))

            # Form and attribute
            entity.form_eng = entry.get('form_eng', [])
            entity.attribute_eng = entry.get('attribute_eng', [])

            # Enums
            entity.card_kind = CardKind(entry.get('card_kind', 0))
            entity.rarity = Rarity(entry.get('rarity', 0))
            entity.card_colors = [CardColor(c) for c in entry.get('card_colors', [])]

            # Load Script
            if entity.card_effect_class_name:
                self._load_script(entity)

            self.cards[entity.card_id] = entity

        print(f"Loaded {len(self.cards)} cards.")

    def _load_script(self, entity: CEntity_Base):
        script_name = entity.card_effect_class_name
        # Assume set_id is the part before the first underscore or hyphen
        parts = script_name.split('_')
        if len(parts) < 2:
             parts = script_name.split('-')

        if len(parts) >= 1:
             set_id = parts[0].lower()
             module_name = script_name.lower()

             # Try multiple package prefixes (python_impl and digimon_gym)
             prefixes = ["python_impl.engine.data.scripts", "digimon_gym.engine.data.scripts"]
             loaded = False

             for prefix in prefixes:
                 module_path = f"{prefix}.{set_id}.{module_name}"
                 try:
                     module = importlib.import_module(module_path)
                     script_class = getattr(module, script_name)
                     self.scripts[entity.card_id] = script_class()
                     loaded = True
                     break
                 except (ImportError, AttributeError):
                     # Try without set folder
                     try:
                         module_path = f"{prefix}.{module_name}"
                         module = importlib.import_module(module_path)
                         script_class = getattr(module, script_name)
                         self.scripts[entity.card_id] = script_class()
                         loaded = True
                         break
                     except (ImportError, AttributeError):
                         continue

             if not loaded:
                 pass  # No script found for this card (vanilla cards with no effects)

    def get_card(self, card_id: str) -> Optional[CEntity_Base]:
        return self.cards.get(card_id)

    def get_script(self, card_id: str) -> Optional[CardScript]:
        return self.scripts.get(card_id)

    def get_all_cards(self) -> Dict[str, CEntity_Base]:
        return self.cards

    def create_card_source(self, card_id: str, owner=None) -> Optional[CardSource]:
        entity = self.get_card(card_id)
        if not entity:
            print(f"Card ID {card_id} not found.")
            return None

        card_source = CardSource()
        card_source.set_base_data(entity, owner)
        return card_source

if __name__ == "__main__":
    db = CardDatabase()
    print("Database contents:", list(db.get_all_cards().keys()))
