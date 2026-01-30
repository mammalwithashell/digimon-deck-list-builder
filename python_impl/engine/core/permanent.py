from __future__ import annotations
from typing import TYPE_CHECKING, List, Optional
from ..data.enums import EffectTiming

if TYPE_CHECKING:
    from .card_source import CardSource
    from ..interfaces.card_effect import ICardEffect

class Permanent:
    def __init__(self, card_sources: List['CardSource']):
        self.card_sources: List['CardSource'] = card_sources
        self.is_suspended: bool = False
        self.dp_modifier: int = 0
        self.cannot_unsuspend_until_turn_end: bool = False

    @property
    def digivolution_cards(self) -> List['CardSource']:
        return self.card_sources

    @property
    def top_card(self) -> Optional['CardSource']:
        if len(self.card_sources) > 0:
            return self.card_sources[-1]
        return None

    @property
    def level(self) -> int:
        return self.top_card.level if self.top_card else 0

    @property
    def is_token(self) -> bool:
        return self.top_card.is_token if self.top_card else False

    @property
    def is_digimon(self) -> bool:
        return self.top_card.is_digimon if self.top_card else False

    @property
    def is_tamer(self) -> bool:
        return self.top_card.is_tamer if self.top_card else False

    @property
    def is_option(self) -> bool:
        return self.top_card.is_option if self.top_card else False

    @property
    def dp(self) -> int:
        base = self.top_card.base_dp if self.top_card else 0
        return base + self.dp_modifier

    def can_attack(self, card_effect: Optional['ICardEffect'], without_tap: bool = False, is_vortex: bool = False) -> bool:
        return not self.is_suspended or without_tap

    def can_block(self, attacking_permanent: 'Permanent') -> bool:
        return not self.is_suspended and self.has_blocker() # Placeholder for blocker check

    def has_blocker(self) -> bool:
        # Check traits or effects for Blocker
        return False

    def effect_list(self, timing: EffectTiming) -> List['ICardEffect']:
        # This would aggregate effects from sources
        return []

    def add_card_source(self, card_source: 'CardSource'):
        self.card_sources.append(card_source)

    def suspend(self):
        self.is_suspended = True
        print(f"Permanent suspended.")

    def unsuspend(self):
        if not self.cannot_unsuspend_until_turn_end:
            self.is_suspended = False
            print(f"Permanent unsuspended.")

    def de_digivolve(self, amount: int):
        trashed = []
        # Can't trash level 3 or below (usually level 3 is the lowest digimon, but Level 2 is egg).
        # Standard rule: "You can't trash past level 3 cards".
        # This usually means if the top card is Level 3, stop? Or if the resulting card would be lower than 3?
        # Actually, it means you trash from top, but you stop if the *next* card to become top is not a valid Digimon or is Level 2 (Egg)?
        # The rule is "Trash up to X cards... You can't trash past level 3 cards."
        # Meaning you cannot trash a Level 3 card to reveal a Level 2.

        for _ in range(amount):
            if len(self.card_sources) <= 1:
                break # Don't trash the last card (usually)

            top = self.top_card
            if top and top.level == 3:
                break # Stop at level 3

            # Additional check: If removing top would leave an egg?
            # Simplified: Just pop.
            trashed.append(self.card_sources.pop())

        print(f"De-digivolved {len(trashed)} cards.")
        return trashed

    def discard_evo_roots(self, ignore_overflow: bool = False, put_to_trash: bool = True):
        # Generator placeholder
        pass
