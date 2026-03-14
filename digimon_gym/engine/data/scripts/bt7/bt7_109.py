from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT7_109(CardScript):
    """BT7-109 Dead or Alive | Option

    [Main] Play 1 purple level 5 Digimon card from your trash without paying
        its memory cost. If there are 10 or more cards in your trash, you may
        play 1 Digimon card with [Lucemon] in its name without paying its
        memory cost instead.
    [Security] Activate this card's [Main] effects.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _do_main_effect(ctx):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            has_10_trash = len(player.trash_cards) >= 10

            if has_10_trash:
                # Choose: play Lucemon from trash OR play purple Lv5 from trash
                def on_branch(choice):
                    if choice == 0:
                        # Play Digimon with [Lucemon] in name from trash
                        def lucemon_filter(c):
                            if not getattr(c, 'is_digimon', False):
                                return False
                            names = getattr(c, 'card_names', []) or []
                            return any('Lucemon' in n for n in names)

                        game.effect_play_from_zone(
                            player, 'trash', lucemon_filter, free=True, is_optional=True)
                    else:
                        # Play purple Lv5 Digimon from trash
                        def purple_lv5_filter(c):
                            if not getattr(c, 'is_digimon', False):
                                return False
                            if getattr(c, 'level', None) != 5:
                                return False
                            colors = [col.name for col in (getattr(c, 'card_colors', []) or [])]
                            return 'Purple' in colors

                        game.effect_play_from_zone(
                            player, 'trash', purple_lv5_filter, free=True, is_optional=True)

                game.effect_choose_branch(
                    player, 2, on_branch,
                    branch_labels=[
                        "Play a Digimon with [Lucemon] in its name",
                        "Play a purple level 5 Digimon"
                    ])
            else:
                # Only purple Lv5
                def purple_lv5_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    if getattr(c, 'level', None) != 5:
                        return False
                    colors = [col.name for col in (getattr(c, 'card_colors', []) or [])]
                    return 'Purple' in colors

                game.effect_play_from_zone(
                    player, 'trash', purple_lv5_filter, free=True, is_optional=True)

        # [Main]
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT7-109 Main: play from trash")
        effect0.set_effect_description(
            "[Main] Play 1 purple level 5 Digimon card from your trash without paying "
            "its memory cost. If there are 10 or more cards in your trash, you may play "
            "1 Digimon card with [Lucemon] in its name without paying its memory cost instead."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effect0.set_on_process_callback(_do_main_effect)
        effects.append(effect0)

        # [Security] Activate this card's [Main] effects
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("BT7-109 Security: activate Main effects")
        effect1.set_effect_description("[Security] Activate this card's [Main] effects.")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_do_main_effect)
        effects.append(effect1)

        return effects
