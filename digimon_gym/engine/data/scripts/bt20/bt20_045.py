from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_045(CardScript):
    """BT20-045 Examon | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Jogress Condition
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-045 Jogress Condition")
        effect0.set_effect_description("Jogress Condition")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blast_dna_digivolve
        # Blast DNA Digivolve
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-045 Blast DNA Digivolve")
        effect1.set_effect_description("Blast DNA Digivolve")
        effect1.is_counter_effect = True
        effect1._is_blast_dna_digivolve = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: raid
        # Raid
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-045 Raid")
        effect2.set_effect_description("Raid")
        effect2.is_on_attack = True
        effect2._is_raid = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: blocker
        # Blocker
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-045 Blocker")
        effect3.set_effect_description("Blocker")
        effect3._is_blocker = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Factory effect: evade
        # Evade
        effect4 = ICardEffect()
        effect4.set_effect_name("BT20-045 Evade")
        effect4.set_effect_description("Evade")
        effect4._is_evade = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If DNA digivolving, return all of your opponent's Digimon with the highest DP to the bottom of the deck.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect5.set_effect_name("BT20-045 Return opponent's Digimon with the highest DP to the bottom of deck")
        effect5.set_effect_description("[When Digivolving] If DNA digivolving, return all of your opponent's Digimon with the highest DP to the bottom of the deck.")
        effect5.is_when_digivolving = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Return all opponent's highest DP Digimon to deck bottom (DNA only)."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            # Check if this was a DNA digivolution (has 2+ sources from different permanents)
            if perm and len(perm.card_sources) < 3:
                # Not a DNA digivolve (DNA typically has sources from 2 permanents)
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
            if not opp_digimon:
                return
            max_dp = max(p.dp for p in opp_digimon)
            highest = [p for p in opp_digimon if p.dp == max_dp]
            for target in highest:
                if target in enemy.battle_area:
                    enemy.return_permanent_to_deck_bottom(target)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.OnTappedAnyone
        # [Your Turn] (Once Per Turn) When any Digimon suspend, this Digimon may unsuspend.
        effect6 = ICardEffect()
        effect6.set_timing(EffectTiming.OnTappedAnyone)
        effect6.set_effect_name("BT20-045 Unsuspend this Digimon")
        effect6.set_effect_description("[Your Turn] (Once Per Turn) When any Digimon suspend, this Digimon may unsuspend.")
        effect6.is_optional = True
        effect6.set_max_count_per_turn(1)
        effect6.set_hash_string("Unsuspend_BT20_045")

        effect = effect6  # alias for condition closure
        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # The suspended permanent must be a Digimon (any player's)
            ctx_perm = context.get('permanent')
            if not ctx_perm or not ctx_perm.is_digimon:
                return False
            # Must not be THIS Digimon (it says "when any Digimon suspend" but
            # the unsuspend only makes sense if it's another Digimon)
            my_perm = card.permanent_of_this_card()
            if my_perm and ctx_perm is my_perm:
                return False
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            """Action: Unsuspend this Digimon"""
            my_perm = card.permanent_of_this_card() if card else None
            if my_perm and my_perm.is_suspended:
                my_perm.unsuspend()

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        return effects
