from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....interfaces.modifiers import ModifierType
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_038(CardScript):
    """EX9-038 Kuwagamon | Lv.4 | Insectoid/DM/Ver.4
    <Training>
    [On Play][When Attacking] By placing 1 hand card face down as bottom digi
    card, suspend 1 opponent Digimon. It can't unsuspend in their next unsuspend phase.
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
            if not player.hand_cards:
                return

            def on_hand_selected(selected_card):
                if selected_card is None:
                    return
                if selected_card in player.hand_cards:
                    player.hand_cards.remove(selected_card)
                    perm.add_card_source_bottom(selected_card)

                def target_filter(p):
                    return p.is_digimon

                def on_freeze(target_perm):
                    target_perm.suspend()
                    # Register CANNOT_UNSUSPEND modifier until opponent's next unsuspend phase
                    game.register_modifier(
                        perm,
                        ModifierType.CANNOT_UNSUSPEND,
                        condition=lambda target, ctx_: target is target_perm,
                        source_effect=None,
                        expiry='end_of_opponent_turn',
                    )

                game.effect_select_opponent_permanent(
                    player, on_freeze, filter_fn=target_filter, is_optional=False,
                    prompt="Suspend 1 opponent Digimon (can't unsuspend).")

            game.effect_select_hand_card(
                player, lambda c: True, on_hand_selected, is_optional=True,
                prompt="Select 1 card to place face down as bottom digivolution card.")

        # [On Play]
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX9-038 On Play: Place hand, freeze opponent")
        effect1.set_effect_description(
            "[On Play] By placing 1 card in your hand face down as this "
            "Digimon's bottom digivolution card, suspend 1 of your opponent's "
            "Digimon. It can't unsuspend in their next unsuspend phase."
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

        # [When Attacking]
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnTappedAnyone)
        effect2.set_effect_name("EX9-038 When Attacking: Place hand, freeze opponent")
        effect2.set_effect_description(
            "[When Attacking] By placing 1 card in your hand face down as this "
            "Digimon's bottom digivolution card, suspend 1 of your opponent's "
            "Digimon. It can't unsuspend in their next unsuspend phase."
        )
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
