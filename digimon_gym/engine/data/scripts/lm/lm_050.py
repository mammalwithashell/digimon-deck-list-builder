from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class LM_050(CardScript):
    """LM-050 Magenta Memory Boost! | Option Purple Cost 3
    Red also meets this card's color requirements.
    [Main] Reveal top 3 cards. Add 1 purple or red Digimon card to hand.
    Return rest to bottom. Then, place this card in battle area.
    [Main] <Delay> Gain 2 memory.
    [Security] Place this card in the battle area.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        # Red also meets color requirements
        card._match_color_requirement = False
        effects = []

        # [Main] Reveal top 3, add purple/red Digimon, place in battle area
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("LM-050 Reveal top 3, add purple/red Digimon")
        effect0.set_effect_description(
            "[Main] Reveal top 3. Add 1 purple or red Digimon to hand. "
            "Return rest to bottom. Place this in battle area."
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

            def purple_or_red_digimon(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = getattr(c, 'card_colors', []) or []
                return CardColor.Purple in colors or CardColor.Red in colors

            def on_selected(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)

            game.effect_reveal_and_select(
                player, 3,
                filter_fn=purple_or_red_digimon,
                on_selected=on_selected,
                is_optional=False,
                prompt="Select 1 purple or red Digimon card to add to hand.")

            # Place this card in battle area
            if card and player:
                player.play_card_from_source(card, pay_cost=False)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Delay marker
        effect1 = ICardEffect()
        effect1.set_effect_name("LM-050 Delay")
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

        # [Main] <Delay> Gain 2 memory
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnStartMainPhase)
        effect2.set_effect_name("LM-050 Delay: Gain 2 memory")
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
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                perm = card.permanent_of_this_card() if card else None
                if perm:
                    player.delete_permanent(perm)
                player.add_memory(2)
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # [Security] Place this card in battle area
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("LM-050 Security: Place in battle area")
        effect3.set_effect_description("[Security] Place this card in the battle area.")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player and card:
                player.play_card_from_source(card, pay_cost=False)
        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
