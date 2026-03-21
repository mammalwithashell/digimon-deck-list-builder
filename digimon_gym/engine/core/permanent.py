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
        self.turn_played: int = -1  # Turn this permanent entered the field (-1 = not tracked)
        self.turn_digivolved: int = -1  # Turn this permanent last digivolved (-1 = never)
        self._owner_game: Optional[object] = None  # Back-reference to Game for turn tracking
        self._granted_keywords: dict = {}  # keyword_attr -> expiry_turn (or -1 for permanent)
        self.is_attacking: bool = False  # True while this permanent is the attacker in combat
        self.attack_count_this_turn: int = 0  # Number of attacks declared this turn
        self._temp_sa_modifier: int = 0  # Temporary Security Attack modifier (e.g. from Alliance)
        self._granted_effects: List[tuple] = []  # [(ICardEffect, expiry_turn)] granted by other cards

    @property
    def owner(self):
        """Return the Player who owns this permanent (via top card)."""
        top = self.top_card
        return top.owner if top else None

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
    def level(self) -> Optional[int]:
        """Level of the top card. None for tamers/options and some Digimon (e.g. Eater Bit)."""
        return self.top_card.level if self.top_card else None

    @property
    def is_digi_egg(self) -> bool:
        """True if the top card is a Digi-Egg (Lv.2)."""
        return self.top_card.is_digi_egg if self.top_card else False

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
    def has_dp(self) -> bool:
        """True if this permanent has a DP value (Digimon top card). False for eggs/tamers."""
        return self.top_card.has_dp if self.top_card else False

    @property
    def dp(self) -> Optional[int]:
        """DP of this permanent. None if the top card has no DP (egg/tamer/option).
        For Digimon, returns base DP + effect modifiers + aura modifiers (minimum 0).
        Cards like Lucemon: Larva have 0 DP (a real value, not None).

        <Progress> immunity: while attacking with Progress, negative DP modifiers
        from opponent effects are ignored (both pre-existing and newly applied).
        """
        if not self.top_card or self.top_card.base_dp is None:
            return None
        base = self.top_card.base_dp
        active_effects = self.get_active_effects()
        modifier = sum(effect.dp_modifier for effect in active_effects)
        aura_modifier = self._get_aura_dp_modifier()
        if self.is_immune_to_opponent_effects:
            # Progress: ignore all negative temp DP modifiers while attacking
            temp_modifier = sum(m for m in self._dp_modifiers if m >= 0)
        else:
            temp_modifier = sum(self._dp_modifiers)
        computed = max(0, base + modifier + aura_modifier + temp_modifier)
        # Apply DP floor if any (e.g. "DP cannot be reduced below X")
        owner = self.owner
        if owner and hasattr(owner, 'game') and owner.game:
            from ..interfaces.modifiers import ModifierType
            floor = owner.game.modifiers.get_int_modifier(
                self, ModifierType.DP_FLOOR, 0)
            if floor > 0:
                computed = max(floor, computed)
        return computed

    def _get_aura_dp_modifier(self) -> int:
        """Sum DP modifiers from aura effects on other friendly permanents.

        Scans the owner's battle_area for effects with _applies_to_all_own_digimon=True
        and dp_modifier != 0. This supports cards like Tamers that grant "+X000 DP to
        all your [Trait] Digimon" as a continuous effect.
        """
        if not self.is_digimon or not self.top_card or not self.top_card.owner:
            return 0
        owner = self.top_card.owner
        if not hasattr(owner, 'battle_area'):
            return 0
        total = 0
        for other_perm in owner.battle_area:
            if other_perm is self:
                continue
            # Inherited effects from sources under top card
            for source in other_perm.card_sources[:-1]:
                for effect in source.effect_list(EffectTiming.NoTiming):
                    if not effect.is_inherited_effect:
                        continue
                    if not getattr(effect, '_applies_to_all_own_digimon', False):
                        continue
                    if effect.dp_modifier == 0:
                        continue
                    ctx = {"permanent": self}
                    if effect.can_use_condition and not effect.can_use_condition(ctx):
                        continue
                    perm_filter = getattr(effect, '_dp_permanent_condition', None)
                    if perm_filter and not perm_filter(self):
                        continue
                    total += effect.dp_modifier
            # Non-inherited effects from top card
            if other_perm.top_card:
                for effect in other_perm.top_card.effect_list(EffectTiming.NoTiming):
                    if effect.is_inherited_effect:
                        continue
                    if not getattr(effect, '_applies_to_all_own_digimon', False):
                        continue
                    if effect.dp_modifier == 0:
                        continue
                    ctx = {"permanent": self}
                    if effect.can_use_condition and not effect.can_use_condition(ctx):
                        continue
                    perm_filter = getattr(effect, '_dp_permanent_condition', None)
                    if perm_filter and not perm_filter(self):
                        continue
                    total += effect.dp_modifier
        # Also scan security cards for DP aura effects
        # (e.g. option cards placed face-up in security that grant DP)
        if hasattr(owner, 'security_cards'):
            for sec_card in owner.security_cards:
                for effect in sec_card.effect_list(EffectTiming.NoTiming):
                    if effect.is_inherited_effect:
                        continue
                    if not getattr(effect, '_applies_to_all_own_digimon', False):
                        continue
                    if effect.dp_modifier == 0:
                        continue
                    ctx = {"permanent": self}
                    if effect.can_use_condition and not effect.can_use_condition(ctx):
                        continue
                    perm_filter = getattr(effect, '_dp_permanent_condition', None)
                    if perm_filter and not perm_filter(self):
                        continue
                    total += effect.dp_modifier
        return total

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

    def has_keyword(self, keyword_attr: str) -> bool:
        """Check if this permanent has a keyword effect (e.g. '_is_rush', '_is_jamming').

        Scans inherited effects from sources under top card and
        non-inherited effects from the top card, matching the same
        pattern used in can_block() and effect_list().
        Also checks granted keywords from effects like CardEffectCommons.Gain*().
        """
        # Check granted keywords first (from effects that grant keywords to targets)
        if keyword_attr in self._granted_keywords:
            expiry = self._granted_keywords[keyword_attr]
            if expiry == -1:
                return True  # permanent grant
            if self._owner_game and self._owner_game.turn_count <= expiry:
                return True
            # Expired — clean up
            del self._granted_keywords[keyword_attr]

        def effect_is_active(effect, source_card) -> bool:
            if effect.can_use_condition is None:
                return True
            ctx = {
                "game": self._owner_game,
                "player": source_card.owner if source_card else None,
                "permanent": self,
            }
            return bool(effect.can_use_condition(ctx))

        for source in self.card_sources[:-1]:
            effects = source.effect_list(EffectTiming.NoTiming)
            for effect in effects:
                if (
                    effect.is_inherited_effect
                    and getattr(effect, keyword_attr, False)
                    and effect_is_active(effect, source)
                ):
                    return True
        if self.top_card:
            effects = self.top_card.effect_list(EffectTiming.NoTiming)
            for effect in effects:
                if (
                    not effect.is_inherited_effect
                    and getattr(effect, keyword_attr, False)
                    and effect_is_active(effect, self.top_card)
                ):
                    return True
        # Effects from linked option cards (non-inherited)
        for linked in self.linked_cards:
            effects = linked.effect_list(EffectTiming.NoTiming)
            for effect in effects:
                if (
                    not effect.is_inherited_effect
                    and getattr(effect, keyword_attr, False)
                    and effect_is_active(effect, linked)
                ):
                    return True
        # Aura keywords from other friendly permanents
        # (mirrors _get_aura_dp_modifier pattern for keyword granting)
        if self.is_digimon and self.top_card and self.top_card.owner:
            owner = self.top_card.owner
            if hasattr(owner, 'battle_area'):
                for other_perm in owner.battle_area:
                    if other_perm is self:
                        continue
                    # Check non-inherited effects from other permanent's top card
                    if other_perm.top_card:
                        for effect in other_perm.top_card.effect_list(EffectTiming.NoTiming):
                            if effect.is_inherited_effect:
                                continue
                            if not getattr(effect, '_applies_to_all_own_digimon', False):
                                continue
                            if not getattr(effect, keyword_attr, False):
                                continue
                            ctx = {"permanent": self}
                            if effect.can_use_condition and not effect.can_use_condition(ctx):
                                continue
                            perm_filter = getattr(effect, '_keyword_permanent_condition', None)
                            if perm_filter and not perm_filter(self):
                                continue
                            return True
                    # Check linked option cards on other permanents
                    for linked in other_perm.linked_cards:
                        for effect in linked.effect_list(EffectTiming.NoTiming):
                            if effect.is_inherited_effect:
                                continue
                            if not getattr(effect, '_applies_to_all_own_digimon', False):
                                continue
                            if not getattr(effect, keyword_attr, False):
                                continue
                            ctx = {"permanent": self}
                            if effect.can_use_condition and not effect.can_use_condition(ctx):
                                continue
                            perm_filter = getattr(effect, '_keyword_permanent_condition', None)
                            if perm_filter and not perm_filter(self):
                                continue
                            return True
            # Also scan security cards for aura keyword effects
            # (e.g. option cards placed face-up in security that grant keywords)
            if hasattr(owner, 'security_cards'):
                for sec_card in owner.security_cards:
                    for effect in sec_card.effect_list(EffectTiming.NoTiming):
                        if effect.is_inherited_effect:
                            continue
                        if not getattr(effect, '_applies_to_all_own_digimon', False):
                            continue
                        if not getattr(effect, keyword_attr, False):
                            continue
                        ctx = {"permanent": self}
                        if effect.can_use_condition and not effect.can_use_condition(ctx):
                            continue
                        perm_filter = getattr(effect, '_keyword_permanent_condition', None)
                        if perm_filter and not perm_filter(self):
                            continue
                        return True
        return False

    def grant_keyword(self, keyword_attr: str, duration: int = -1):
        """Grant a keyword to this permanent.

        Args:
            keyword_attr: The keyword attribute name (e.g. '_is_rush', '_is_cannot_attack').
            duration: Number of turns the grant lasts. -1 = permanent (until removed).
                      Positive values are absolute turn number of expiry.
        """
        self._granted_keywords[keyword_attr] = duration

    def clear_expired_grants(self, current_turn: int):
        """Remove expired keyword grants."""
        expired = [k for k, v in self._granted_keywords.items()
                   if v != -1 and current_turn > v]
        for k in expired:
            del self._granted_keywords[k]

    def security_attack_modifier(self) -> int:
        """Sum of all <Security Attack +/-X> modifiers on this permanent."""
        total = 0
        for source in self.card_sources[:-1]:
            effects = source.effect_list(EffectTiming.NoTiming)
            for effect in effects:
                if effect.is_inherited_effect:
                    total += getattr(effect, '_security_attack_modifier', 0)
        if self.top_card:
            effects = self.top_card.effect_list(EffectTiming.NoTiming)
            for effect in effects:
                if not effect.is_inherited_effect:
                    total += getattr(effect, '_security_attack_modifier', 0)
        # Linked option cards (non-inherited)
        for linked in self.linked_cards:
            effects = linked.effect_list(EffectTiming.NoTiming)
            for effect in effects:
                if not effect.is_inherited_effect:
                    total += getattr(effect, '_security_attack_modifier', 0)
        # Temporary modifier (e.g. from Alliance)
        total += self._temp_sa_modifier
        # Query modifier registry for CHANGE_SECURITY_ATTACK modifiers
        if self._owner_game and hasattr(self._owner_game, 'modifiers'):
            from ..interfaces.modifiers import ModifierType
            total = self._owner_game.modifiers.get_int_modifier(
                self, ModifierType.CHANGE_SECURITY_ATTACK, total)
        return total

    def can_attack(self, card_effect: Optional['ICardEffect'] = None, without_tap: bool = False, is_vortex: bool = False) -> bool:
        if self.is_suspended and not without_tap:
            return False
        if not self.is_digimon:
            return False
        # Restriction: cannot attack
        if self.has_keyword('_is_cannot_attack'):
            return False
        # Restriction: cannot suspend (attacking requires suspending)
        if not without_tap and self._owner_game and hasattr(self._owner_game, 'modifiers'):
            from ..interfaces.modifiers import ModifierType
            if self._owner_game.modifiers.has_modifier(self, ModifierType.CANNOT_SUSPEND):
                return False
        # Restriction: cannot attack player (partial — checked separately for target filtering)
        # Summoning sickness: can't attack the turn played, unless has <Rush> or <Vortex>
        if self.turn_played >= 0 and self._owner_game is not None:
            if self.turn_played == self._owner_game.turn_count and not self.has_keyword('_is_rush') and not is_vortex:
                return False
        return True

    def can_attack_player(self) -> bool:
        """Check if this permanent can attack the player directly.
        Returns False if restricted by <cannot attack player> or CANNOT_ATTACK_PLAYER modifier."""
        if self.has_keyword('_is_cannot_attack_player'):
            return False
        if self._owner_game and hasattr(self._owner_game, 'modifiers'):
            from ..interfaces.modifiers import ModifierType
            if self._owner_game.modifiers.has_modifier(self, ModifierType.CANNOT_ATTACK_PLAYER):
                return False
        return True

    def can_block(self, attacking_permanent: 'Permanent') -> bool:
        """Check if this permanent has <Blocker> and can block the attack.

        Requires: unsuspended, is a Digimon, has _is_blocker effect.
        Also checks:
        - <cannot block> restriction on this blocker
        - <cannot suspend> restriction (blocking requires suspending)
        - <cannot be blocked> on the attacker
        - <Collision> on the attacker (all Digimon gain Blocker, skip _is_blocker check)
        """
        if self.is_suspended:
            return False
        if not self.is_digimon:
            return False
        # Restriction: this Digimon cannot block
        if self.has_keyword('_is_cannot_block'):
            return False
        # Restriction: cannot suspend (blocking requires suspending)
        if self._owner_game and hasattr(self._owner_game, 'modifiers'):
            from ..interfaces.modifiers import ModifierType
            if self._owner_game.modifiers.has_modifier(self, ModifierType.CANNOT_SUSPEND):
                return False
        # Restriction: attacker cannot be blocked
        if attacking_permanent.has_keyword('_is_cannot_be_blocked'):
            return False

        # <Collision>: all opponent Digimon gain Blocker while the attacker is attacking
        if attacking_permanent.has_keyword('_is_collision'):
            return True

        # Standard Blocker check: scan all card sources for _is_blocker flag
        return self.has_keyword('_is_blocker')

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
        # Granted temporary effects from other cards
        for eff, _expiry in self._granted_effects:
            if eff.timing == timing:
                effects.append(eff)
        return effects

    def grant_temp_effect(self, effect: 'ICardEffect', expiry_turn: int = -1):
        """Grant a temporary triggered effect to this permanent.

        Args:
            effect: The ICardEffect to attach.
            expiry_turn: Turn number when effect expires (-1 = permanent).
        """
        self._granted_effects.append((effect, expiry_turn))

    def clear_expired_effects(self, current_turn: int):
        """Remove granted effects whose expiry_turn has passed."""
        self._granted_effects = [
            (eff, exp) for eff, exp in self._granted_effects
            if exp == -1 or exp > current_turn
        ]

    def remove_granted_effects_by_source(self, source_perm: 'Permanent'):
        """Remove all granted effects that were sourced from a specific permanent."""
        self._granted_effects = [
            (eff, exp) for eff, exp in self._granted_effects
            if getattr(eff, '_source_permanent', None) is not source_perm
        ]

    def add_card_source(self, card_source: 'CardSource'):
        self.card_sources.append(card_source)
        self._fire_timing(EffectTiming.OnAddDigivolutionCards, {"permanent": self, "added_card": card_source})

    def add_card_source_bottom(self, card_source: 'CardSource'):
        """Add a card source at the bottom of the digivolution stack."""
        self.card_sources.insert(0, card_source)
        self._fire_timing(EffectTiming.OnAddDigivolutionCards, {"permanent": self, "added_card": card_source})

    # ─── Effect Action Methods ───────────────────────────────────────

    def change_dp(self, amount: int):
        """Apply a temporary DP modifier (lasts until end of turn).

        <Progress> immunity: negative DP modifiers from opponent effects are
        blocked entirely while this permanent is attacking with Progress.
        """
        if amount < 0 and self.is_immune_to_opponent_effects:
            return  # Progress blocks opponent DP debuffs while attacking
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
        if removed:
            self._fire_timing(EffectTiming.WhenTopCardTrashed,
                              {"permanent": self, "trashed_cards": removed})
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
        if trashed:
            self._fire_timing(EffectTiming.OnDigivolutionCardDiscarded,
                              {"permanent": self, "trashed_cards": trashed})
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
           0.0  — source has no once-per-turn effects OR all exhausted
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
            return 0.0
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
        self._fire_timing(EffectTiming.WhenLinked, {"permanent": self, "linked_card": card})

    def unlink_all(self) -> List['CardSource']:
        """Remove all linked cards and return them (for when permanent leaves field)."""
        cards = list(self.linked_cards)
        self.linked_cards.clear()
        return cards

    def _fire_timing(self, timing: 'EffectTiming', context: dict = None):
        """Fire an effect timing via the owner game if available."""
        if self._owner_game and hasattr(self._owner_game, 'execute_effects'):
            self._owner_game.execute_effects(timing, context)

    def suspend(self):
        if not self.is_suspended:
            self.is_suspended = True
            self._fire_timing(EffectTiming.OnTappedAnyone, {"permanent": self})
            # Fire cross-permanent suspend observers
            if self._owner_game and hasattr(self._owner_game, '_fire_suspend_observers'):
                self._owner_game._fire_suspend_observers(self)

    def unsuspend(self):
        if self.is_suspended:
            # Check CANNOT_UNSUSPEND modifier (aura-style, covers new permanents)
            if self._owner_game and hasattr(self._owner_game, 'modifiers'):
                from ..interfaces.modifiers import ModifierType
                if self._owner_game.modifiers.has_modifier(self, ModifierType.CANNOT_UNSUSPEND):
                    return  # blocked by modifier
            self.is_suspended = False
            self._fire_timing(EffectTiming.OnUnTappedAnyone, {"permanent": self})

    def clear_attack_state(self):
        """Clear temporary attack state (is_attacking flag and temp SA modifier)."""
        self.is_attacking = False
        self._temp_sa_modifier = 0

    @property
    def is_immune_to_opponent_effects(self) -> bool:
        """True if this permanent has <Progress> and is currently attacking."""
        return self.is_attacking and self.has_keyword('_is_progress')
