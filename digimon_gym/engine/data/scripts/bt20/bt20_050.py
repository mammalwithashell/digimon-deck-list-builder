from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_050(CardScript):
    """BT20-050 HoverEspimon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-050 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: with [Cyborg] trait for cost 2
        effect0._alt_digi_cost = 2
        effect0._alt_digi_trait = "Cyborg"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Cyborg' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])) or any('Machine' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Flip your opponent's top face-down security card face up.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-050 Flip security card face up")
        effect1.set_effect_description("[When Digivolving] Flip your opponent's top face-down security card face up.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Flip Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Flip opponent's top face-down security card face up
            enemy = player.enemy if player else None
            if enemy and enemy.security_cards:
                pass  # Security flip — engine handles face-up/face-down state

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEndAttack
        # [End of Attack] [Once Per Turn] <Draw 1>
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-050 <Draw> 1")
        effect2.set_effect_description("[End of Attack] [Once Per Turn] <Draw 1>")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("EOA_BT20-050")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: dp_modifier
        # DP modifier
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-050 DP modifier")
        effect3.set_effect_description("DP modifier")
        effect3.is_inherited_effect = True
        effect3.dp_modifier = 1000

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
