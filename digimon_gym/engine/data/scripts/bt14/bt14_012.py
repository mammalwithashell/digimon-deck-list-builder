from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_012(CardScript):
    """BT14-012 Greymon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] This Digimon gets +2000 DP for the turn. Then, if you have a Tamer with [Tai Kamiya] in its name, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-012 DP +2000 and conditional Memory +1")
        effect1.set_effect_description("[When Attacking] This Digimon gets +2000 DP for the turn. Then, if you have a Tamer with [Tai Kamiya] in its name, gain 1 memory.")
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if perm:
                perm.change_dp(2000)

            if not player:
                return

            has_tai_kamiya = False
            tamers = getattr(player, 'tamers', None)
            if tamers:
                for tamer in tamers:
                    name = getattr(tamer, 'name', None)
                    if name is None:
                        source = getattr(tamer, 'card_source', None)
                        name = getattr(source, 'card_name_eng', '') if source else ''
                    if name and 'tai kamiya' in str(name).lower():
                        has_tai_kamiya = True
                        break

            if has_tai_kamiya:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Inherited: [Your Turn] While this Digimon has [Greymon] or [Omnimon] in its name, it gets +2000 DP.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-012 Inherited DP +2000")
        effect2.set_effect_description("[Your Turn] While this Digimon has [Greymon] or [Omnimon] in its name, it gets +2000 DP.")
        effect2.is_inherited_effect = True
        effect2.dp_modifier = 2000

        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False

            host = context.get('permanent')
            if host is None:
                host = card.permanent_of_this_card() if card else None
            if host is None:
                return False

            name = getattr(host, 'name', None)
            if name is None:
                source = getattr(host, 'card_source', None)
                name = getattr(source, 'card_name_eng', '') if source else ''

            lowered = str(name).lower() if name else ''
            if not (('greymon' in lowered) or ('omnimon' in lowered)):
                return False

            excluded_names = ('dorugreymon', 'burninggreymon', 'dexdorugreymon')
            return not any(excluded in lowered for excluded in excluded_names)

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
