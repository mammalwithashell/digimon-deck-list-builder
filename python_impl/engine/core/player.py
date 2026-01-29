from __future__ import annotations
from typing import TYPE_CHECKING, List

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

    @property
    def is_lose(self) -> bool:
        return False

    def draw(self):
        pass
