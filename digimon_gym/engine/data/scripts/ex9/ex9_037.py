from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_037(CardScript):
    """EX9-037 Kabuterimon | Lv.4 | Insectoid/DM/Ver.2
    <Training>
    [On Play][When Digivolving] By placing 1 hand card as bottom digi card,
    suspend 1 opponent Digimon. It can't unsuspend in their next unsuspend phase.
    Inherited: [When Attacking][Once Per Turn] Suspend 1 opponent Digimon.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # <Training>
        effect0 = ICardEffect()
        effect0.set_effect_name("EX9-037 Training")
        effect0.set_effect_description("<Training>")
        effect0._is_training = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        def _place_and_freeze(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            if player.hand_cards:
                placed = player.hand_cards.pop()
                perm.add_card_source_bottom(placed)

            def target_filter(p):
                return p.is_digimon

            def on_freeze(target_perm):
                target_perm.suspend()
                target_perm._skip_unsuspend = True

            game.effect_select_opponent_permanent(
                player, on_freeze, filter_fn=target_filter, is_optional=False,
                prompt="Suspend 1 opponent Digimon (can't unsuspend).")

        # [On Play]
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX9-037 On Play: Place hand, freeze opponent")
        effect1.set_effect_description(
            "[On Play] Place 1 hand card as bottom digi card, suspend+freeze opponent."
        )
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner and len(card.owner.hand_cards) < 1:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_place_and_freeze)
        effects.append(effect1)

        # [When Digivolving]
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX9-037 When Digivolving: Place hand, freeze opponent")
        effect2.set_effect_description(
            "[When Digivolving] Place 1 hand card as bottom digi card, suspend+freeze."
        )
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner and len(card.owner.hand_cards) < 1:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_place_and_freeze)
        effects.append(effect2)

        # Inherited: [When Attacking][Once Per Turn] Suspend 1 opponent
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnTappedAnyone)
        effect3.set_effect_name("EX9-037 Inherited: When Attacking suspend opponent")
        effect3.set_effect_description(
            "Inherited: [When Attacking][Once Per Turn] Suspend 1 opponent Digimon."
        )
        effect3.is_inherited_effect = True
        effect3.is_on_attack = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("suspend_EX9_037_inh")

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return p.is_digimon

            def on_suspend(target_perm):
                target_perm.suspend()

            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)
        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
