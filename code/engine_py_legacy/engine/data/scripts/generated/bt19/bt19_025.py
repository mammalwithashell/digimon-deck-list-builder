from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_025(CardScript):
    """BT19-025 MetalGreymon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: save
        # Save
        effect0 = ICardEffect()
        effect0.set_effect_name("BT19-025 Save")
        effect0.set_effect_description("Save")
        effect0._is_save = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: material_save
        # Material Save
        effect1 = ICardEffect()
        effect1.set_effect_name("BT19-025 Material Save")
        effect1.set_effect_description("Material Save")
        effect1._is_material_save = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] This Digimon gains <Rush> for the turn.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT19-025 This Digimon gains <Rush>")
        effect2.set_effect_description("[On Play] This Digimon gains <Rush> for the turn.")
        effect2.is_on_play = True
        effect2._is_rush = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain Keyword Rush"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.grant_keyword('_is_rush')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] <De-Digivolve 1> 1 of your opponent's Digimon. Then, this Digimon may digivolve into a Digimon card with the [Blue Flare] trait from under your Tamers without paying the cost.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT19-025 <De-Digivolve 1> and digivolve into a [Blue Flare] card")
        effect3.set_effect_description("[When Attacking] <De-Digivolve 1> 1 of your opponent's Digimon. Then, this Digimon may digivolve into a Digimon card with the [Blue Flare] trait from under your Tamers without paying the cost.")
        effect3.is_on_attack = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Card, De Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.None
        # Effect
        effect4 = ICardEffect()
        effect4.set_effect_name("BT19-025 Effect")
        effect4.set_effect_description("Effect")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEndAttack
        # [End of Attack] (Once Per Turn) You may play 1 level 4 or lower Digimon card with the [Blue Flare] trait from under any of your Tamers without paying the cost.
        effect5 = ICardEffect()
        effect5.set_effect_name("BT19-025 Play 1 level 4 or lower [Blue Flare] Digimon")
        effect5.set_effect_description("[End of Attack] (Once Per Turn) You may play 1 level 4 or lower Digimon card with the [Blue Flare] trait from under any of your Tamers without paying the cost.")
        effect5.is_inherited_effect = True
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("PlayDigimon_BT19_025")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
