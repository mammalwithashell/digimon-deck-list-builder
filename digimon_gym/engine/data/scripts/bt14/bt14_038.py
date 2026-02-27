from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_038(CardScript):
    """BT14-038 Etemon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-038 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 4
        effect0._alt_digi_cost = 4

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.SecuritySkill
        # [Security] If you have 3 or more cards with [Sukamon] in their names in your trash,
        # you may play 1 level 6 Digimon card with [Etemon] in its name from your hand without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-038 Play 1 Digimon from hand")
        effect1.set_effect_description("[Security] If you have 3 or more cards with [Sukamon] in their names in your trash, you may play 1 level 6 Digimon card with [Etemon] in its name from your hand without paying the cost.")
        effect1.is_optional = True
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            player = context.get('player')
            if not player:
                return False
            trash = getattr(player, 'trash', [])
            count = 0
            for c in trash:
                for n in getattr(c, 'card_names', []):
                    if 'Sukamon' in n:
                        count += 1
                        break
            return count >= 3

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) != 6:
                    return False
                return any('Etemon' in _n for _n in getattr(c, 'card_names', []))

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Place this card at the bottom of your security stack.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-038 Place this card at the bottom of security")
        effect2.set_effect_description("[On Deletion] Place this card at the bottom of your security stack.")
        effect2.is_on_deletion = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Move this card to bottom of security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            game.effect_add_card_to_security(player, perm.card, to_bottom=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Place 1 [Etemon] from your trash at the bottom of your security stack.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT14-038 Place 1 [Etemon] from trash at the bottom of security")
        effect3.set_effect_description("[On Deletion] Place 1 [Etemon] from your trash at the bottom of your security stack.")
        effect3.is_inherited_effect = True
        effect3.is_on_deletion = True

        def condition3(context: Dict[str, Any]) -> bool:
            player = context.get('player')
            if not player:
                return False
            for c in getattr(player, 'trash', []):
                if any('Etemon' in n for n in getattr(c, 'card_names', [])):
                    return True
            return False

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Add To Security"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def pick_filter(c):
                return any('Etemon' in n for n in getattr(c, 'card_names', []))

            game.effect_move_card_from_zone_to_security(
                player, 'trash', pick_filter, to_bottom=True, is_optional=False, count=1)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
