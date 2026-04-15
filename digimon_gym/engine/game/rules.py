"""Rules: frozen configuration for a game session.

Captures all configurable rule parameters (deck size, security count,
memory limits, banlist) so that Game instances can support different
game modes (Standard, NoRestriction, future EDH/Titan) without
hardcoding policy in the engine.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Optional

from ..data.enums import GameMode

if TYPE_CHECKING:
    from ..data.deck_loader import CardRestriction


@dataclass(frozen=True)
class Rules:
    """Immutable ruleset for a game session.

    All fields have sensible defaults matching Standard tournament rules.
    Use factory methods for common presets.
    """

    game_mode: GameMode = GameMode.Standard

    # Deck construction
    deck_size: int = 50
    egg_deck_max: int = 5
    max_copies_per_card: int = 4

    # Opening setup
    starting_security: int = 5
    starting_hand_size: int = 5

    # Memory system
    starting_memory: int = 0
    memory_gauge_limit: int = 10

    # Banlist (None = no restrictions)
    card_restrictions: Optional[CardRestriction] = field(default=None, repr=False)

    @classmethod
    def standard(cls) -> Rules:
        """Standard tournament rules with the official restricted list."""
        from ..data.deck_loader import RESTRICTED_LIST
        return cls(card_restrictions=RESTRICTED_LIST)

    @classmethod
    def no_banlist(cls) -> Rules:
        """Standard rules without banlist enforcement."""
        return cls(
            game_mode=GameMode.NoRestriction,
            card_restrictions=None,
        )

    @classmethod
    def training(cls) -> Rules:
        """RL training preset: no banlist, standard everything else."""
        return cls(
            game_mode=GameMode.NoRestriction,
            card_restrictions=None,
        )
