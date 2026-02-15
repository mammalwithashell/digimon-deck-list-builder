from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_087(CardScript):
    """BT14-087 Eiji Nagasumi"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: security_play
        # Security: Play this card
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-087 Security: Play this card")
        effect0.set_effect_description("Security: Play this card")
        effect0.is_security_effect = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-087 Memory +1")
        effect1.set_effect_description("[Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDeclaration
        # [Main] <Mind Link> with 1 of your Digimon with the [Dark Animal] or [SoC] trait. (Place this Tamer as that Digimon's bottom digivolution card if there are no Tamer cards in its digivolution cards.)
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-087 Mind Link")
        effect2.set_effect_description("[Main] <Mind Link> with 1 of your Digimon with the [Dark Animal] or [SoC] trait. (Place this Tamer as that Digimon's bottom digivolution card if there are no Tamer cards in its digivolution cards.)")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Mind Link"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            game.effect_link_to_permanent(player, card, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: blocker
        # Blocker
        effect3 = ICardEffect()
        effect3.set_effect_name("BT14-087 Blocker")
        effect3.set_effect_description("Blocker")
        effect3.is_inherited_effect = True
        effect3._is_blocker = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Factory effect: alliance
        # Alliance
        effect4 = ICardEffect()
        effect4.set_effect_name("BT14-087 Alliance")
        effect4.set_effect_description("Alliance")
        effect4.is_inherited_effect = True
        effect4._is_alliance = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEndTurn
        # [End of All Turns] You may play 1 [Eiji Nagasumi] from this Digimon's digivolution cards without paying the cost.
        effect5 = ICardEffect()
        effect5.set_effect_name("BT14-087 Play 1 [Eiji Nagasumi] from this Digimon's digivolution cards")
        effect5.set_effect_description("[End of All Turns] You may play 1 [Eiji Nagasumi] from this Digimon's digivolution cards without paying the cost.")
        effect5.is_inherited_effect = True
        effect5.is_optional = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
