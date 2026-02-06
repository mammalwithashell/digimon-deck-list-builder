"""BaseGameRunner: shared deck setup and game initialization for all game modes.

Mirrors C# Digimon.Core.BaseGameRunner.
"""

from __future__ import annotations
from abc import ABC
from typing import List, Optional

try:
    from python_impl.engine.game import Game
    from python_impl.engine.loggers import IGameLogger, SilentLogger
    from python_impl.engine.data.card_database import CardDatabase
    from python_impl.engine.data.card_registry import CardRegistry
except ImportError:
    from digimon_gym.engine.game import Game
    from digimon_gym.engine.loggers import IGameLogger, SilentLogger
    from digimon_gym.engine.data.card_database import CardDatabase
    from digimon_gym.engine.data.card_registry import CardRegistry


class BaseGameRunner(ABC):
    """Abstract base for game runners. Handles deck setup and game creation."""

    def __init__(self, deck1_ids: List[str], deck2_ids: List[str],
                 logger: Optional[IGameLogger] = None):
        self.logger: IGameLogger = logger if logger is not None else SilentLogger()
        self.game = Game(self.logger)

        # Ensure card registry is initialized
        CardRegistry.ensure_initialized()

        # Setup decks from card ID lists
        self._setup_deck(self.game.player1, deck1_ids)
        self._setup_deck(self.game.player2, deck2_ids)

        # Start the game (shuffles, draws security + hand, parks at Breeding)
        self.game.start_game()

    @staticmethod
    def _setup_deck(player, card_ids: List[str]):
        """Populate a player's library and digitama library from card IDs.

        DigiEgg cards go to digitama_library_cards, everything else to library_cards.
        """
        db = CardDatabase()
        for card_id in card_ids:
            card_source = db.create_card_source(card_id, player)
            if card_source is None:
                continue
            if card_source.is_digi_egg:
                player.digitama_library_cards.append(card_source)
            else:
                player.library_cards.append(card_source)

    @property
    def is_game_over(self) -> bool:
        return self.game.game_over

    @property
    def winner_id(self) -> Optional[int]:
        return self.game.winner.player_id if self.game.winner else None
