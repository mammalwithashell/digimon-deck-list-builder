from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_023(CardScript):
    """BT14-023 Ikkakumon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [When Digivolving] Trash any 2 digivolution cards from your opponent's Digimon.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-023 Trash digivolution cards")
        effect0.set_effect_description("[When Digivolving] Trash any 2 digivolution cards from your opponent's Digimon.")
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return p.is_digimon and (not p.has_no_digivolution_cards)

            def on_trash(target_perm):
                target_perm.trash_digivolution_cards(2)

            game.effect_select_opponent_permanent(
                player, on_trash, filter_fn=target_filter, is_optional=False
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [When Attacking][Once Per Turn] Until the end of your opponent's turn,
        # 1 of your opponent's Digimon with as many or fewer digivolution cards as this Digimon can't attack.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-023 Opponent's 1 Digimon can't attack")
        effect1.set_effect_description("[When Attacking][Once Per Turn] Until the end of your opponent's turn, 1 of your opponent's Digimon with as many or fewer digivolution cards as this Digimon can't attack.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("CantAtatck_BT14_023")
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            my_evo_count = len(getattr(perm, 'digivolution_cards', []) or [])

            def target_filter(p):
                return p.is_digimon and len(getattr(p, 'digivolution_cards', []) or []) <= my_evo_count

            def on_grant(target_perm):
                target_perm.grant_keyword('_is_cannot_attack')

            game.effect_select_opponent_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=False
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Inherited effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-023 Opponent's 1 Digimon can't attack")
        effect2.set_effect_description("[When Attacking][Once Per Turn] Until the end of your opponent's turn, 1 of your opponent's Digimon with as many or fewer digivolution cards as this Digimon can't attack.")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("CantAtatck_BT14_023_inherited")
        effect2.is_on_attack = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            my_evo_count = len(getattr(perm, 'digivolution_cards', []) or [])

            def target_filter(p):
                return p.is_digimon and len(getattr(p, 'digivolution_cards', []) or []) <= my_evo_count

            def on_grant(target_perm):
                target_perm.grant_keyword('_is_cannot_attack')

            game.effect_select_opponent_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=False
            )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
