from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_053(CardScript):
    """EX10-053 Regulusmon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-053 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 5
        effect0._alt_digi_cost = 5

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blocker
        # Blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-053 Blocker")
        effect1.set_effect_description("Blocker")
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: rush
        # Rush
        effect2 = ICardEffect()
        effect2.set_effect_name("EX10-053 Rush")
        effect2.set_effect_description("Rush")
        effect2._is_rush = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may place up to 5 differently named Digimon cards with [Gammamon] in their names from your trash as this Digimon's bottom digivolution cards. Then, delete 1 of your opponent's Digimon with as much or less DP as this Digimon.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX10-053 Place up to 5 [Gammamon] Digimon from trash to digivolution cards, then delete 1 Digimon")
        effect3.set_effect_description("[On Play] You may place up to 5 differently named Digimon cards with [Gammamon] in their names from your trash as this Digimon's bottom digivolution cards. Then, delete 1 of your opponent's Digimon with as much or less DP as this Digimon.")
        effect3.is_on_play = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if not (p.contains_card_name('Gammamon')):
                    return False
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may place up to 5 differently named Digimon cards with [Gammamon] in their names from your trash as this Digimon's bottom digivolution cards. Then, delete 1 of your opponent's Digimon with as much or less DP as this Digimon.
        effect4 = ICardEffect()
        effect4.set_effect_name("EX10-053 Place up to 5 [Gammamon] Digimon from trash to digivolution cards, then delete 1 Digimon")
        effect4.set_effect_description("[When Digivolving] You may place up to 5 differently named Digimon cards with [Gammamon] in their names from your trash as this Digimon's bottom digivolution cards. Then, delete 1 of your opponent's Digimon with as much or less DP as this Digimon.")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if not (p.contains_card_name('Gammamon')):
                    return False
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] [Once Per Turn] If this Digimon has 5 or more digivolution cards, it may attack without suspending.
        effect5 = ICardEffect()
        effect5.set_effect_name("EX10-053 Attack without suspending")
        effect5.set_effect_description("[End of Your Turn] [Once Per Turn] If this Digimon has 5 or more digivolution cards, it may attack without suspending.")
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("EOT_EX10-053")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and len(permanent.digivolution_cards) >= 5):
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Force Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Force attack — target Digimon may attack (requires engine SelectAttack)
            pass  # descriptive-tagged: force_attack

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [Your Turn] [Once Per Turn] When any of your opponent's Digimon are deleted, gain 1 memory.
        effect6 = ICardEffect()
        effect6.set_effect_name("EX10-053 Memory +1")
        effect6.set_effect_description("[Your Turn] [Once Per Turn] When any of your opponent's Digimon are deleted, gain 1 memory.")
        effect6.is_inherited_effect = True
        effect6.set_max_count_per_turn(1)
        effect6.set_hash_string("Memory+1_EX10_053")
        effect6.is_on_deletion = True

        effect = effect6  # alias for condition closure
        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        return effects
