from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, CardColor

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT5_106(CardScript):
    """BT5-106 Demonic Disaster | Option Purple"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Main] Delete 1 own Digimon to unsuspend 1 purple Digimon ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT5-106 Delete own Digimon, unsuspend purple Digimon")
        effect0.set_effect_description(
            "[Main] You may delete 1 of your Digimon to unsuspend 1 of "
            "your purple Digimon."
        )
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Delete 1 own Digimon, unsuspend 1 purple Digimon"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Delete 1 of your Digimon
            def own_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                player.delete_permanent(target_perm)
                # Then unsuspend 1 purple Digimon
                def purple_filter(p):
                    if not p.is_digimon or not p.is_suspended:
                        return False
                    if p.top_card:
                        colors = [col.name for col in (p.top_card.card_colors or [])]
                        return 'Purple' in colors
                    return False
                def on_unsuspend(unsuspend_perm):
                    unsuspend_perm.unsuspend()
                game.effect_select_own_permanent(
                    player, on_unsuspend, filter_fn=purple_filter, is_optional=True)
            game.effect_select_own_permanent(
                player, on_delete, filter_fn=own_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Security] Play 1 level 3 purple Digimon from trash
        #     without paying cost (On Play effects don't activate) ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("BT5-106 Security: Play Lv3 purple from trash")
        effect1.set_effect_description(
            "[Security] You may play 1 level 3 purple Digimon card from "
            "your trash without paying its memory cost. Any [On Play] effects "
            "on Digimon played with this effect don't activate."
        )
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Lv3 purple Digimon from trash free (no On Play)"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            def lv3_purple_filter(c):
                if not c.is_digimon:
                    return False
                if c.level != 3:
                    return False
                colors = [col.name for col in (c.card_colors or [])]
                return 'Purple' in colors
            game.effect_play_from_zone(
                player, 'trash', lv3_purple_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
