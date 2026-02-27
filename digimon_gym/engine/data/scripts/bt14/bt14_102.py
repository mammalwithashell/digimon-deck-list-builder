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

        # [When Attacking] By deleting this Digimon, activate 1 of the effects below:
        # - Place 1 of your opponent's Digimon with the [Virus] trait at the bottom of their security stack.
        # - 1 of your opponent's Digimon gets -5000 DP for the turn.
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
            game = ctx.get('game')
            perm = ctx.get('permanent')
            if not (player and game and perm):
                return

            # Cost: delete this Digimon
            if hasattr(perm, 'delete'):
                perm.delete()
            elif hasattr(player, 'delete_permanent'):
                player.delete_permanent(perm)

            enemy = player.enemy
            if not enemy:
                return

            def virus_filter(p):
                attrs = getattr(p, 'attribute_eng', None) or getattr(p, 'attributes', None) or []
                return bool(getattr(p, 'is_digimon', False)) and ('Virus' in attrs)

            def any_enemy_digimon_filter(p):
                return bool(getattr(p, 'is_digimon', False))

            def do_bottom_security(target_perm):
                enemy.put_permanent_to_security(target_perm)

            def do_minus_5000(target_perm):
                target_perm.change_dp(-5000)

            # Choose one mode deterministically by availability preference:
            # Prefer Virus bottom-security if a valid target exists; otherwise DP reduction.
            virus_targets = [p for p in getattr(enemy, 'battle_area', []) if virus_filter(p)]
            if virus_targets:
                game.effect_select_enemy_permanent(player, do_bottom_security, filter_fn=virus_filter, is_optional=False)
                return

            dp_targets = [p for p in getattr(enemy, 'battle_area', []) if any_enemy_digimon_filter(p)]
            if dp_targets:
                game.effect_select_enemy_permanent(player, do_minus_5000, filter_fn=any_enemy_digimon_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [On Deletion] Place this card at the bottom of your security stack.
        # Then, if you have a Tamer, you may hatch in your breeding area.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-102 Place this card at bottom security, then optional hatch")
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

            tamers = [p for p in getattr(player, 'battle_area', []) if getattr(p, 'is_tamer', False)]
            if tamers and hasattr(player, 'hatch'):
                player.hatch(is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Inherited [On Deletion] Place 1 yellow Digimon with the [Vaccine] trait from your hand
        # at the bottom of your security stack.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT14-102 Inherited place yellow Vaccine Digimon from hand to security")
        effect3.set_effect_description("[On Deletion] Place 1 yellow Digimon with the [Vaccine] trait from your hand at the bottom of your security stack.")
        effect3.is_inherited_effect = True
        effect3.is_optional = True
        effect3.is_on_deletion = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def hand_filter(c):
                if getattr(c, 'card_kind', None) != 0:
                    return False
                colors = getattr(c, 'card_colors', []) or []
                attrs = getattr(c, 'attribute_eng', []) or []
                return (2 in colors) and ('Vaccine' in attrs)

            def on_select(card_source):
                player.put_card_from_hand_to_security(card_source)

            game.effect_select_own_hand_card(player, on_select, filter_fn=hand_filter, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
