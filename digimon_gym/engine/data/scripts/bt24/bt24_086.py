from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_086(CardScript):
    """BT24-086 The Crossroad Witch"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-086 Also treated as [Shuu Yulin]")
        effect0.set_effect_description("Effect")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: security_play
        # Security: Play this card
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-086 Security: Play this card")
        effect1.set_effect_description("Security: Play this card")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-086 Memory +1")
        effect2.set_effect_description("[Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [All Turns] When any of your Digimon are played or digivolve, you may <Mind Link> with 1 of your Digimon with the [X Antibody], [DigiPolice] or [SEEKERS] trait.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-086 Mind Link")
        effect3.set_effect_description("[All Turns] When any of your Digimon are played or digivolve, you may <Mind Link> with 1 of your Digimon with the [X Antibody], [DigiPolice] or [SEEKERS] trait.")
        effect3.is_optional = True
        effect3.is_on_play = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Mind Link"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            game.effect_link_to_permanent(player, card, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Factory effect: reboot
        # Reboot
        effect4 = ICardEffect()
        effect4.set_effect_name("BT24-086 Reboot")
        effect4.set_effect_description("Reboot")
        effect4.is_inherited_effect = True
        effect4._is_reboot = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('DigiPolice' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Factory effect: alliance
        # Alliance
        effect5 = ICardEffect()
        effect5.set_effect_name("BT24-086 Alliance")
        effect5.set_effect_description("Alliance")
        effect5.is_inherited_effect = True
        effect5._is_alliance = True

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('DigiPolice' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        # Timing: EffectTiming.OnEndTurn
        # [End of All Turns] You may play 1 [Shuu Yulin] from this Digimon's digivolution cards without paying the cost.
        effect6 = ICardEffect()
        effect6.set_effect_name("BT24-086 Play 1 [Shuu Yulin] from this Digimon's digivolution cards")
        effect6.set_effect_description("[End of All Turns] You may play 1 [Shuu Yulin] from this Digimon's digivolution cards without paying the cost.")
        effect6.is_inherited_effect = True
        effect6.is_optional = True

        effect = effect6  # alias for condition closure
        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
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

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        return effects
