from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming
from ....core.permanent import Permanent

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class BT20_085(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Effect 1: Start Main Phase
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-085 Start Main")
        effect1.set_effect_description("[Start of Your Main Phase] Return this Tamer to bottom of deck -> Play 1 [Shoto Kazama] from hand -> If no Digimon, play [Avian]/[Bird] from trash.")

        def condition1(context: Dict[str, Any]) -> bool:
            timing = context.get("timing")
            if timing != EffectTiming.OnStartMainPhase: return False
            player = context.get("player")
            if not player or not player.is_my_turn: return False
            perm = context.get("permanent")
            if not perm: return False
            return True

        def on_process1(context: Dict[str, Any]):
            game = context.get("game")
            player = context.get("player")
            perm = context.get("permanent")
            if not game or not player or not perm: return

            # Return to bottom of deck
            if perm in player.battle_area:
                player.battle_area.remove(perm)
                # Trash sources
                if perm.card_sources:
                    top = perm.top_card
                    sources = perm.card_sources[1:]
                    player.trash_cards.extend(sources)
                    player.library_cards.append(top)
                    print(f"BT20-085: Returned {top.card_names[0]} to bottom of deck.")

                # Play Shoto from hand
                shoto_cards = [c for c in player.hand_cards if any("shoto" in n.lower() and "kazama" in n.lower() for n in c.card_names)]
                if shoto_cards:
                     new_shoto = shoto_cards[0]
                     player.play_card(new_shoto)
                     # Trigger OnPlay
                     game.execute_effects(EffectTiming.OnEnterFieldAnyone)

                # If no Digimon
                has_digimon = any(p.top_card and p.top_card.is_digimon for p in player.battle_area)
                if not has_digimon:
                     # Play Level 3 Avian/Bird from Trash
                     targets = []
                     for t_card in player.trash_cards:
                         if t_card.level == 3 and ("Avian" in t_card.card_traits or "Bird" in t_card.card_traits):
                             targets.append(t_card)

                     if targets:
                         target = targets[0]
                         player.trash_cards.remove(target)
                         new_perm = Permanent([target])
                         player.battle_area.append(new_perm)
                         print(f"BT20-085: Played {target.card_names[0]} from trash.")
                         game.execute_effects(EffectTiming.OnEnterFieldAnyone)

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(on_process1)
        effects.append(effect1)

        # Effect 2: End Turn
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-085 End Turn")
        effect2.set_effect_description("[End of Your Turn] Suspend this Tamer -> Suspend opponent's Digimon, +2000 DP to Vortex Warrior.")

        def condition2(context: Dict[str, Any]) -> bool:
             timing = context.get("timing")
             if timing != EffectTiming.OnEndTurn: return False
             player = context.get("player")
             perm = context.get("permanent")
             if not player or not player.is_my_turn: return False
             if not perm or perm.is_suspended: return False
             return True

        def on_process2(context: Dict[str, Any]):
             game = context.get("game")
             player = context.get("player")
             perm = context.get("permanent")

             # Suspend self
             perm.suspend()
             print("BT20-085: Suspended self.")

             # Suspend 1 opponent Digimon
             opponent = game.opponent_player
             targets = [p for p in opponent.battle_area if p.top_card and p.top_card.is_digimon and not p.is_suspended]
             if targets:
                 target = targets[0]
                 target.suspend()
                 print(f"BT20-085: Suspended opponent {target.top_card.card_names[0]}")

             # Buff 1 Vortex Warrior
             allies = [p for p in player.battle_area if p.top_card and p.top_card.is_digimon and any("Vortex Warrior" in t for t in p.top_card.card_traits)]
             if allies:
                 ally = allies[0]
                 ally.temp_dp_modifier += 2000
                 # TODO: This buff should expire at the end of the opponent's turn.
                 # Currently, the engine does not support effect expiration management.
                 print(f"BT20-085: +2000 DP to {ally.top_card.card_names[0]}")

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(on_process2)
        effects.append(effect2)

        # Security
        effect3 = ICardEffect()
        effect3.is_security_effect = True
        effect3.set_effect_name("BT20-085 Security")
        effect3.set_effect_description("[Security] Play this card without paying the cost.")

        def on_process3(context: Dict[str, Any]):
             player = context.get("player")
             game = context.get("game")
             if player:
                 player.play_card(card)
                 print(f"BT20-085: Played from Security.")
                 game.execute_effects(EffectTiming.OnEnterFieldAnyone)

        effect3.set_on_process_callback(on_process3)
        effects.append(effect3)

        return effects
