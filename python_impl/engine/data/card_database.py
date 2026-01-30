import json
import os
from typing import Dict, Any, Optional

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
