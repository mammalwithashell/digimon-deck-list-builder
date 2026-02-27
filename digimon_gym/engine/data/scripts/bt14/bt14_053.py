from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_053(CardScript):
    """BT14-053 Rosemon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [When Digivolving] Suspend 1 of your opponent's Digimon or Tamers.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-053 Suspend 1 Digimon or Tamer")
        effect0.set_effect_description("[When Digivolving] Suspend 1 of your opponent's Digimon or Tamers.")
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            return not (card and card.permanent_of_this_card() is None)

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return getattr(p, 'is_tamer', False) or getattr(p, 'is_digimon', False)

            def on_suspend(target_perm):
                target_perm.suspend()

            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [When Attacking] Suspend 1 of your opponent's Digimon or Tamers.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-053 Suspend 1 Digimon or Tamer")
        effect1.set_effect_description("[When Attacking] Suspend 1 of your opponent's Digimon or Tamers.")
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            return not (card and card.permanent_of_this_card() is None)

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return getattr(p, 'is_tamer', False) or getattr(p, 'is_digimon', False)

            def on_suspend(target_perm):
                target_perm.suspend()

            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [Your Turn][Once Per Turn] When an effect suspends a Digimon or Tamer, you may unsuspend this Digimon.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-053 Unsuspend this Digimon")
        effect2.set_effect_description("[Your Turn][Once Per Turn] When an effect suspends a Digimon or Tamer, you may unsuspend this Digimon.")
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Unsuspend_BT14_053")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False

            suspended_perm = context.get('target_permanent') or context.get('suspended_permanent')
            if suspended_perm is None:
                return False
            return getattr(suspended_perm, 'is_tamer', False) or getattr(suspended_perm, 'is_digimon', False)

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            perm = ctx.get('permanent')
            if perm:
                perm.unsuspend()

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
