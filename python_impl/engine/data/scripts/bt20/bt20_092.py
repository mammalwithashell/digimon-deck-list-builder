from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming
from ....core.permanent import Permanent

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class BT20_092(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Start Turn (Memory Setter)
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-092 Start Turn")
        effect1.set_effect_description("[Start of Your Turn] If you have 2 or less memory, set it to 3.")

        def condition1(context: Dict[str, Any]) -> bool:
            if context.get("timing") != EffectTiming.OnStartTurn: return False
            player = context.get("player")
            if not player or not player.is_my_turn: return False
            return True

        def on_process1(context: Dict[str, Any]):
            game = context.get("game")
            if game and game.memory <= 2:
                game.memory = 3
                print("BT20-092: Memory set to 3.")

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(on_process1)
        effects.append(effect1)

        # On Play
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-092 On Play")
        effect2.set_effect_description("[On Play] Place 1 Level 3 Digimon from hand under -> Draw 1.")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if context.get("timing") != EffectTiming.OnEnterFieldAnyone: return False
            return True

        def on_process2(context: Dict[str, Any]):
            player = context.get("player")
            perm = context.get("permanent")
            if not player or not perm: return

            targets = [c for c in player.hand_cards if c.level == 3 and c.is_digimon]
            if targets:
                target = targets[0]
                player.hand_cards.remove(target)
                perm.card_sources.append(target)
                print(f"BT20-092: Placed {target.card_names[0]} under.")
                player.draw()

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(on_process2)
        effects.append(effect2)

        # Start Main
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-092 Start Main")
        effect3.set_effect_description("[Start of Your Main Phase] If no Digimon, play <=3 cost from under -> Delete self.")

        def condition3(context: Dict[str, Any]) -> bool:
            if context.get("timing") != EffectTiming.OnStartMainPhase: return False
            player = context.get("player")
            if not player.is_my_turn: return False
            has_digimon = any(p.top_card and p.top_card.is_digimon for p in player.battle_area)
            if has_digimon: return False
            return True

        def on_process3(context: Dict[str, Any]):
            player = context.get("player")
            perm = context.get("permanent")
            game = context.get("game")

            if not perm: return
            sources = perm.card_sources[1:]
            targets = [c for c in sources if c.is_digimon and c.get_cost_itself <= 3]

            if targets:
                target = targets[0]
                perm.card_sources.remove(target)

                # Manually play card since play_card assumes hand
                new_perm = Permanent([target])
                player.battle_area.append(new_perm)

                print(f"BT20-092: Played {target.card_names[0]} from under.")
                game.execute_effects(EffectTiming.OnEnterFieldAnyone)

                player.delete_permanent(perm)
                print("BT20-092: Deleted self.")

        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(on_process3)
        effects.append(effect3)

        # Security
        effect4 = ICardEffect()
        effect4.is_security_effect = True
        effect4.set_effect_name("BT20-092 Security")
        effect4.set_effect_description("[Security] Play this card without paying the cost.")
        def on_process4(context: Dict[str, Any]):
             player = context.get("player")
             game = context.get("game")
             if player:
                 player.play_card(card)
                 print("BT20-092: Played from Security.")
                 game.execute_effects(EffectTiming.OnEnterFieldAnyone)
        effect4.set_on_process_callback(on_process4)
        effects.append(effect4)

        return effects
