from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class LM_047(CardScript):
    """LM-047 Chartreuse Memory Boost! | Option (Yellow, Cost 3)

    Green also meets this card's color requirements.
    [Main] Reveal the top 3 cards of your deck. Add 1 yellow or green
    Digimon card among them to the hand. Return the rest to the bottom
    of deck. Then, place this card in the battle area.
    [Main] Delay: Gain 2 memory.
    [Security] Place this card in the battle area.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Ignore color requirements (green also meets)
        effect0 = ICardEffect()
        effect0.set_effect_name("LM-047 Green meets color requirements")
        effect0.set_effect_description("Green also meets this card's color requirements.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [Main] Reveal top 3, add 1 yellow/green Digimon, rest to deck bottom.
        # Then place this card in battle area.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("LM-047 Reveal 3, add 1 yellow/green Digimon")
        effect1.set_effect_description(
            "[Main] Reveal the top 3 cards of your deck. Add 1 yellow or "
            "green Digimon card among them to the hand. Return the rest to "
            "the bottom of deck. Then, place this card in the battle area."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            from ....data.enums import CardColor

            def reveal_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = getattr(c, 'card_colors', []) or []
                return any(
                    clr in (CardColor.Yellow, CardColor.Green)
                    for clr in colors
                )

            def on_selected(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)

            game.effect_reveal_and_select(
                player, 3, reveal_filter, on_selected,
                is_optional=True,
                prompt="Add 1 yellow/green Digimon to hand.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Delay: Gain 2 memory
        effect2 = ICardEffect()
        effect2.set_effect_name("LM-047 Delay")
        effect2.set_effect_description("Delay: Gain 2 memory")
        effect2._is_delay = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            return owner is not None and owner.is_my_turn
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnStartMainPhase)
        effect3.set_effect_name("LM-047 Delay: Gain 2 memory")
        effect3.set_effect_description("[Main] Delay: Gain 2 memory.")
        effect3.is_optional = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            return owner is not None and owner.is_my_turn
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.add_memory(2)
        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # [Security] Place this card in the battle area.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.SecuritySkill)
        effect4.set_effect_name("LM-047 Security: Place in battle area")
        effect4.set_effect_description("[Security] Place this card in the battle area.")
        effect4.is_security_effect = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
