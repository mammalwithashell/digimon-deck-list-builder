from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class BT20_086(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Effect 1: Memory Setter
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-086 Start of Turn")
        effect1.set_effect_description("[Start of Your Turn] If you have 2 or less memory, set it to 3.")

        def condition1(context: Dict[str, Any]) -> bool:
            player = context.get("player")
            game = context.get("game")

            # Hack: Store game in effect to access in process
            effect1.game_context = game

            if not player or not player.is_my_turn:
                return False
            if not game:
                return False
            return game.memory <= 2

        def process1():
            game = getattr(effect1, 'game_context', None)
            if game:
                game.memory = 3
                print("BT20-086: Memory set to 3.")

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Effect 2: Start of Main Phase
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-086 Start of Main Phase")
        effect2.set_effect_description("[Start of Your Main Phase] By placing 1 black Digimon card with the [Cyborg] or [Machine] trait with a play cost of 4 or less from your hand or trash at the bottom of your Digimon with such trait, flip your opponent's top face down security card face up.")

        def condition2(context: Dict[str, Any]) -> bool:
            player = context.get("player")
            if not player or not player.is_my_turn:
                return False
            return True

        def process2():
            print("BT20-086 Start of Main Phase effect triggered (Complex interaction Not Implemented).")

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Effect 3: Security
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-086 Security")
        effect3.set_effect_description("[Security] Play this card without paying the cost.")
        effect3.is_security_effect = True

        def process3():
            if card.owner:
                # Play card logic.
                # Note: CardSource doesn't have play_card logic itself, Player does.
                card.owner.play_card(card)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
