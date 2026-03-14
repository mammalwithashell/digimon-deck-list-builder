from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT6_107(CardScript):
    """BT6-107 Glaive Memory Boost! | Option Purple Cost 3
    [Main] Return 1 purple Digimon card from your trash to your hand.
    Then, place this card in your battle area.
    [Main] <Delay> Gain 2 memory.
    [Security] Place this card in the battle area.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Main] Return purple Digimon from trash, place this in battle area
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT6-107 Return purple Digimon from trash")
        effect0.set_effect_description(
            "[Main] Return 1 purple Digimon card from your trash to your hand. "
            "Then, place this card in your battle area."
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

            qualifying = [c for c in player.trash_cards
                         if getattr(c, 'is_digimon', False) and
                         CardColor.Purple in (getattr(c, 'card_colors', []) or [])]
            if qualifying:
                chosen = qualifying[0]
                player.trash_cards.remove(chosen)
                player.hand_cards.append(chosen)

            # Place this card in the battle area
            if card and player:
                player.play_card_from_source(card, pay_cost=False)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Delay marker
        effect1 = ICardEffect()
        effect1.set_effect_name("BT6-107 Delay")
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
        effect2.set_effect_name("BT6-107 Delay: Gain 2 memory")
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

        # [Security] Place this card in the battle area
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("BT6-107 Security: Place in battle area")
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
