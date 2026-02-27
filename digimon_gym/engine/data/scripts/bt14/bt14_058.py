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

        def _has_satsuki_in_hand(player) -> bool:
            hand = getattr(player, 'hand', None)
            if not hand:
                return False
            for c in hand:
                cid = getattr(c, 'card_id', '')
                name = getattr(c, 'card_name', '')
                if cid == 'BT14-084' or name == 'Satsuki Tamahime':
                    return True
            return False

        def _place_satsuki_as_bottom_evo(player, perm) -> bool:
            if not (player and perm):
                return False
            hand = getattr(player, 'hand', None)
            if not hand:
                return False
            target = None
            for c in hand:
                cid = getattr(c, 'card_id', '')
                name = getattr(c, 'card_name', '')
                if cid == 'BT14-084' or name == 'Satsuki Tamahime':
                    target = c
                    break
            if target is None:
                return False

            if hasattr(player, 'remove_card_from_hand'):
                player.remove_card_from_hand(target)
            else:
                hand.remove(target)

            if hasattr(perm, 'add_bottom_digivolution_card'):
                perm.add_bottom_digivolution_card(target)
            elif hasattr(perm, 'add_evolution_base_bottom'):
                perm.add_evolution_base_bottom(target)
            elif hasattr(perm, 'digivolution_cards'):
                perm.digivolution_cards.insert(0, target)
            else:
                return False
            return True

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By placing 1 [Satsuki Tamahime] from your hand as this Digimon's bottom digivolution card, 1 of your Digimon gains <Rush> for the turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-058 Place 1 card to digivolution cards and your 1 Digimon gains Rush")
        effect0.set_effect_description("[On Play] By placing 1 [Satsuki Tamahime] from your hand as this Digimon's bottom digivolution card, 1 of your Digimon gains <Rush> for the turn.")
        effect0.is_optional = True
        effect0.is_on_play = True
        effect0._is_rush = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = context.get('player')
            return _has_satsuki_in_hand(player)

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: pay by placing Satsuki, then grant Rush"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return
            if not _place_satsuki_as_bottom_evo(player, perm):
                return

            def target_filter(p):
                return p.is_digimon

            def on_grant(target_perm):
                target_perm.grant_keyword('_is_rush')

            game.effect_select_own_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By placing 1 [Satsuki Tamahime] from your hand as this Digimon's bottom digivolution card, 1 of your Digimon gains <Rush> for the turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-058 Place 1 card to digivolution cards and your 1 Digimon gains Rush")
        effect1.set_effect_description("[When Digivolving] By placing 1 [Satsuki Tamahime] from your hand as this Digimon's bottom digivolution card, 1 of your Digimon gains <Rush> for the turn.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True
        effect1._is_rush = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = context.get('player')
            return _has_satsuki_in_hand(player)

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: pay by placing Satsuki, then grant Rush"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return
            if not _place_satsuki_as_bottom_evo(player, perm):
                return

            def target_filter(p):
                return p.is_digimon

            def on_grant(target_perm):
                target_perm.grant_keyword('_is_rush')

            game.effect_select_own_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: blocker
        # Blocker
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
