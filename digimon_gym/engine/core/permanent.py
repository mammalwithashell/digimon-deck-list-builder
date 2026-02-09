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
        self._dp_modifiers: List[int] = []  # Temporary DP changes from effects
        self.linked_cards: List['CardSource'] = []  # Option cards linked sideways (e.g. [TS])

    @property
    def digivolution_cards(self) -> List['CardSource']:
        return self.card_sources

    @property
    def has_no_digivolution_cards(self) -> bool:
        return len(self.card_sources) <= 1

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
        active_effects = self.get_active_effects()
        modifier = sum(effect.dp_modifier for effect in active_effects)
        temp_modifier = sum(self._dp_modifiers)
        return max(0, base + modifier + temp_modifier)

    def get_active_effects(self) -> List['ICardEffect']:
        active = []

        # Inherited effects from sources UNDER top card
        for source in self.card_sources[:-1]:
            effects = source.effect_list(EffectTiming.NoTiming)
            for effect in effects:
                if effect.is_inherited_effect:
                    ctx = {"permanent": self}
                    if effect.can_use_condition and effect.can_use_condition(ctx):
                        active.append(effect)

        # Effects from Top Card (not inherited)
        if self.top_card:
            effects = self.top_card.effect_list(EffectTiming.NoTiming)
            for effect in effects:
                if not effect.is_inherited_effect:
                    if effect.dp_modifier != 0:
                        ctx = {"permanent": self}
                        if effect.can_use_condition and effect.can_use_condition(ctx):
                            active.append(effect)

        return active

    def can_attack(self, card_effect: Optional['ICardEffect'] = None, without_tap: bool = False, is_vortex: bool = False) -> bool:
        if self.is_suspended and not without_tap:
            return False
        return True

    def can_block(self, attacking_permanent: 'Permanent') -> bool:
        """Check if this permanent has <Blocker> and can block the attack.

        Requires: unsuspended, is a Digimon, has _is_blocker effect.
        Scans inherited effects from sources and non-inherited from top card,
        matching the pattern in effect_list().
        """
        if self.is_suspended:
            return False
        if not self.is_digimon:
            return False

        # Check all card sources for _is_blocker flag
        for source in self.card_sources[:-1]:
            # Inherited effects from under top card
            effects = source.effect_list(EffectTiming.NoTiming)
            for effect in effects:
                if effect.is_inherited_effect and getattr(effect, '_is_blocker', False):
                    return True

        if self.top_card:
            # Non-inherited effects from top card
            effects = self.top_card.effect_list(EffectTiming.NoTiming)
            for effect in effects:
                if not effect.is_inherited_effect and getattr(effect, '_is_blocker', False):
                    return True

        return False

    def effect_list(self, timing: EffectTiming) -> List['ICardEffect']:
        effects = []
        for source in self.card_sources[:-1]:
            source_effects = source.effect_list(timing)
            for eff in source_effects:
                if eff.is_inherited_effect:
                    effects.append(eff)
        if self.top_card:
            top_effects = self.top_card.effect_list(timing)
            for eff in top_effects:
                if not eff.is_inherited_effect:
                    effects.append(eff)
        # Effects from linked option cards (not inherited)
        for linked in self.linked_cards:
            linked_effects = linked.effect_list(timing)
            for eff in linked_effects:
                if not eff.is_inherited_effect:
                    effects.append(eff)
        return effects

    def add_card_source(self, card_source: 'CardSource'):
        self.card_sources.append(card_source)

    # ─── Effect Action Methods ───────────────────────────────────────

    def change_dp(self, amount: int):
        """Apply a temporary DP modifier (lasts until end of turn)."""
        self._dp_modifiers.append(amount)

    def clear_temp_dp(self):
        """Clear temporary DP modifiers at end of turn."""
        self._dp_modifiers.clear()

    def de_digivolve(self, count: int = 1) -> List['CardSource']:
        """Remove top N digivolution cards (not the base) and send to trash.
        Returns the removed cards."""
        removed = []
        for _ in range(count):
            if len(self.card_sources) <= 1:
                break
            card = self.card_sources.pop()
            removed.append(card)
        # Caller is responsible for putting removed cards in trash
        return removed

    def trash_digivolution_cards(self, count: int, from_top: bool = True) -> List['CardSource']:
        """Trash N digivolution cards (from under the top card).
        Returns the trashed cards."""
        trashed = []
        for _ in range(count):
            if len(self.card_sources) <= 1:
                break
            if from_top:
                # Trash from just under top (index -2, -3, etc.)
                idx = len(self.card_sources) - 2
            else:
                # Trash from bottom
                idx = 0
            if idx >= 0:
                card = self.card_sources.pop(idx)
                trashed.append(card)
        return trashed

    def contains_card_name(self, name: str) -> bool:
        """Check if the top card's name contains the given string."""
        if self.top_card:
            for card_name in self.top_card.card_names:
                if name.lower() in card_name.lower():
                    return True
        return False

    def has_trait(self, trait: str) -> bool:
        """Check if the top card has a given trait."""
        if self.top_card:
            return trait in self.top_card.card_traits
        return False

    @property
    def opt_total(self) -> int:
        """Count of once-per-turn effects on this permanent (inherited + top + linked)."""
        count = 0
        for effect in self.effect_list(EffectTiming.NoTiming):
            if effect.max_count_per_turn > 0:
                count += 1
        return count

    @property
    def opt_used(self) -> int:
        """Count of once-per-turn effects that have been activated this turn."""
        count = 0
        for effect in self.effect_list(EffectTiming.NoTiming):
            if effect.max_count_per_turn > 0 and not effect.can_activate_this_turn():
                count += 1
        return count

    def source_opt_state(self, source: 'CardSource') -> float:
        """Return OPT availability state for a specific source card.

        Returns:
          -1.0  — source has no once-per-turn effects
           0.0  — all OPT effects exhausted this turn
           1.0  — all OPT effects still available
           0.0-1.0 — fraction available (e.g. 0.5 = 1 of 2 available)

        For inherited sources (under top card), only considers inherited effects.
        For the top card, considers non-inherited effects only.
        """
        is_under = source is not self.top_card
        total = 0
        available = 0
        for effect in source.effect_list(EffectTiming.NoTiming):
            if is_under and not effect.is_inherited_effect:
                continue
            if not is_under and effect.is_inherited_effect:
                continue
            if effect.max_count_per_turn > 0:
                total += 1
                if effect.can_activate_this_turn():
                    available += 1
        if total == 0:
            return -1.0
        return float(available) / float(total)

    def source_dp_contribution(self, source: 'CardSource') -> float:
        """Return the DP modifier this source currently contributes.

        For inherited sources (under top card): sums dp_modifier from active
        inherited effects whose can_use_condition passes right now (e.g. [Your Turn]
        effects return 0 on the opponent's turn).
        For the top card: sums dp_modifier from active non-inherited effects.
        """
        is_under = source is not self.top_card
        total_dp = 0
        ctx = {"permanent": self}
        for effect in source.effect_list(EffectTiming.NoTiming):
            if is_under and not effect.is_inherited_effect:
                continue
            if not is_under and effect.is_inherited_effect:
                continue
            if effect.dp_modifier != 0:
                if effect.can_use_condition and effect.can_use_condition(ctx):
                    total_dp += effect.dp_modifier
        return float(total_dp)

    def link_card(self, card: 'CardSource'):
        """Link an option card sideways to this permanent (e.g. [TS] options)."""
        self.linked_cards.append(card)

    def unlink_all(self) -> List['CardSource']:
        """Remove all linked cards and return them (for when permanent leaves field)."""
        cards = list(self.linked_cards)
        self.linked_cards.clear()
        return cards

    def suspend(self):
        self.is_suspended = True

    def unsuspend(self):
        self.is_suspended = False
