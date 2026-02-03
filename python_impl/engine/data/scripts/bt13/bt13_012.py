from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
import random
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class BT13_012(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Effect 1: [When Digivolving] Search your security stack...
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-012 Main Effect")
        effect1.set_effect_description("[When Digivolving] Search your security stack, and you may play 1 red or yellow Tamer card among it without paying its cost. If you did, <Recovery +1 (Deck)>. Then, shuffle your security stack.")
        effect1.is_when_digivolving = True

        def on_process1():
            owner = card.owner
            if not owner: return

            # Search security stack
            candidates = []
            for c in owner.security_cards:
                if c.is_tamer:
                    # Check color: Red(0) or Yellow(2)
                    colors = [col.value for col in c.card_colors]
                    if 0 in colors or 2 in colors:
                        candidates.append(c)

            played_card = False
            if candidates:
                # Select one to play. Greedy: first one.
                target = candidates[0]
                print(f"Playing {target.card_names[0]} from security.")

                # Remove from security
                owner.security_cards.remove(target)

                # Play it
                owner.play_card(target)
                played_card = True
            else:
                print("No Red/Yellow Tamer found in security.")

            if played_card:
                owner.recovery(1)

            # Shuffle security
            random.shuffle(owner.security_cards)

        effect1.set_on_process_callback(on_process1)
        effects.append(effect1)

        # Effect 2: Inherited
        # [Your Turn] [Once Per Turn] When one of your red or yellow Tamers becomes suspended, you may delete 1 of your opponent's Digimon with 3000 DP or less.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT13-012 Inherited Effect")
        effect2.set_effect_description("[Your Turn] [Once Per Turn] When one of your red or yellow Tamers becomes suspended, you may delete 1 of your opponent's Digimon with 3000 DP or less.")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)

        def condition2(context: Dict[str, Any]) -> bool:
            if context.get("timing") != EffectTiming.OnTappedAnyone: return False

            game = context.get("game")
            player = context.get("player")
            if not game or not player: return False
            if game.turn_player != player: return False

            # Assumes context has caused_by_permanent for OnTappedAnyone
            target = context.get("caused_by_permanent")
            if not target: return False
            if target.top_card.owner != player: return False
            if not target.is_tamer: return False

            # Color check
            colors = [col.value for col in target.top_card.card_colors]
            if not (0 in colors or 2 in colors): return False

            return True

        def on_process2():
            owner = card.owner
            if not owner or not owner.game: return
            opponent = owner.game.opponent_player

            targets = [p for p in opponent.battle_area if p.is_digimon and p.dp <= 3000]
            if targets:
                t = targets[0]
                print(f"Deleting {t.top_card.card_names[0]} due to Tamer suspension.")
                opponent.delete_permanent(t)

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(on_process2)
        effects.append(effect2)

        return effects
