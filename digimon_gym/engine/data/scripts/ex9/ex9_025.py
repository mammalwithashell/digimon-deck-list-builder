from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_025(CardScript):
    """EX9-025 Airdramon | Lv.4 | Mythical Beast/DM/Ver.1
    <Training>
    [When Attacking][Once Per Turn] By placing deck's top card face down as
    bottom digi card, give -2000 DP per face-down digi card to 1 opponent Digimon.
    Inherited: <Barrier>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # <Training>
        effect0 = ICardEffect()
        effect0.set_effect_name("EX9-025 Training")
        effect0.set_effect_description("<Training>")
        effect0._is_training = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [When Attacking][Once Per Turn] Place deck top, give -DP
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnTappedAnyone)
        effect1.set_effect_name("EX9-025 When Attacking: Place deck top, -DP to opponent")
        effect1.set_effect_description(
            "[When Attacking][Once Per Turn] Place deck's top card as bottom "
            "digi card, give -2000 DP per face-down digi card to opponent."
        )
        effect1.is_on_attack = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("place_dp_minus_EX9_025")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner and not card.owner.library_cards:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            if player.library_cards:
                placed = player.library_cards.pop(0)
                perm.add_card_source_bottom(placed)
            face_down_count = len(perm.digivolution_cards)
            dp_reduction = -2000 * face_down_count

            def target_filter(p):
                return p.is_digimon

            def on_reduce_dp(target_perm):
                target_perm.dp_modifier += dp_reduction

            game.effect_select_opponent_permanent(
                player, on_reduce_dp, filter_fn=target_filter, is_optional=False,
                prompt=f"Select opponent Digimon to give {dp_reduction} DP.")
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Inherited: <Barrier>
        effect2 = ICardEffect()
        effect2.set_effect_name("EX9-025 Inherited: Barrier")
        effect2.set_effect_description("Inherited: <Barrier>")
        effect2.is_inherited_effect = True
        effect2._is_barrier = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
