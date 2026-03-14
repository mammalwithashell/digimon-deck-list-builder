from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT5_107(CardScript):
    """BT5-107 Revive From the Darkness! | Option Purple Cost 4
    [Main] Delete 1 of your purple Digimon. Then, you may play 1 level 5 or
    lower purple Digimon card from your trash without paying its memory cost.
    Any [On Play] effects on Digimon played with this effect don't activate.
    [Security] Add this card to the hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Main] Delete own purple Digimon, play Lv5 or lower purple from trash
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT5-107 Delete own purple, play from trash")
        effect0.set_effect_description(
            "[Main] Delete 1 of your purple Digimon. Then, you may play 1 "
            "level 5 or lower purple Digimon card from your trash without "
            "paying its memory cost."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            from ....data.enums import CardColor

            def own_purple_filter(p):
                if not p.is_digimon:
                    return False
                colors = getattr(p.top_card, 'card_colors', []) or [] if p.top_card else []
                return CardColor.Purple in colors

            def on_delete(target_perm):
                player.delete_permanent(target_perm)
                # Then play Lv5 or lower purple Digimon from trash
                def trash_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    level = getattr(c, 'level', None)
                    if level is None or level > 5:
                        return False
                    colors = getattr(c, 'card_colors', []) or []
                    return CardColor.Purple in colors

                game.effect_play_from_zone(
                    player, 'trash', trash_filter, free=True, is_optional=True,
                    prompt="Select 1 level 5 or lower purple Digimon from trash to play.")

            game.effect_select_own_permanent(
                player, on_delete, filter_fn=own_purple_filter, is_optional=False,
                prompt="Delete 1 of your purple Digimon.")
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [Security] Add this card to the hand.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("BT5-107 Security: Add to hand")
        effect1.set_effect_description("[Security] Add this card to the hand.")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player and card:
                if card in player.trash_cards:
                    player.trash_cards.remove(card)
                    player.hand_cards.append(card)
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
