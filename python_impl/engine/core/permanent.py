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
        base = self.top_card.base_dp if self.top_card else 0

        # Calculate active effects
        # Currently, we don't have a global EffectTiming.Continuous or similar for passive buffs.
        # But we can iterate all sources and check for inherited/self effects that satisfy condition.
        # This is a simplified check.
        active_effects = self.get_active_effects()
        modifier = sum(effect.dp_modifier for effect in active_effects)

        return max(0, base + modifier)

    def get_active_effects(self) -> List['ICardEffect']:
        # Gather effects from all sources in stack
        # Check conditions (requires context, usually passed or available globally)
        # For PoC, assuming context availability or simplified check inside condition.
        active = []

        # Inherited effects from sources UNDER top card
        for i, source in enumerate(self.card_sources[:-1]):
            # Inherited effects
            effects = source.effect_list(EffectTiming.NoTiming) # Or specific timing? Scripts set is_inherited_effect
            # Note: Scripts usually return effects regardless of timing, we filter properties.
            # But effect_list currently calls get_card_effects which returns a LIST.
            # We need to iterate that list.
            for effect in effects:
                if effect.is_inherited_effect:
                    # Check condition
                    # Condition requires 'context'. For continuous effects, context is essentially "Now".
                    # We pass 'self' as permanent context.
                    # We need a proper context dict.
                    ctx = {"permanent": self}
                    if effect.can_use_condition and effect.can_use_condition(ctx):
                        active.append(effect)

        # Effects from Top Card (not inherited, unless specified otherwise)
        if self.top_card:
            effects = self.top_card.effect_list(EffectTiming.NoTiming)
            for effect in effects:
                if not effect.is_inherited_effect: # Normal effects
                     # Check if it's a continuous effect (has no specific trigger timing usually, or we flag it)
                     # For DP buff, it's usually just there.
                     # Using dp_modifier > 0 as proxy for "Continuous Stat Buff" if timing is generic.
                     if effect.dp_modifier != 0:
                        ctx = {"permanent": self}
                        if effect.can_use_condition and effect.can_use_condition(ctx):
                            active.append(effect)

        return active

    def can_attack(self, card_effect: Optional['ICardEffect'], without_tap: bool = False, is_vortex: bool = False) -> bool:
        if self.is_suspended and not without_tap:
            return False

        # Check summoning sickness (not tracked yet)

        return True

    def can_block(self, attacking_permanent: 'Permanent') -> bool:
        # Check for Blocker effect and not suspended
        # For now, return False unless we implement Blocker trait check
        return False

    def effect_list(self, timing: EffectTiming) -> List['ICardEffect']:
        # Aggregate effects from all sources for a specific timing
        # This is used for "OnAttack", "WhenDigivolving" etc.
        effects = []
        # Sources (Inherited)
        for source in self.card_sources[:-1]:
            source_effects = source.effect_list(timing)
            for eff in source_effects:
                if eff.is_inherited_effect:
                    effects.append(eff)

        # Top Card (Not Inherited)
        if self.top_card:
            top_effects = self.top_card.effect_list(timing)
            for eff in top_effects:
                if not eff.is_inherited_effect:
                    effects.append(eff)

        return effects

    def add_card_source(self, card_source: 'CardSource'):
        self.card_sources.append(card_source)

    def discard_evo_roots(self, ignore_overflow: bool = False, put_to_trash: bool = True):
        # Generator placeholder
        pass

    def suspend(self):
        self.is_suspended = True

    def unsuspend(self):
        self.is_suspended = False
