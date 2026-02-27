from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_058(CardScript):
    """BT14-058 Numemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _can_pay_satsuki_cost(ctx: Dict[str, Any]) -> bool:
            player = ctx.get('player')
            if not player:
                return False
            hand = getattr(player, 'hand', None) or []
            for c in hand:
                if getattr(c, 'card_id', '') == 'BT14-084' or getattr(c, 'name', '') == 'Satsuki Tamahime':
                    return True
            return False

        def _pay_and_grant_rush(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            hand = getattr(player, 'hand', None) or []
            satsuki = None
            for c in hand:
                if getattr(c, 'card_id', '') == 'BT14-084' or getattr(c, 'name', '') == 'Satsuki Tamahime':
                    satsuki = c
                    break
            if satsuki is None:
                return

            # Optional cost: place 1 Satsuki Tamahime from hand as bottom digivolution card.
            if hasattr(game, 'move_card_to_bottom_digivolution'):
                game.move_card_to_bottom_digivolution(player, satsuki, perm)
            elif hasattr(perm, 'add_digivolution_source_bottom'):
                if hasattr(player, 'remove_from_hand'):
                    player.remove_from_hand(satsuki)
                perm.add_digivolution_source_bottom(satsuki)
            else:
                return

            def target_filter(p):
                return p.is_digimon

            def on_grant(target_perm):
                target_perm.grant_keyword('_is_rush')

            game.effect_select_own_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=False
            )

        # [On Play]
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-058 On Play: place Satsuki to sources, 1 of your Digimon gains Rush")
        effect0.set_effect_description("[On Play] By placing 1 [Satsuki Tamahime] from your hand as this Digimon's bottom digivolution card, 1 of your Digimon gains <Rush> for the turn.")
        effect0.is_optional = True
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return _can_pay_satsuki_cost(context)

        effect0.set_can_use_condition(condition0)
        effect0.set_on_process_callback(_pay_and_grant_rush)
        effects.append(effect0)

        # [When Digivolving]
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-058 When Digivolving: place Satsuki to sources, 1 of your Digimon gains Rush")
        effect1.set_effect_description("[When Digivolving] By placing 1 [Satsuki Tamahime] from your hand as this Digimon's bottom digivolution card, 1 of your Digimon gains <Rush> for the turn.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return _can_pay_satsuki_cost(context)

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_pay_and_grant_rush)
        effects.append(effect1)

        # Inherited: <Blocker>
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-058 Blocker")
        effect2.set_effect_description("Blocker")
        effect2.is_inherited_effect = True
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
