from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

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

        def process0(ctx: Dict[str, Any]):
            """Action: Jogress Condition"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DNA/Jogress digivolution condition — handled by engine
            pass

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: raid
        # Raid
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-045 Raid")
        effect1.set_effect_description("Raid")
        effect1._is_raid = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: blocker
        # Blocker
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-045 Blocker")
        effect2.set_effect_description("Blocker")
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: evade
        # Evade
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-045 Evade")
        effect3.set_effect_description("Evade")
        effect3._is_evade = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If DNA digivolving, return all of your opponent's Digimon with the highest DP to the bottom of the deck.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT20-045 Return opponent's Digimon with the highest DP to the bottom of deck")
        effect4.set_effect_description("[When Digivolving] If DNA digivolving, return all of your opponent's Digimon with the highest DP to the bottom of the deck.")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.OnTappedAnyone
        # [Your Turn] (Once Per Turn) When any Digimon suspend, this Digimon may unsuspend.
        effect5 = ICardEffect()
        effect5.set_effect_name("BT20-045 Unsuspend this Digimon")
        effect5.set_effect_description("[Your Turn] (Once Per Turn) When any Digimon suspend, this Digimon may unsuspend.")
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("Unsuspend_BT20_045")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Unsuspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=True)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
