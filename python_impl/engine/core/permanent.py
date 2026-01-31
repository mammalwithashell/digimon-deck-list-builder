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
        if not self.top_card:
            return 0

        base = self.top_card.base_dp
        mod = 0

        # Calculate DP modifications from effects
        # Using NoTiming for continuous/static effects
        for effect in self.effect_list(EffectTiming.NoTiming):
            if not effect.is_disabled:
                mod += effect.get_change_dp_value(self)

        return max(0, base + mod)

    def can_attack(self, card_effect: Optional['ICardEffect'], without_tap: bool = False, is_vortex: bool = False) -> bool:
        return True

    def can_block(self, attacking_permanent: 'Permanent') -> bool:
        return False

    def effect_list(self, timing: EffectTiming) -> List['ICardEffect']:
        effects = []

        # Top card effects
        if self.top_card:
            top_effects = self.top_card.effect_list(timing)
            for eff in top_effects:
                if not eff.is_inherited_effect:
                    effects.append(eff)

        # Inherited effects from sources
        for card in self.card_sources:
            if card == self.top_card:
                continue

            source_effects = card.effect_list(timing)
            for eff in source_effects:
                if eff.is_inherited_effect:
                    effects.append(eff)

        # Set the source permanent for all collected effects
        for eff in effects:
            eff.set_effect_source_permanent(self)

        return effects

    def add_card_source(self, card_source: 'CardSource'):
        self.card_sources.append(card_source)

    def discard_evo_roots(self, ignore_overflow: bool = False, put_to_trash: bool = True):
        # Generator placeholder
        pass

    def suspend(self):
        self.is_suspended = True
