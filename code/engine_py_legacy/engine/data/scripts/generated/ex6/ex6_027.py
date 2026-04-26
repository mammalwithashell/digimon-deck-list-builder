from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_027(CardScript):
    """EX6-027 Ophanimon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-027 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Angewomon] for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_name = "Angewomon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Angewomon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blast_digivolve
        # Blast Digivolve
        effect1 = ICardEffect()
        effect1.set_effect_name("EX6-027 Blast Digivolve")
        effect1.set_effect_description("Blast Digivolve")
        effect1.is_counter_effect = True
        effect1._is_blast_digivolve = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By trashing the top or bottom card of your security stack, 1 of your opponent's Digimon gets -8000 DP until the end of your opponents turn.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX6-027 By trashing the top or bottom card of your security stack, 1 of your opponent's Digimon gets -8000 DP until the end of your opponents turn.")
        effect2.set_effect_description("[On Play] By trashing the top or bottom card of your security stack, 1 of your opponent's Digimon gets -8000 DP until the end of your opponents turn.")
        effect2.is_optional = True
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: DP -8000, Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-8000)
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By trashing the top or bottom card of your security stack, 1 of your opponent's Digimon gets -8000 DP until the end of your opponents turn.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX6-027  By trashing the top or bottom card of your security stack, 1 of your opponent's Digimon gets -8000 DP until the end of your opponents turn.")
        effect3.set_effect_description("[When Digivolving] By trashing the top or bottom card of your security stack, 1 of your opponent's Digimon gets -8000 DP until the end of your opponents turn.")
        effect3.is_optional = True
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: DP -8000, Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-8000)
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnLoseSecurity
        # [All Turns][Once Per Turn] When a card is removed from your security stack, if it's your turn, this Digimon may gain [Security Attack +1] for the turn and attack. If it's your opponent's turn, [Recovery +1 <Deck>].
        effect4 = ICardEffect()
        effect4.set_effect_name("EX6-027 If it's your turn, this Digimon may gain [Security Attack +1] for the turn and attack")
        effect4.set_effect_description("[All Turns][Once Per Turn] When a card is removed from your security stack, if it's your turn, this Digimon may gain [Security Attack +1] for the turn and attack. If it's your opponent's turn, [Recovery +1 <Deck>].")
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("AllTurnsYourTurn_EX6_027")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Force Attack, Change Security Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Force attack — target Digimon may attack (requires engine SelectAttack)
            pass  # descriptive-tagged: force_attack
            # Grant Security Attack modifier to target permanent
            pass  # descriptive-tagged: change_security_attack

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnLoseSecurity
        # [All Turns][Once Per Turn] When a card is removed from your security stack, if it's your turn, this Digimon may gain [Security Attack +1] for the turn and attack. If it's your opponent's turn, [Recovery +1 <Deck>].
        effect5 = ICardEffect()
        effect5.set_effect_name("EX6-027 If it's your opponent's turn, [Recovery +1 <Deck>]")
        effect5.set_effect_description("[All Turns][Once Per Turn] When a card is removed from your security stack, if it's your turn, this Digimon may gain [Security Attack +1] for the turn and attack. If it's your opponent's turn, [Recovery +1 <Deck>].")
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("AllTurnsOpponentTurn_EX6_027")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Recovery +1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.recovery(1)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
