from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_094(CardScript):
    """BT16-094 Dragon's Breath"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Reveal the top 4 cards of your deck. Add 1 yellow card or 1 card with the [Four Great Dragons] trait among them to your hand. Place the remaining cards at the bottom of your deck in any order. Then, place this card in your battle area.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT16-094 Add To Hand")
        effect0.set_effect_description("[Main] Reveal the top 4 cards of your deck. Add 1 yellow card or 1 card with the [Four Great Dragons] trait among them to your hand. Place the remaining cards at the bottom of your deck in any order. Then, place this card in your battle area.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Add To Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: delay
        # Delay
        effect1 = ICardEffect()
        effect1.set_effect_name("BT16-094 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDeclaration
        # [Main] <Delay>\r\n� Place 1 [Trial of the Four Great Dragons] from your hand in the battle area, or you may trash 1 card with the [Four Great Dragons] trait in your hand. If you did either, 1 of your opponent's Digimon gets -7000 DP for the turn.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDeclaration)
        effect2.set_effect_name("BT16-094 Play 1 [Trial of the Four Great Dragons] or trash 1 card and apply effects.")
        effect2.set_effect_description("[Main] <Delay>\r\n� Place 1 [Trial of the Four Great Dragons] from your hand in the battle area, or you may trash 1 card with the [Four Great Dragons] trait in your hand. If you did either, 1 of your opponent's Digimon gets -7000 DP for the turn.")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def hand_filter(c):
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.SecuritySkill
        # [Security] 1 of your opponent's Digimon gets -7000 DP for the turn. Then, place this card in the battle area.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("BT16-094 Effect")
        effect3.set_effect_description("[Security] 1 of your opponent's Digimon gets -7000 DP for the turn. Then, place this card in the battle area.")
        effect3.is_security_effect = True
        effect3.is_security_effect = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
