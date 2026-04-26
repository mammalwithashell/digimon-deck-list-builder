from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_100(CardScript):
    """BT22-100 Cyberspace EDEN"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Ignore Color Req — while you have no face-up security cards
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-100 Ignore color requirements")
        effect0.set_effect_description("While you have no face-up security cards, you can ignore this card's color requirements.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Ignore Color Req"""
            # Ignores color requirement for playing Options — not modeled in engine
            pass  # descriptive-tagged

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [Security] [All Turns] All of your Digimon with the [CS] trait get +2000 DP.
        # This is a continuous effect while this card is face-up in security.
        # Modeled as a DP modifier that checks if card is in security.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT22-100 All your CS Digimon +2000 DP")
        effect1.set_effect_description("[Security] [All Turns] All of your Digimon with the [CS] trait get +2000 DP.")
        effect1.dp_modifier = 2000
        effect1._applies_to_all_own_digimon = True

        def condition1(context: Dict[str, Any]) -> bool:
            # This effect is active while the card is in the security stack (face-up)
            owner = getattr(card, 'owner', None)
            if not owner:
                return False
            if card not in owner.security_cards:
                return False
            # Check if the target permanent has CS trait
            perm = context.get('permanent')
            if perm:
                traits = getattr(perm.top_card, 'card_traits', []) or []
                if not any('CS' in t for t in traits):
                    return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OptionSkill
        # [Main] Add your bottom security card to the hand. Then, place this
        # card face up as the bottom security card.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OptionSkill)
        effect2.set_effect_name("BT22-100 Swap bottom security with this card")
        effect2.set_effect_description("[Main] Add your bottom security card to the hand. Then, place this card face up as the bottom security card.")

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Add bottom security to hand, place this as bottom security face-up"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Add bottom security card to hand
            if player.security_cards:
                bottom_card = player.security_cards.pop(0)
                player.hand_cards.append(bottom_card)
            # The engine handles placing this option card in battle area via
            # PlaceDelayOptionCards after OptionSkill resolves. For this card,
            # it should go to security instead. Place as bottom security card.
            # The card itself (as an option) will be placed in the battle area
            # by the engine's option placement flow, which also handles
            # "place this card in the battle area" semantics.
            # Per the C# ReplaceBottomSecurityWithFaceUpOptionMainEffect,
            # the card is placed face-up as bottom security instead of battle area.
            # In our engine, the option card placement is handled by the engine,
            # so we insert it into security_cards at index 0 (bottom).
            player.security_cards.insert(0, card)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.SecuritySkill
        # [Security] You may play 1 play cost 5 or lower card with the [CS]
        # trait from your hand without paying the cost.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("BT22-100 Play 1 CS card from hand")
        effect3.set_effect_description("[Security] You may play 1 play cost 5 or lower card with the [CS] trait from your hand without paying the cost.")
        effect3.is_optional = True
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play 1 CS card from hand free"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if getattr(c, 'get_cost_itself', 0) > 5:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                if not any('CS' in t for t in traits):
                    return False
                return True

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
