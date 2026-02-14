from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_093(CardScript):
    """BT23-093"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-093 Ignore color requirements")
        effect0.set_effect_description("Effect")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # [Main] <Draw 1> (Draw 1 card from your deck.) Then, place this card in the battle area.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-093 Draw 1")
        effect1.set_effect_description("[Main] <Draw 1> (Draw 1 card from your deck.) Then, place this card in the battle area.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns] When any of your [Appmon] trait Digimon suspend <Delay>, You may link 1 [Appmon] trait Digimon card from your hand to 1 of those Digimon without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-093 Link 1 [Appmon] trait digimon from hand")
        effect2.set_effect_description("[All Turns] When any of your [Appmon] trait Digimon suspend <Delay>, You may link 1 [Appmon] trait Digimon card from your hand to 1 of those Digimon without paying the cost.")
        effect2.is_optional = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
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
                player, hand_filter, on_trashed, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.SecuritySkill
        # [Security] place this card in the battle area.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-093 place in battle area")
        effect3.set_effect_description("[Security] place this card in the battle area.")
        effect3.is_security_effect = True
        effect3.is_security_effect = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
