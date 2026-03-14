from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_051(CardScript):
    """EX9-051 Monochromon | Lv.4 | Ankylosaur/DM/Ver.4
    <Training>
    [On Play][When Attacking] By placing 1 hand card as bottom digi card,
    De-Digivolve 1 opponent Digimon.
    Inherited: <Blocker>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # <Training>
        effect0 = ICardEffect()
        effect0.set_effect_name("EX9-051 Training")
        effect0.set_effect_description("<Training>")
        effect0._is_training = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        def _place_and_dedigivolve(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            if player.hand_cards:
                placed = player.hand_cards.pop()
                perm.add_card_source_bottom(placed)

            def target_filter(p):
                return p.is_digimon

            def on_dedigivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy.trash_cards.extend(removed)

            game.effect_select_opponent_permanent(
                player, on_dedigivolve, filter_fn=target_filter, is_optional=False)

        # [On Play]
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX9-051 On Play: Place hand, De-Digivolve 1")
        effect1.set_effect_description("[On Play] Place hand card, De-Digivolve 1.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner and len(card.owner.hand_cards) < 1:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_place_and_dedigivolve)
        effects.append(effect1)

        # [When Attacking]
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnTappedAnyone)
        effect2.set_effect_name("EX9-051 When Attacking: Place hand, De-Digivolve 1")
        effect2.set_effect_description("[When Attacking] Place hand card, De-Digivolve 1.")
        effect2.is_on_attack = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner and len(card.owner.hand_cards) < 1:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_place_and_dedigivolve)
        effects.append(effect2)

        # Inherited: <Blocker>
        effect3 = ICardEffect()
        effect3.set_effect_name("EX9-051 Inherited: Blocker")
        effect3.set_effect_description("Inherited: <Blocker>")
        effect3.is_inherited_effect = True
        effect3._is_blocker = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
