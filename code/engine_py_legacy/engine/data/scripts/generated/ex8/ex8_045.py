from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_045(CardScript):
    """EX8-045 Callismon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX8-045 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.5 with [NSo] trait for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5
        effect0._alt_digi_trait = "NSo"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('NSo' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Jogress Condition
        effect1 = ICardEffect()
        effect1.set_effect_name("EX8-045 Jogress Condition")
        effect1.set_effect_description("Jogress Condition")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Suspend 1 of your opponent's Digimon or Tamers. Then, return 1 of their suspended Tamers to the bottom of the deck.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX8-045 Suspend and then return Tamer to bottom of deck")
        effect2.set_effect_description("[When Digivolving] Suspend 1 of your opponent's Digimon or Tamers. Then, return 1 of their suspended Tamers to the bottom of the deck.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Suspend, Bounce"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_bounce(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.bounce_permanent_to_hand(target_perm)
            game.effect_select_opponent_permanent(
                player, on_bounce, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: security_attack_plus
        # Security Attack +1
        effect3 = ICardEffect()
        effect3.set_effect_name("EX8-045 Security Attack +1")
        effect3.set_effect_description("Security Attack +1")
        effect3._security_attack_modifier = 1

        def condition3(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Factory effect: dp_modifier
        # DP modifier
        effect4 = ICardEffect()
        effect4.set_effect_name("EX8-045 DP modifier")
        effect4.set_effect_description("DP modifier")
        effect4.dp_modifier = 0

        def condition4(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
