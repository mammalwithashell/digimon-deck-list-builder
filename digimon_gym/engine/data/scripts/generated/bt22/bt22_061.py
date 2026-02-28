from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_061(CardScript):
    """BT22-061 Vademon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-061 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.4 for cost 4
        effect0._alt_digi_cost = 4
        effect0._alt_digi_level = 4

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect1 = ICardEffect()
        effect1.set_effect_name("BT22-061 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Vegiemon] for cost 3
        effect1._alt_digi_cost = 3
        effect1._alt_digi_name = "Vegiemon"

        def condition1(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Vegiemon'))):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.BeforePayCost
        # Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT22-061 Reduce the digivolution cost by 1 for each face-down source")
        effect2.set_effect_description("Effect")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Effect"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction (variable amount) — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: fragment
        # Fragment
        effect3 = ICardEffect()
        effect3.set_effect_name("BT22-061 Fragment")
        effect3.set_effect_description("Fragment")
        effect3._is_fragment = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] [Once Per Turn] <De-Digivolve 1> 1 of your opponent's Digimon. (Trash the top card. You can't trash past level 3 cards.) Then, by trashing this Digimon's bottom face-down digivolution card, return 1 of your opponent's play cost 4 or lower Digimon or Tamers to the hand.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT22-061 De-Digivolve 1, then by trashing bottom FD source, bounce 1 level 4 or lower digimon or tamer to hand")
        effect4.set_effect_description("[When Digivolving] [Once Per Turn] <De-Digivolve 1> 1 of your opponent's Digimon. (Trash the top card. You can't trash past level 3 cards.) Then, by trashing this Digimon's bottom face-down digivolution card, return 1 of your opponent's play cost 4 or lower Digimon or Tamers to the hand.")
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("BT22_061_DeDigivolve1AndBounce")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Bounce, Trash Digivolution Cards, De Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
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
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] <De-Digivolve 1> 1 of your opponent's Digimon. (Trash the top card. You can't trash past level 3 cards.) Then, by trashing this Digimon's bottom face-down digivolution card, return 1 of your opponent's play cost 4 or lower Digimon or Tamers to the hand.
        effect5 = ICardEffect()
        effect5.set_effect_name("BT22-061 De-Digivolve 1, then by trashing bottom FD source, bounce 1 level 4 or lower digimon or tamer to hand")
        effect5.set_effect_description("[When Attacking] [Once Per Turn] <De-Digivolve 1> 1 of your opponent's Digimon. (Trash the top card. You can't trash past level 3 cards.) Then, by trashing this Digimon's bottom face-down digivolution card, return 1 of your opponent's play cost 4 or lower Digimon or Tamers to the hand.")
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("BT22_061_DeDigivolve1AndBounce")
        effect5.is_on_attack = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Bounce, Trash Digivolution Cards, De Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
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
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.OnAllyAttack
        # [Opponent's Turn] [Once Per Turn] When one of your opponent's Digimon attacks, you may change the attack target to this Digimon.
        effect6 = ICardEffect()
        effect6.set_effect_name("BT22-061 Change attack target to this card")
        effect6.set_effect_description("[Opponent's Turn] [Once Per Turn] When one of your opponent's Digimon attacks, you may change the attack target to this Digimon.")
        effect6.is_inherited_effect = True
        effect6.is_optional = True
        effect6.set_max_count_per_turn(1)
        effect6.set_hash_string("BT22_061_ChangeAttackTarget")
        effect6.is_on_attack = True

        effect = effect6  # alias for condition closure
        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            """Action: Redirect Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Redirect attack target (SwitchDefender) — not yet in engine
            pass  # descriptive-tagged: redirect_attack

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        return effects
