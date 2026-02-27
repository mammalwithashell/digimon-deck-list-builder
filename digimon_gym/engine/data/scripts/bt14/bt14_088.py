from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_088(CardScript):
    """BT14-088 Gennai"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may reveal the top 5 cards of your deck. Add 1 level 3 Digimon card and 1 non-white Tamer card among them to the hand. Return the rest to the bottom of the deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-088 Reveal the top 5 cards of deck")
        effect0.set_effect_description("[On Play] You may reveal the top 5 cards of your deck. Add 1 level 3 Digimon card and 1 non-white Tamer card among them to the hand. Return the rest to the bottom of the deck.")
        effect0.is_optional = True
        effect0.is_on_play = True

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

            def reveal_filter_0(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                return getattr(c, 'level', None) == 3

            def reveal_filter_1(c):
                if not getattr(c, 'is_tamer', False):
                    return False
                return not ('White' in [col.name for col in getattr(c, 'card_colors', [])])

            game.effect_reveal_and_select_multi(
                player,
                5,
                [(reveal_filter_0, 'hand'), (reveal_filter_1, 'hand')],
                remaining_placement='deck_bottom',
                is_optional=True,
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [Opponent's Turn] When an opponent's level 5 or higher Digimon attacks, by suspending this Tamer, move 1 of your Digimon from the breeding area to the battle area.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-088 Move your Digimon")
        effect1.set_effect_description("[Opponent's Turn] When an opponent's level 5 or higher Digimon attacks, by suspending this Tamer, move 1 of your Digimon from the breeding area to the battle area.")
        effect1.is_optional = True
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            attacker = context.get('attacker')
            if not attacker:
                return False
            if getattr(attacker, 'level', None) is None or getattr(attacker, 'level', 0) < 5:
                return False
            owner = context.get('player')
            if owner is not None and hasattr(attacker, 'owner') and attacker.owner == owner:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return

            perm.suspend()

            def can_move(p):
                return True

            game.effect_select_own_breeding_permanent(
                player,
                lambda target_perm: game.move_permanent_to_battle_area(target_perm),
                filter_fn=can_move,
                is_optional=False,
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-088 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
