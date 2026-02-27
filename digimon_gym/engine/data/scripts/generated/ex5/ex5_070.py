from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX5_070(CardScript):
    """EX5-070 X Antibody Proto Form"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Also Treated As
        effect0 = ICardEffect()
        effect0.set_effect_name("EX5-070 Also treated as [X Antibody]")
        effect0.set_effect_description("Also Treated As")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Also Treated As"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Also treated as [Name] — name aliasing not modeled in engine
            pass  # descriptive-tagged: also_treated_as_name

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Ignore Color Req
        effect1 = ICardEffect()
        effect1.set_effect_name("EX5-070 Ignore color requirements")
        effect1.set_effect_description("Ignore Color Req")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Ignore Color Req"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Ignores color requirement for playing Options — not modeled in engine
            pass  # descriptive-tagged

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.SecuritySkill
        # [Security] Add this card to its owner's hand.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX5-070 Add To Hand")
        effect2.set_effect_description("[Security] Add this card to its owner's hand.")
        effect2.is_security_effect = True
        effect2.is_security_effect = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Add To Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OptionSkill
        # [Main] 1 of your Digimon without [X Antibody] in its digivolution cards may digivolve into a Digimon card with the [X Antibody] trait in your hand with the digivolution cost reduced by 1. If it did, place this card as its bottom digivolution card.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX5-070 Digivolve")
        effect3.set_effect_description("[Main] 1 of your Digimon without [X Antibody] in its digivolution cards may digivolve into a Digimon card with the [X Antibody] trait in your hand with the digivolution cost reduced by 1. If it did, place this card as its bottom digivolution card.")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When this Digimon would leave the battle area other than by one of your effects, from this Digimon's digivolution cards, return 1 Digimon card to the hand and place 1 [X Antibody] on top of your security stack.
        effect4 = ICardEffect()
        effect4.set_effect_name("EX5-070 Return 1 card from digivolution cards and place 1 [XAntibody] to the top of security")
        effect4.set_effect_description("[All Turns] When this Digimon would leave the battle area other than by one of your effects, from this Digimon's digivolution cards, return 1 Digimon card to the hand and place 1 [X Antibody] on top of your security stack.")
        effect4.is_inherited_effect = True
        effect4.set_hash_string("ReturnCard_EX5_070")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Add To Hand, Add To Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            # Add top card of deck to security
            if player:
                player.recovery(1)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
