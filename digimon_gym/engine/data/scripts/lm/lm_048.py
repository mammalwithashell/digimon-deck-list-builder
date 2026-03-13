from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class LM_048(CardScript):
    """LM-048 Chrome Memory Boost! | Option (Green/Black, Cost 3)

    [Main] Reveal the top 3 cards of your deck. Add 1 green or black
        Digimon card among them to the hand. Return the rest to the bottom
        of deck. Then, place this card in the battle area.
    [Main] <Delay> (By trashing this card after the placing turn, activate
        the effect below.)
        - Gain 2 memory.
    [Security] Place this card in the battle area.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Main] Reveal top 3, add 1 green/black Digimon ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("LM-048 Reveal top 3 add green/black Digimon")
        effect0.set_effect_description(
            "[Main] Reveal the top 3 cards of your deck. Add 1 green or "
            "black Digimon card among them to the hand. Return the rest "
            "to the bottom of deck. Then, place this card in the battle area."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Reveal top 3, add 1 green or black Digimon to hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            revealed = []
            for _ in range(min(3, len(player.library_cards))):
                revealed.append(player.library_cards.pop(0))

            added = False
            to_bottom = []
            for c in revealed:
                colors = getattr(c, 'card_colors', []) or []
                color_names = [col.name for col in colors]
                is_digimon = getattr(c, 'is_digimon', False)
                if (is_digimon and not added
                        and ('Green' in color_names or 'Black' in color_names)):
                    player.hand_cards.append(c)
                    added = True
                else:
                    to_bottom.append(c)

            for c in to_bottom:
                player.library_cards.append(c)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Delay marker ---
        effect1 = ICardEffect()
        effect1.set_effect_name("LM-048 Delay")
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
        effect2.set_effect_name("LM-048 Delay: Gain 2 memory")
        effect2.set_effect_description(
            "<Delay> Gain 2 memory."
        )
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
        effect3.set_effect_name("LM-048 Security: Place in battle area")
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
