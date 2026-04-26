from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

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
        effect2.set_timing(EffectTiming.BeforePayCost)
        effect2.set_effect_name("BT22-061 Reduce the digivolution cost by 1 for each face-down source")
        effect2.set_effect_description("Effect")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return False
            # Count face-down digivolution cards (all non-top card_sources)
            fd_count = max(0, len(perm.card_sources) - 1)
            return fd_count > 0

        effect2.set_can_use_condition(condition2)

        # Dynamic cost_reduction based on face-down source count
        def _get_fd_reduction(context=None):
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return 0
            return max(0, len(perm.card_sources) - 1)

        effect2._cost_reduction_value_fn = _get_fd_reduction

        def process2(ctx: Dict[str, Any]):
            """Reduce digivolution cost by 1 for each face-down source."""
            pass  # cost_reduction_fn handles the reduction declaratively

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
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
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
            """Action: De-Digivolve 1 opponent, then trash own bottom source to bounce cost 4-"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            # Step 1: <De-Digivolve 1> 1 of your opponent's Digimon
            def dedigivolve_filter(p):
                return p.is_digimon and len(p.card_sources) > 1

            def on_de_digivolve(target_perm):
                if target_perm is None:
                    return
                removed = target_perm.de_digivolve(1)
                enemy.trash_cards.extend(removed)

                # Step 2: By trashing this Digimon's bottom face-down digivolution
                # card, return 1 of your opponent's play cost 4 or lower Digimon
                # or Tamers to the hand.
                if not perm or len(perm.card_sources) <= 1:
                    return
                # Trash bottom digivolution card (index 0 is bottom)
                bottom_card = perm.card_sources[0]
                if bottom_card is perm.top_card:
                    return
                perm.card_sources.remove(bottom_card)
                player.trash_cards.append(bottom_card)

                # Bounce 1 opponent's play cost 4 or lower Digimon or Tamer
                def bounce_filter(p):
                    if not (p.is_digimon or p.is_tamer):
                        return False
                    cost = getattr(p.top_card, 'get_cost_itself', 0)
                    if isinstance(cost, property):
                        cost = 0
                    return cost <= 4

                def on_bounce(bounce_target):
                    if bounce_target is None:
                        return
                    enemy.bounce_permanent_to_hand(bounce_target)

                if any(bounce_filter(p) for p in enemy.battle_area):
                    game.effect_select_opponent_permanent(
                        player, on_bounce, filter_fn=bounce_filter,
                        is_optional=False,
                        prompt="Return 1 of your opponent's play cost 4 or lower Digimon or Tamer to hand.")

            if any(dedigivolve_filter(p) for p in enemy.battle_area):
                game.effect_select_opponent_permanent(
                    player, on_de_digivolve, filter_fn=dedigivolve_filter,
                    is_optional=False,
                    prompt="Select 1 of your opponent's Digimon to <De-Digivolve 1>.")

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] [Once Per Turn] <De-Digivolve 1> 1 of your opponent's Digimon. (Trash the top card. You can't trash past level 3 cards.) Then, by trashing this Digimon's bottom face-down digivolution card, return 1 of your opponent's play cost 4 or lower Digimon or Tamers to the hand.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnUseAttack)
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
            """Action: De-Digivolve 1 opponent, then trash own bottom source to bounce cost 4-"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            # Step 1: <De-Digivolve 1> 1 of your opponent's Digimon
            def dedigivolve_filter(p):
                return p.is_digimon and len(p.card_sources) > 1

            def on_de_digivolve(target_perm):
                if target_perm is None:
                    return
                removed = target_perm.de_digivolve(1)
                enemy.trash_cards.extend(removed)

                # Step 2: By trashing this Digimon's bottom face-down digivolution
                # card, return 1 of your opponent's play cost 4 or lower Digimon
                # or Tamers to the hand.
                if not perm or len(perm.card_sources) <= 1:
                    return
                bottom_card = perm.card_sources[0]
                if bottom_card is perm.top_card:
                    return
                perm.card_sources.remove(bottom_card)
                player.trash_cards.append(bottom_card)

                def bounce_filter(p):
                    if not (p.is_digimon or p.is_tamer):
                        return False
                    cost = getattr(p.top_card, 'get_cost_itself', 0)
                    if isinstance(cost, property):
                        cost = 0
                    return cost <= 4

                def on_bounce(bounce_target):
                    if bounce_target is None:
                        return
                    enemy.bounce_permanent_to_hand(bounce_target)

                if any(bounce_filter(p) for p in enemy.battle_area):
                    game.effect_select_opponent_permanent(
                        player, on_bounce, filter_fn=bounce_filter,
                        is_optional=False,
                        prompt="Return 1 of your opponent's play cost 4 or lower Digimon or Tamer to hand.")

            if any(dedigivolve_filter(p) for p in enemy.battle_area):
                game.effect_select_opponent_permanent(
                    player, on_de_digivolve, filter_fn=dedigivolve_filter,
                    is_optional=False,
                    prompt="Select 1 of your opponent's Digimon to <De-Digivolve 1>.")

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.OnUseAttack
        # [Opponent's Turn] [Once Per Turn] When one of your opponent's Digimon attacks, you may change the attack target to this Digimon.
        effect6 = ICardEffect()
        effect6.set_timing(EffectTiming.OnUseAttack)
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
