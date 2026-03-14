from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_072(CardScript):
    """EX6-072 Mega Digimon Assembly!"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Ignore Color Req
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-072 Ignore color requirements")
        effect0.set_effect_description("Ignore Color Req")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Ignore Color Req"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Ignores color requirement for playing Options — not modeled in engine
            pass  # descriptive-tagged

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: blast_dna_digivolve
        # Blast DNA Digivolve
        effect1 = ICardEffect()
        effect1.set_effect_name("EX6-072 Blast DNA Digivolve")
        effect1.set_effect_description("Blast DNA Digivolve")
        effect1.is_counter_effect = True
        effect1._is_blast_dna_digivolve = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OptionSkill
        # Play Card, Jogress Condition
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OptionSkill)
        effect2.set_effect_name("EX6-072 DNA Digivolve")
        effect2.set_effect_description("Play Card, Jogress Condition")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: 2 of your Digimon may DNA digivolve into a Lv.6+ Digimon from hand."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            # DNA digivolve: 2 of your Digimon may DNA digivolve into a Digimon
            # card with a play cost of 0 (free) that is Lv.6+
            def dna_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                lv = getattr(c, 'level', None)
                if lv is None or lv < 7:
                    return False
                return True
            game.effect_dna_digivolve_from_hand(player, dna_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.SecuritySkill
        # [Security] Return 1 level 6 or higher Digimon card from your trash to the hand. Then, add this card to the hand.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("EX6-072 Return 1 Digimon from trash to hand then add this card to hand")
        effect3.set_effect_description("[Security] Return 1 level 6 or higher Digimon card from your trash to the hand. Then, add this card to the hand.")
        effect3.is_security_effect = True
        effect3.is_security_effect = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Return 1 Lv.6+ Digimon from trash to hand, then add this card to hand."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            # Select a level 6+ Digimon from trash and return to hand
            def trash_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                lv = getattr(c, 'level', None)
                if lv is None or lv < 6:
                    return False
                return True
            matching = [c for c in player.trash_cards if trash_filter(c)]
            if matching:
                # Use the last matching card (most recently trashed)
                chosen = matching[-1]
                if chosen in player.trash_cards:
                    player.trash_cards.remove(chosen)
                    player.hand_cards.append(chosen)
            # Then add this card (the option) to hand
            if card:
                # The option card should be in trash after security check
                if card in player.trash_cards:
                    player.trash_cards.remove(card)
                    player.hand_cards.append(card)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
