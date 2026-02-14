from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_021(CardScript):
    """BT20-021 Jesmon GX | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Jogress Condition
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-021 Jogress Condition")
        effect0.set_effect_description("Jogress Condition")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Jogress Condition"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DNA/Jogress digivolution condition — handled by engine
            pass

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: blast_digivolve
        # Blast Digivolve
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-021 Blast Digivolve")
        effect1.set_effect_description("Blast Digivolve")
        effect1.is_counter_effect = True
        effect1._is_blast_digivolve = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] [Once Per Turn] Place 1 [Royal Knight] trait card from your hand or trash as this Digimon's bottom digivolution card, delete 1 of your opponent's Digimon with as much or less DP as this digimon.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-021 Select 1 card, delete 1 card")
        effect2.set_effect_description("[On Play] [Once Per Turn] Place 1 [Royal Knight] trait card from your hand or trash as this Digimon's bottom digivolution card, delete 1 of your opponent's Digimon with as much or less DP as this digimon.")
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Delete_BT20_021")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def hand_filter(c):
                if not (any('Royal Knight' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] [Once Per Turn] Place 1 [Royal Knight] trait card from your hand or trash as this Digimon's bottom digivolution card, delete 1 of your opponent's Digimon with as much or less DP as this digimon.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-021 Select 1 card, delete 1 card")
        effect3.set_effect_description("[When Digivolving] [Once Per Turn] Place 1 [Royal Knight] trait card from your hand or trash as this Digimon's bottom digivolution card, delete 1 of your opponent's Digimon with as much or less DP as this digimon.")
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("Delete_BT20_021")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def hand_filter(c):
                if not (any('Royal Knight' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] By placing 1 [Royal Knight] trait card from your hand or trash as this Digimon's bottom digivolution card, delete 1 of your opponent's Digimon with as much or less DP as this digimon.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT20-021 Select 1 card, delete 1 card")
        effect4.set_effect_description("[When Attacking] [Once Per Turn] By placing 1 [Royal Knight] trait card from your hand or trash as this Digimon's bottom digivolution card, delete 1 of your opponent's Digimon with as much or less DP as this digimon.")
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("Delete_BT20_021")
        effect4.is_on_attack = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def hand_filter(c):
                if not (any('Royal Knight' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once per Turn] This Digimon unsuspends. Then, for every 2 [Royal Knight] trait cards in this Digimon's digivolution cards, trash your opponent's top security card
        effect5 = ICardEffect()
        effect5.set_effect_name("BT20-021 Unsuspend, Then for every 2 [Royal Knight] traits in sources, trash opponent's top security")
        effect5.set_effect_description("[When Attacking] [Once per Turn] This Digimon unsuspends. Then, for every 2 [Royal Knight] trait cards in this Digimon's digivolution cards, trash your opponent's top security card")
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("Unsuspend_BT20_021")
        effect5.is_on_attack = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Unsuspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=False)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
