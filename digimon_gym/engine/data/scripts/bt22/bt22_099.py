from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_099(CardScript):
    """BT22-099 Kuremi Detective Agency"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Ignore Color Req — while you have a [CS] trait Digimon or Tamer
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-099 Ignore color requirements")
        effect0.set_effect_description("While you have a [CS] trait Digimon or Tamer on the field, you can ignore this card's color requirements.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Ignore Color Req"""
            # Ignores color requirement for playing Options — not modeled in engine
            pass  # descriptive-tagged

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # [Main] Reveal the top 3 cards of your deck. Add 1 [CS] trait card
        # among them to the hand. Return the rest to the bottom of the deck.
        # Then, place this card in the battle area.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("BT22-099 Reveal top 3, add 1 [CS] card to hand, bottom deck the rest")
        effect1.set_effect_description("[Main] Reveal the top 3 cards of your deck. Add 1 [CS] trait card among them to the hand. Return the rest to the bottom of the deck. Then, place this card in the battle area.")

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Reveal top 3, add 1 CS card to hand"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def reveal_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                return any('CS' in t for t in traits)

            game.effect_reveal_and_select_multi(
                player, 3,
                passes=[
                    (reveal_filter, 'hand'),
                ],
                remaining_placement='deck_bottom',
                is_optional=True
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: delay
        # Delay
        effect2 = ICardEffect()
        effect2.set_effect_name("BT22-099 Delay")
        effect2.set_effect_description("Delay")
        effect2._is_delay = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnDeclaration
        # [Main] <Delay> Gain 2 memory.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDeclaration)
        effect3._is_field_main = True
        effect3.set_effect_name("BT22-099 Delay: Gain 2 memory")
        effect3.set_effect_description("[Main] <Delay> Gain 2 memory.")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Gain 2 memory"""
            player = ctx.get('player')
            if player:
                player.add_memory(2)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Security Effect: Place this card in the battle area
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.SecuritySkill)
        effect4.set_effect_name("BT22-099 Security: Place in battle area")
        effect4.set_effect_description("[Security] Place this card in the battle area.")
        effect4.is_security_effect = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
