import json
import os
import importlib
from typing import Dict, Optional, List
from ..core.entity_base import CEntity_Base
from ..core.card_script import CardScript
from .enums import CardColor, CardKind, Rarity, EffectTiming

class CardDatabase:
    _instance = None

    def __new__(cls):
        if cls._instance is None:
            cls._instance = super(CardDatabase, cls).__new__(cls)
            cls._instance.cards = {}
            cls._instance.load_cards()
        return cls._instance

    def load_cards(self):
        # Determine the path to cards.json
        current_dir = os.path.dirname(os.path.abspath(__file__))
        json_path = os.path.join(current_dir, 'cards.json')

        try:
            with open(json_path, 'r', encoding='utf-8') as f:
                card_list = json.load(f)
                for card_data in card_list:
                    self.cards[card_data['id']] = card_data
            print(f"Loaded {len(self.cards)} cards from {json_path}")
        except FileNotFoundError:
            print(f"Error: cards.json not found at {json_path}")
        except json.JSONDecodeError as e:
            print(f"Error decoding cards.json: {e}")

    def get_card_data(self, card_id: str) -> Optional[Dict[str, Any]]:
        return self.cards.get(card_id)

    def get_all_cards(self) -> Dict[str, Any]:
        return self.cards

if __name__ == "__main__":
    db = CardDatabase()
    print("Database contents:", db.get_all_cards().keys())
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
            entity.card_effect_class_name = entry.get('card_effect_class_name', '')
            entity.play_cost = entry.get('play_cost', 0)
            entity.dp = entry.get('dp', 0)
            entity.level = entry.get('level', 0)
            entity.max_count_in_deck = entry.get('max_count_in_deck', 4)
            entity.card_sprite_name = entry.get('card_sprite_name', '')
            entity.effect_description_eng = entry.get('effect_description_eng', '')
            entity.inherited_effect_description_eng = entry.get('inherited_effect_description_eng', '')
            entity.security_effect_description_eng = entry.get('security_effect_description_eng', '')

            # Enums
            entity.card_kind = CardKind(entry.get('card_kind', 0))
            entity.rarity = Rarity(entry.get('rarity', 0))
            entity.card_colors = [CardColor(c) for c in entry.get('card_colors', [])]

            # Load Script
            if entity.card_effect_class_name:
                self._load_script(entity)

            self.cards[entity.card_id] = entity

    def _load_script(self, entity: CEntity_Base):
        script_name = entity.card_effect_class_name
        # Assume set_id is the part before the first underscore or hyphen, but usually ST1_01 -> ST1
        # script_name is usually "ST1_01" (class name)
        # We need to handle potential naming conventions.
        # If class name is "ST1_01", we assume file is python_impl.engine.data.scripts.st1.st1_01

        parts = script_name.split('_')
        if len(parts) < 2:
             # Fallback or different naming
             parts = script_name.split('-')

        if len(parts) >= 1:
             set_id = parts[0].lower()
             module_name = script_name.lower()

             module_path = f"python_impl.engine.data.scripts.{set_id}.{module_name}"

             try:
                 module = importlib.import_module(module_path)
                 script_class = getattr(module, script_name)
                 self.scripts[entity.card_id] = script_class()
             except ImportError:
                 # Try ignoring set folder
                 try:
                     module_path = f"python_impl.engine.data.scripts.{module_name}"
                     module = importlib.import_module(module_path)
                     script_class = getattr(module, script_name)
                     self.scripts[entity.card_id] = script_class()
                 except ImportError as e:
                     print(f"Could not load script for {entity.card_id} ({script_name}): {e}")
             except AttributeError as e:
                 print(f"Could not find class {script_name} in {module_path}: {e}")

    def get_card(self, card_id: str) -> Optional[CEntity_Base]:
        return self.cards.get(card_id)

    def get_script(self, card_id: str) -> Optional[CardScript]:
        return self.scripts.get(card_id)
