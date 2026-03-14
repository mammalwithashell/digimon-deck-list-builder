from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT7_105(CardScript):
    """BT7-105 Pride Memory Boost! | Option (Black, Cost 4)

    [Main] Reveal the top 3 cards of your deck. You may play 1 black
        Digimon card with a play cost of 4 or less among them without
        paying its memory cost. Trash the remaining cards. Then, place
        this card in your battle area.
    [Main] <Delay> (By trashing this card after the placing turn,
        activate the effect below.)
        - Gain 2 memory.
    [Security] Place this card in the battle area.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Main] Reveal top 3, play 1 black Digimon cost<=4, trash rest ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT7-105 Reveal top 3 play black Digimon cost<=4")
        effect0.set_effect_description(
            "[Main] Reveal the top 3 cards of your deck. You may play 1 black "
            "Digimon card with a play cost of 4 or less among them without "
            "paying its memory cost. Trash the remaining cards. Then, place "
            "this card in your battle area."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Reveal top 3, play 1 qualifying card free, trash remaining."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def reveal_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = getattr(c, 'card_colors', []) or []
                if not any(col.name == 'Black' for col in colors):
                    return False
                cost = getattr(c, 'get_cost_itself', 99)
                return cost <= 4

            def on_revealed(selected, remaining):
                # Play the selected card for free
                player.play_card_from_source(selected, pay_cost=False)
                # Trash remaining cards
                for c in remaining:
                    player.trash_cards.append(c)

            game.effect_reveal_and_select(
                player, 3, reveal_filter, on_revealed, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Delay marker ---
        effect1 = ICardEffect()
        effect1.set_effect_name("BT7-105 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: Delay effect — Gain 2 memory ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnStartMainPhase)
        effect2.set_effect_name("BT7-105 Delay: Gain 2 memory")
        effect2.set_effect_description("<Delay> Gain 2 memory.")
        effect2._is_delay_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Gain 2 memory."""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                player.add_memory(2)
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: [Security] Place this card in the battle area ---
        effect3 = ICardEffect()
        effect3.set_effect_name("BT7-105 Security: Place in battle area")
        effect3.set_effect_description("[Security] Place this card in the battle area.")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Place this card in the battle area from security."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game and card):
                return
            player.play_card_from_source(card, pay_cost=False)
        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
