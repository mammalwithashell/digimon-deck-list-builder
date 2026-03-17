from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_029(CardScript):
    """BT21-029 Medusamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: progress
        # Progress
        effect0 = ICardEffect()
        effect0.set_effect_name("BT21-029 Progress")
        effect0.set_effect_description("Progress")
        effect0._is_progress = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: security_attack_plus
        # Security Attack +1
        effect1 = ICardEffect()
        effect1.set_effect_name("BT21-029 Security Attack +1")
        effect1.set_effect_description("Security Attack +1")
        effect1._security_attack_modifier = 1

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT21-029 Delete lowest DP Digimon")
        effect2.set_effect_description("Effect")
        effect2.set_hash_string("Delete_BT21_029")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_max_count_per_turn(1)
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete 1 of opponent's lowest DP Digimon (When Digivolving)"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            opponents = list(enemy.battle_area)
            if not opponents:
                return
            min_dp = min((getattr(p.top_card, 'dp', 0) or 0) for p in opponents if p.top_card)
            lowest = [p for p in opponents if p.top_card and (getattr(p.top_card, 'dp', 0) or 0) == min_dp]
            if lowest:
                def on_delete(selected, _enemy=enemy):
                    _enemy.delete_permanent(selected)
                game.effect_select_opponent_permanent(
                    player,
                    on_delete,
                    lambda p: p in lowest,
                    is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEndAttack
        # Effect
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEndAttack)
        effect3.set_effect_name("BT21-029 Delete lowest DP Digimon")
        effect3.set_effect_description("Effect")
        effect3.set_hash_string("Delete_BT21_029")
        effect3.set_max_count_per_turn(1)

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Delete 1 of opponent's lowest DP Digimon (End of Attack)"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            opponents = list(enemy.battle_area)
            if not opponents:
                return
            min_dp = min((getattr(p.top_card, 'dp', 0) or 0) for p in opponents if p.top_card)
            lowest = [p for p in opponents if p.top_card and (getattr(p.top_card, 'dp', 0) or 0) == min_dp]
            if lowest:
                def on_delete(selected, _enemy=enemy):
                    _enemy.delete_permanent(selected)
                game.effect_select_opponent_permanent(
                    player,
                    on_delete,
                    lambda p: p in lowest,
                    is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # [All Turns] [Once Per Turn] When any of your opponent's Digimon are deleted, they play 1 [Petrification] Token
        # Uses NoTiming + _is_deletion_observer so _fire_deletion_observers() picks it up
        effect4 = ICardEffect()
        effect4.set_effect_name("BT21-029 Play 1 [Petrification Token]")
        effect4.set_effect_description("[All Turns] [Once Per Turn] When any of your opponent's Digimon are deleted, they play 1 [Petrification] Token")
        effect4.set_hash_string("PlayToken_BT21_029")
        effect4._is_deletion_observer = True
        effect4.set_max_count_per_turn(1)

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only trigger when an opponent's Digimon is deleted
            deleted_perm = context.get('deleted_permanent')
            if not deleted_perm:
                return False
            if not getattr(deleted_perm, 'is_digimon', False):
                return False
            # _fire_deletion_observers(deleted_perm, deleted_owner) scans both sides.
            # Medusamon is found in the enemy-scan branch where context['player'] = card.owner.
            # If context['player'] is card.owner, the deleted perm belonged to the opponent.
            owner = card.owner if card else None
            if context.get('player') is not owner:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Play Petrification Token on opponent's field"""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                enemy = player.enemy if player else None
                if enemy:
                    game.effect_play_token(enemy, 'petrification')

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnLoseSecurity
        # [All Turns] [Once Per Turn] When opponents security stack is removed from, they play 1 [Petrification] Token
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnLoseSecurity)
        effect5.set_effect_name("BT21-029 Play 1 [Petrification Token]")
        effect5.set_effect_description("[All Turns] [Once Per Turn] When opponents security stack is removed from, they play 1 [Petrification] Token")
        effect5.set_hash_string("PlayToken_BT21_029")

        effect5.set_max_count_per_turn(1)

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Trigger when opponent's security is removed (All Turns)
            # context['player'] is set to the permanent owner (Medusamon's owner),
            # context['event_player'] is the player who actually lost security.
            # We want to trigger only when the opponent lost security.
            event_player = context.get('event_player') or context.get('player')
            owner = card.owner if card else None
            if event_player is owner:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Play Petrification Token on opponent's field"""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                enemy = player.enemy if player else None
                if enemy:
                    game.effect_play_token(enemy, 'petrification')

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
