from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_052(CardScript):
    """BT20-052 Oblivimon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-052 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: with [Cyborg] trait for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_trait = "Cyborg"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Cyborg' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])) or any('Machine' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEndTurn
        # (Security) [End of Opponent's Turn] Play this card without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-052 Play this card")
        effect1.set_effect_description("(Security) [End of Opponent's Turn] Play this card without paying the cost.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
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

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Flip your opponent's top face-down security card face up.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-052 Flip security card face up")
        effect2.set_effect_description("[When Digivolving] Flip your opponent's top face-down security card face up.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Flip Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Flip opponent's top face-down security card face up
            enemy = player.enemy if player else None
            if enemy and enemy.security_cards:
                pass  # Security flip — engine handles face-up/face-down state

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnSecurityCheck
        # [Your Turn] When your Digimon check face-up security cards, you may place this Digimon's top stacked card face up as the bottom security card.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-052 Place top card face up as bottom security")
        effect3.set_effect_description("[Your Turn] When your Digimon check face-up security cards, you may place this Digimon's top stacked card face up as the bottom security card.")
        effect3.is_optional = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and len(permanent.digivolution_cards) >= 0):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Add To Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add top card of deck to security
            if player:
                player.recovery(1)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.None
        # Target Lock
        effect4 = ICardEffect()
        effect4.set_effect_name("BT20-052 This Digimon's attack target can't be switched.")
        effect4.set_effect_description("Target Lock")
        effect4.is_inherited_effect = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Target Lock"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Target lock — this Digimon's attack target can't be switched
            pass  # Handled by engine attack target resolution

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
