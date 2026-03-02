from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX1_066(CardScript):
    """EX1-066 Analog Youth | Tamer White"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [On Play] Reveal top 3, add 1 Digimon card to hand,
        #     trash the remaining ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX1-066 Reveal top 3, add 1 Digimon to hand")
        effect0.set_effect_description(
            "[On Play] Reveal the top 3 cards of your deck. Add 1 Digimon "
            "card among them to your hand. Trash the remaining cards."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Reveal top 3, add 1 Digimon, trash rest"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Reveal top 3
            revealed = []
            for _ in range(min(3, len(player.library_cards))):
                revealed.append(player.library_cards.pop(0))

            added = []
            # Add 1 Digimon card to hand
            for c in revealed:
                if c.is_digimon:
                    added.append(c)
                    player.hand_cards.append(c)
                    break

            # Trash the remaining
            remaining = [c for c in revealed if c not in added]
            for c in remaining:
                player.trash_cards.append(c)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [All Turns] When a Lv.5+ Digimon with digivolution
        #     cards is deleted, suspend this Tamer to gain 1 memory and hatch ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDestroyedAnyone)
        effect1.set_effect_name("EX1-066 On ally Lv5+ deletion: gain memory + hatch")
        effect1.set_effect_description(
            "[All Turns] When one of your level 5 or higher Digimon with "
            "digivolution cards is deleted, you may suspend this Tamer. If "
            "you do, gain 1 memory, then hatch 1 Digi-Egg card to an empty "
            "space in your Breeding Area."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            # Must not already be suspended
            if perm and perm.is_suspended:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend this Tamer, gain 1 memory, hatch"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game and card):
                return
            perm = card.permanent_of_this_card()
            if not perm:
                return
            # Suspend this Tamer
            perm.suspend()
            # Gain 1 memory
            player.add_memory(1)
            # Hatch: if breeding area is empty and there are eggs
            if player.breeding_area is None and player.digitama_library_cards:
                player.hatch()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Play this card without paying the cost ---
        effect2 = ICardEffect()
        effect2.set_effect_name("EX1-066 Security Effect")
        effect2.set_effect_description("[Security] Play this card without paying the cost.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
