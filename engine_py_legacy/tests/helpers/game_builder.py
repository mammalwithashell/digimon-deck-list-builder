"""Fluent builder for setting up specific board states for testing."""

from __future__ import annotations
from typing import List, Optional

from digimon_gym.engine.game import Game
from digimon_gym.engine.loggers import EventLogger
from digimon_gym.engine.data.enums import GamePhase, CardKind, CardColor
from digimon_gym.engine.core.permanent import Permanent
from digimon_gym.engine.core.card_source import CardSource
from digimon_gym.engine.core.entity_base import CEntity_Base


def make_card(
    card_id: str = "TEST-001",
    name: str = "TestDigimon",
    kind: CardKind = CardKind.Digimon,
    dp: int = 5000,
    level: int = 4,
    play_cost: int = 5,
    colors: Optional[List[CardColor]] = None,
    owner=None,
) -> CardSource:
    """Create a CardSource with given attributes."""
    entity = CEntity_Base()
    entity.card_id = card_id
    entity.card_name_eng = name
    entity.card_kind = kind
    entity.dp = dp
    entity.level = level
    entity.play_cost = play_cost
    entity.card_colors = colors or [CardColor.Red]
    cs = CardSource()
    cs.set_base_data(entity, owner)
    return cs


def make_permanent(
    card_id: str = "TEST-001",
    name: str = "TestDigimon",
    dp: int = 5000,
    level: int = 4,
    **card_kwargs,
) -> Permanent:
    """Create a Permanent with a single card."""
    card = make_card(card_id=card_id, name=name, dp=dp, level=level, **card_kwargs)
    return Permanent([card])


class GameBuilder:
    """Fluent builder for setting up specific game board states."""

    def __init__(self):
        self._logger = EventLogger()
        self._p1_field: List[Permanent] = []
        self._p2_field: List[Permanent] = []
        self._p1_hand: List[CardSource] = []
        self._p2_hand: List[CardSource] = []
        self._p1_security: List[CardSource] = []
        self._p2_security: List[CardSource] = []
        self._memory: int = 0
        self._phase: GamePhase = GamePhase.Main
        self._turn_count: int = 1

    def with_player1_field(self, *perms: Permanent) -> GameBuilder:
        self._p1_field.extend(perms)
        return self

    def with_player2_field(self, *perms: Permanent) -> GameBuilder:
        self._p2_field.extend(perms)
        return self

    def with_player1_hand(self, *cards: CardSource) -> GameBuilder:
        self._p1_hand.extend(cards)
        return self

    def with_player2_hand(self, *cards: CardSource) -> GameBuilder:
        self._p2_hand.extend(cards)
        return self

    def with_player1_security(self, *cards: CardSource) -> GameBuilder:
        self._p1_security.extend(cards)
        return self

    def with_player2_security(self, *cards: CardSource) -> GameBuilder:
        self._p2_security.extend(cards)
        return self

    def with_memory(self, value: int) -> GameBuilder:
        self._memory = value
        return self

    def in_phase(self, phase: GamePhase) -> GameBuilder:
        self._phase = phase
        return self

    def with_turn_count(self, count: int) -> GameBuilder:
        self._turn_count = count
        return self

    def build(self) -> Game:
        game = Game(logger=self._logger)
        game.memory = self._memory
        game.current_phase = self._phase
        game.turn_count = self._turn_count

        # Set up player 1 field
        for perm in self._p1_field:
            game.player1.battle_area.append(perm)

        # Set up player 2 field
        for perm in self._p2_field:
            game.player2.battle_area.append(perm)

        # Set up hands
        game.player1.hand_cards.extend(self._p1_hand)
        game.player2.hand_cards.extend(self._p2_hand)

        # Set up security
        game.player1.security_cards.extend(self._p1_security)
        game.player2.security_cards.extend(self._p2_security)

        return game
