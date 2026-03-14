from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_038(CardScript):
    """EX9-038 Kuwagamon | Lv.4 | Insectoid/DM/Ver.4
    <Training>
    [On Play][When Attacking] By placing 1 hand card as bottom digi card,
    suspend 1 opponent Digimon. It can't unsuspend in their next unsuspend phase.
    Inherited: <Piercing>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # <Training>
        effect0 = ICardEffect()
        effect0.set_effect_name("EX9-038 Training")
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
                player, on_freeze, filter_fn=target_filter, is_optional=False)

        # [On Play]
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX9-038 On Play: Place hand, freeze opponent")
        effect1.set_effect_description("[On Play] Place hand card, suspend+freeze.")
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

        # [When Attacking]
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnTappedAnyone)
        effect2.set_effect_name("EX9-038 When Attacking: Place hand, freeze opponent")
        effect2.set_effect_description("[When Attacking] Place hand card, suspend+freeze.")
        effect2.is_on_attack = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner and len(card.owner.hand_cards) < 1:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_place_and_freeze)
        effects.append(effect2)

        # Inherited: <Piercing>
        effect3 = ICardEffect()
        effect3.set_effect_name("EX9-038 Inherited: Piercing")
        effect3.set_effect_description("Inherited: <Piercing>")
        effect3.is_inherited_effect = True
        effect3._is_piercing = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
