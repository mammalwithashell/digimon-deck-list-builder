from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_048(CardScript):
    """BT14-048 Leomon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] If attacking a Digimon with DP higher than this Digimon,
        # this Digimon may digivolve into 1 level 6 Digimon card with [Leomon] in its name
        # in your hand for a cost of 6, ignoring its digivolution requirements.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-048 Digivolve this Digimon")
        effect0.set_effect_description("[When Attacking] If attacking a Digimon with DP higher than this Digimon, this Digimon may digivolve into 1 level 6 Digimon card with [Leomon] in its name in your hand for a cost of 6, ignoring its digivolution requirements.")
        effect0.is_optional = True
        effect0.is_on_attack = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False

            attacker = context.get('permanent')
            target = context.get('attack_target') or context.get('target')
            if attacker is None or target is None:
                return False

            # Must be attacking an opponent Digimon with higher DP.
            if not getattr(target, 'is_digimon', False):
                return False

            attacker_dp = getattr(attacker, 'dp', None)
            target_dp = getattr(target, 'dp', None)
            if attacker_dp is None or target_dp is None:
                return False

            return target_dp > attacker_dp

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) != 6:
                    return False
                return any('Leomon' in n for n in getattr(c, 'card_names', []))

            game.effect_digivolve_from_hand(
                player,
                perm,
                digi_filter,
                digivolve_cost=6,
                ignore_requirements=True,
                is_optional=True,
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Inherited: [Your Turn] While this Digimon has [Leomon] in its name, it gets +2000 DP.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-048 DP modifier")
        effect1.set_effect_description("[Your Turn] While this Digimon has [Leomon] in its name, it gets +2000 DP.")
        effect1.is_inherited_effect = True
        effect1.dp_modifier = 2000

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False

            perm = context.get('permanent')
            if perm is None:
                perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False

            names = []
            for attr in ('card_names', 'name', 'card_name'):
                v = getattr(perm, attr, None)
                if isinstance(v, list):
                    names.extend(v)
                elif isinstance(v, str):
                    names.append(v)

            return any('Leomon' in n for n in names)

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
