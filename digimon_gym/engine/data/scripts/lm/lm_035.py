from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class LM_035(CardScript):
    """LM-035 Amber Memory Boost! | Option (Yellow, Cost 3)

    Purple also meets this card's color requirements.
    [Main] Reveal the top 3 cards of your deck. Add 1 yellow or purple
        Digimon card among them to the hand. Return the rest to the bottom
        of deck. Then, place this card in the battle area.
    [Main] <Delay> (By trashing this card after the placing turn, activate
        the effect below.)
        - Gain 2 memory.
    [Security] Place this card in the battle area.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Ignore color requirements (Purple also meets) ---
        effect0 = ICardEffect()
        effect0.set_effect_name("LM-035 Ignore color requirements")
        effect0.set_effect_description(
            "Purple also meets this card's color requirements."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [Main] Reveal top 3, add 1 yellow/purple Digimon, place in battle area ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("LM-035 Reveal top 3, add yellow/purple Digimon")
        effect1.set_effect_description(
            "[Main] Reveal the top 3 cards of your deck. Add 1 yellow or "
            "purple Digimon card among them to the hand. Return the rest "
            "to the bottom of deck. Then, place this card in the battle area."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def yellow_or_purple_digimon_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = getattr(c, 'card_colors', []) or []
                color_names = [col.name for col in colors]
                return 'Yellow' in color_names or 'Purple' in color_names

            def on_selected(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)

            game.effect_reveal_and_select(
                player, 3,
                filter_fn=yellow_or_purple_digimon_filter,
                on_selected=on_selected,
                is_optional=False,
                prompt="Select 1 yellow or purple Digimon card to add to hand."
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: Delay marker ---
        effect2 = ICardEffect()
        effect2.set_effect_name("LM-035 Delay")
        effect2.set_effect_description("Delay")
        effect2._is_delay = True

        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # --- Effect 3: [Main] <Delay> Gain 2 memory ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnStartMainPhase)
        effect3.set_effect_name("LM-035 Delay: Gain 2 memory")
        effect3.set_effect_description("<Delay> Gain 2 memory.")
        effect3._is_delay_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                # Trash this card from battle area (Delay consumption)
                perm = card.permanent_of_this_card() if card else None
                if perm:
                    player.delete_permanent(perm)
                player.add_memory(2)
        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # --- Effect 4: [Security] Place this card in the battle area ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.SecuritySkill)
        effect4.set_effect_name("LM-035 Security: Place in battle area")
        effect4.set_effect_description(
            "[Security] Place this card in the battle area."
        )
        effect4.is_security_effect = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game and card):
                return
            player.play_card_from_source(card, pay_cost=False)
        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
