from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....interfaces.modifiers import ModifierType
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_013(CardScript):
    """BT16-013 Valkyrimon | Lv.6

    [Hand] [Counter] Blast Digivolve
    [On Play] [When Digivolving] All of your opponent's Digimon get
    -5000 DP for the turn.
    [All Turns] [Once Per Turn] When a card is removed from a security
    stack, delete 1 of your opponent's 8000 DP or lower Digimon. If this
    effect didn't delete, this Digimon gains Security A. +1 until the end
    of your turn.
    Inherited: Ace Overflow -4
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Blast Digivolve
        effect0 = ICardEffect()
        effect0.set_effect_name("BT16-013 Blast Digivolve")
        effect0.set_effect_description("Blast Digivolve")
        effect0.is_counter_effect = True
        effect0._is_blast_digivolve = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Alt digi: from Silphymon for cost 3
        effect1 = ICardEffect()
        effect1.set_effect_name("BT16-013 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution requirement")
        effect1._alt_digi_cost = 3
        effect1._alt_digi_name = "Silphymon"

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        def _dp_minus_all(ctx: Dict[str, Any]):
            """All opponent Digimon get -5000 DP for the turn."""
            player = ctx.get('player')
            if not player:
                return
            enemy = player.enemy
            if not enemy:
                return
            for perm in enemy.battle_area:
                if perm.is_digimon:
                    perm.change_dp(-5000)

        # [On Play] All opponent's Digimon get -5000 DP
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT16-013 -5000 DP to all opponent Digimon")
        effect2.set_effect_description(
            "[On Play] All of your opponent's Digimon get -5000 DP for the turn."
        )
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_dp_minus_all)
        effects.append(effect2)

        # [When Digivolving] All opponent's Digimon get -5000 DP
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT16-013 -5000 DP to all opponent Digimon")
        effect3.set_effect_description(
            "[When Digivolving] All of your opponent's Digimon get -5000 DP for the turn."
        )
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_dp_minus_all)
        effects.append(effect3)

        # [All Turns] [Once Per Turn] When a card is removed from a security
        # stack, delete 1 opponent Digimon with 8000 DP or lower. If didn't
        # delete, gain SA+1 until end of your turn.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnLoseSecurity)
        effect4.set_effect_name("BT16-013 Delete 8000 DP or less, else SA+1")
        effect4.set_effect_description(
            "[All Turns] [Once Per Turn] When a card is removed from a security "
            "stack, delete 1 of your opponent's 8000 DP or lower Digimon. If "
            "this effect didn't delete, this Digimon gains Security A. +1 "
            "until the end of your turn."
        )
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("Delete8000DPorLess_BT16_013")

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            enemy = player.enemy
            if not enemy:
                return

            # Find opponent Digimon with 8000 DP or less
            targets = [
                p for p in enemy.battle_area
                if p.is_digimon and (game.modifiers.get_int_modifier(
                    p, ModifierType.CHANGE_DP, p.base_dp) <= 8000)
            ]

            if targets:
                def on_delete(target_perm):
                    enemy2 = player.enemy if player else None
                    if enemy2:
                        enemy2.delete_permanent(target_perm)

                def target_filter(p):
                    return p.is_digimon and (game.modifiers.get_int_modifier(
                        p, ModifierType.CHANGE_DP, p.base_dp) <= 8000)

                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=target_filter, is_optional=False)
            else:
                # Didn't delete: this Digimon gains SA+1 until end of your turn
                if perm:
                    perm._temp_sa_modifier += 1

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Inherited: Ace Overflow -4
        effect5 = ICardEffect()
        effect5.set_effect_name("BT16-013 Ace Overflow -4")
        effect5.set_effect_description("Ace Overflow -4")
        effect5.is_inherited_effect = True
        effect5._is_ace_overflow = True
        effect5._overflow_amount = -4

        def condition5(context: Dict[str, Any]) -> bool:
            return True
        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
