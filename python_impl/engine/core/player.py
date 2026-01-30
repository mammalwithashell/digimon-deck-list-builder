from __future__ import annotations
from typing import TYPE_CHECKING, List, Optional
import random
from .permanent import Permanent

if TYPE_CHECKING:
    from .card_source import CardSource

class Player:
    def __init__(self):
        self.player_id: int = 0
        self.player_name: str = ""
        self.is_my_turn: bool = False
        self.memory: int = 0
        self.hand_cards: List['CardSource'] = []
        self.library_cards: List['CardSource'] = []
        self.security_cards: List['CardSource'] = []
        self.trash_cards: List['CardSource'] = []
        self.digitama_library_cards: List['CardSource'] = []

        self.battle_area: List['Permanent'] = []
        self.breeding_area: Optional['Permanent'] = None

    @property
    def is_lose(self) -> bool:
        # Simple check: empty deck is not instant loss, loss happens when drawing from empty.
        # But we can flag it.
        return False

    def setup_game(self):
        # Shuffle decks
        random.shuffle(self.library_cards)
        random.shuffle(self.digitama_library_cards)

        # Set Security (top 5 cards of library)
        for _ in range(5):
            if self.library_cards:
                self.security_cards.append(self.library_cards.pop(0))

        # Draw initial hand (5 cards)
        for _ in range(5):
            self.draw()

    def draw(self):
        if not self.library_cards:
            print(f"{self.player_name} cannot draw! Deck empty.")
            return

        card = self.library_cards.pop(0)
        self.hand_cards.append(card)
        print(f"{self.player_name} drew a card. Hand size: {len(self.hand_cards)}")

    def hatch(self):
        if self.breeding_area is not None:
            print("Cannot hatch: Breeding area occupied.")
            return

        if not self.digitama_library_cards:
            print("Cannot hatch: Digitama deck empty.")
            return

        card = self.digitama_library_cards.pop(0)
        new_permanent = Permanent([card])
        self.breeding_area = new_permanent
        print(f"{self.player_name} hatched {card.card_names[0]}.")

    def play_card(self, card_source: 'CardSource'):
        if card_source in self.hand_cards:
            self.hand_cards.remove(card_source)
            new_permanent = Permanent([card_source])
            self.battle_area.append(new_permanent)
            print(f"{self.player_name} played {card_source.card_names[0]}.")

    def unsuspend_all(self):
        for perm in self.battle_area:
            perm.is_suspended = False
        if self.breeding_area:
            self.breeding_area.is_suspended = False
        print(f"{self.player_name} unsuspended all permanents.")
