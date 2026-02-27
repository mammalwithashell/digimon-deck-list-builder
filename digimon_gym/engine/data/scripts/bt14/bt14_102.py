from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_102(CardScript):
    """BT14-102 Angemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [When Attacking] By deleting this Digimon, choose 1:
        # - Place 1 opponent Digimon with [Virus] trait at bottom of security.
        # - 1 opponent Digimon gets -5000 DP for the turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-102 Delete this Digimon to choose 1 effect")
        effect1.set_effect_description("[When Attacking] By deleting this Digimon, activate 1 of the effects below: - Place 1 of your opponent's Digimon with the [Virus] trait at the bottom of their security stack. - 1 of your opponent's Digimon gets -5000 DP for the turn.")
        effect1.is_optional = True
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            return card is not None and card.permanent_of_this_card() is not None

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            # Cost: delete this Digimon.
            perm.delete()

            enemy = player.enemy
            if not enemy:
                return

            # Deterministic mode selection: prefer Virus-bottom-security if valid target exists;
            # otherwise apply -5000 DP if possible.
            virus_targets = [
                p for p in (enemy.battle_area or [])
                if p.is_digimon and hasattr(p, 'card') and p.card is not None
                and ('Virus' in (getattr(p.card, 'attribute_eng', []) or []))
            ]
            if virus_targets:
                target = virus_targets[0]
                player.put_permanent_to_security(target)
                return

            dp_targets = [p for p in (enemy.battle_area or []) if p.is_digimon and p.dp is not None]
            if dp_targets:
                target = min(dp_targets, key=lambda p: p.dp)
                target.change_dp(-5000)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [On Deletion] Place this card at the bottom of your security stack.
        # Then, if you have a Tamer, you may hatch in your breeding area.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-102 Place this card at bottom of security and hatch")
        effect2.set_effect_description("[On Deletion] Place this card at the bottom of your security stack. Then, if you have a Tamer, you may hatch in your breeding area.")
        effect2.is_on_deletion = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if not player:
                return

            if perm is not None:
                player.put_permanent_to_security(perm)

            has_tamer = bool(getattr(player, 'tamer_area', None))
            if has_tamer and hasattr(player, 'hatch'):
                player.hatch()

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Inherited [On Deletion] Place 1 yellow Digimon with [Vaccine] trait from hand
        # at the bottom of your security stack.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT14-102 Inherited place yellow Vaccine Digimon from hand to security")
        effect3.set_effect_description("[On Deletion] Place 1 yellow card with the [Vaccine] trait from your hand at the bottom of your security stack.")
        effect3.is_inherited_effect = True
        effect3.is_optional = True
        effect3.is_on_deletion = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not player:
                return

            hand = getattr(player, 'hand', None) or []
            valid = [
                c for c in hand
                if 2 in (getattr(c, 'card_colors', []) or [])
                and 'Vaccine' in (getattr(c, 'attribute_eng', []) or [])
                and getattr(c, 'card_kind', None) == 0
            ]
            if not valid:
                return

            chosen = valid[0]
            if hasattr(player, 'put_hand_card_to_security'):
                player.put_hand_card_to_security(chosen)
            elif hasattr(player, 'move_card_from_hand_to_security'):
                player.move_card_from_hand_to_security(chosen)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
