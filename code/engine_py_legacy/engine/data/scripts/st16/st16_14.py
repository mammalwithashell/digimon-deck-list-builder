from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST16_14(CardScript):
    """ST16-14 Matt Ishida | Tamer | Purple | Cost 4

    [Start of Your Turn] If your memory is at 2 or less, it becomes 3.
    [All Turns] When one of your effects trashes a card in your hand,
        by suspending this Tamer, gain 1 memory.
    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Start of Your Turn] Memory to 3 ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartTurn)
        effect0.set_effect_name("ST16-14 Set memory to 3")
        effect0.set_effect_description(
            "[Start of Your Turn] If your memory is at 2 or less, it becomes 3."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and game.memory <= 2:
                game.memory = 3
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [All Turns] When your effect trashes a card in hand,
        #     suspend this Tamer to gain 1 memory ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDiscardHand)
        effect1.set_effect_name("ST16-14 Suspend for memory on hand trash")
        effect1.set_effect_description(
            "[All Turns] When one of your effects trashes a card in "
            "your hand, by suspending this Tamer, gain 1 memory."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if perm is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            if perm not in owner.battle_area:
                return False
            # Tamer must not already be suspended
            if perm.is_suspended:
                return False
            # Must be triggered by one of our effects trashing our hand card
            event_player = context.get('event_player')
            if event_player is not None and event_player is not owner:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return
            # Cost: suspend this Tamer
            perm.suspend()
            # Gain 1 memory
            player.add_memory(1)
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Play this card without paying the cost ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("ST16-14 Security: Play free")
        effect2.set_effect_description(
            "[Security] Play this card without paying the cost."
        )
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player and card:
                player.play_card_from_source(card, pay_cost=False)
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
