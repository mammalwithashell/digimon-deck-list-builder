from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_036(CardScript):
    """BT20-036 BanchoLeomon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-036 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: with [ACCEL] trait for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_trait = "ACCEL"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('ACCEL' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.BeforePayCost
        # When this card would be played, if you have a Digimon with the [ACCEL] trait, reduce the play cost by 5.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-036 Reduce the play cost by 5")
        effect1.set_effect_description("When this card would be played, if you have a Digimon with the [ACCEL] trait, reduce the play cost by 5.")
        effect1.set_hash_string("PlayCost-5_BT20_036")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Effect"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            pass

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.None
        # Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-036 Play Cost -5")
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
            pass

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] <De-Digivolve 2> 1 of your opponent's Digimon. Then, 1 of their Digimon gets -5000 DP until the end of their turn.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-036 <De-Digivolve 2> 1 of your opponent's Digimon, then 1 gets -5000 DP")
        effect3.set_effect_description("[On Play] <De-Digivolve 2> 1 of your opponent's Digimon. Then, 1 of their Digimon gets -5000 DP until the end of their turn.")
        effect3.is_on_play = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: DP -5000, De Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-5000)
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(2)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] <De-Digivolve 2> 1 of your opponent's Digimon. Then, 1 of their Digimon gets -5000 DP until the end of their turn.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT20-036 <De-Digivolve 2> 1 of your opponent's Digimon, then 1 gets -5000 DP")
        effect4.set_effect_description("[When Digivolving] <De-Digivolve 2> 1 of your opponent's Digimon. Then, 1 of their Digimon gets -5000 DP until the end of their turn.")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: DP -5000, De Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-5000)
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(2)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] This Digimon and any of your other Digimon may DNA digivolve into a Digimon card with [Chaosmon] in its name in the hand. Then, the DNA digivolved Digimon may attack.
        effect5 = ICardEffect()
        effect5.set_effect_name("BT20-036 DNA digivolve this Digimon")
        effect5.set_effect_description("[End of Your Turn] This Digimon and any of your other Digimon may DNA digivolve into a Digimon card with [Chaosmon] in its name in the hand. Then, the DNA digivolved Digimon may attack.")
        effect5.is_optional = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Play Card, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not (any('Chaosmon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def hand_filter(c):
                if not (any('Chaosmon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=True)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.OnAllyAttack
        # [Opponent's Turn] [Once Per Turn] When any of your opponent's Digimon attack, you may change the attack target to this Digimon.
        effect6 = ICardEffect()
        effect6.set_effect_name("BT20-036 You may change the attack target to this Digimon.")
        effect6.set_effect_description("[Opponent's Turn] [Once Per Turn] When any of your opponent's Digimon attack, you may change the attack target to this Digimon.")
        effect6.is_inherited_effect = True
        effect6.is_optional = True
        effect6.set_max_count_per_turn(1)
        effect6.set_hash_string("Redirect_BT20-036")
        effect6.is_on_attack = True

        effect = effect6  # alias for condition closure
        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect6.set_can_use_condition(condition6)
        effects.append(effect6)

        return effects
