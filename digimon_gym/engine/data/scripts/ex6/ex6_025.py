from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_025(CardScript):
    """EX6-025 Sanzomon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-025 Effect")
        effect0.set_effect_description("Effect")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] [Once Per Turn] 1 Digimon may gain [Security Attack -1] until the end of your opponent's turn. Then, if DigiXrosing, reveal the top 4 cards of your deck. Add 1 of each [Gokuumon], [Sagomon], [Cho-Hakkaimon] and [Shakamon] among them to the hand. Return the rest to the bottom of the deck.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX6-025 1 Digimon may gain [Security Attack -1] until the end of your opponent's turn. Then, if DigiXrosing, reveal the top 4 cards of your deck. Add 1 [Gokuumon], 1 [Sagomon], 1 [Cho-Hakkaimon] and 1 [Shakamon] among them to the hand. Return the rest to the bottom of the deck.")
        effect1.set_effect_description("[On Play] [Once Per Turn] 1 Digimon may gain [Security Attack -1] until the end of your opponent's turn. Then, if DigiXrosing, reveal the top 4 cards of your deck. Add 1 of each [Gokuumon], [Sagomon], [Cho-Hakkaimon] and [Shakamon] among them to the hand. Return the rest to the bottom of the deck.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("SecurityAttack-1Search_EX6-025")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Add To Hand, Change Security Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            # Grant Security Attack modifier to target permanent
            pass  # descriptive-tagged: change_security_attack

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] 1 Digimon may gain [Security Attack -1] until the end of your opponent's turn. Then, if DigiXrosing, reveal the top 4 cards of your deck. Add 1 of each [Gokuumon], [Sagomon], [Cho-Hakkaimon] and [Shakamon] among them to the hand. Return the rest to the bottom of the deck.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnAllyAttack)
        effect2.set_effect_name("EX6-025 1 Digimon may gain [Security Attack -1] until the end of your opponent's turn")
        effect2.set_effect_description("[When Attacking] [Once Per Turn] 1 Digimon may gain [Security Attack -1] until the end of your opponent's turn. Then, if DigiXrosing, reveal the top 4 cards of your deck. Add 1 of each [Gokuumon], [Sagomon], [Cho-Hakkaimon] and [Shakamon] among them to the hand. Return the rest to the bottom of the deck.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("SecurityAttack-1_EX6-025")
        effect2.is_on_attack = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Change Security Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant Security Attack modifier to target permanent
            pass  # descriptive-tagged: change_security_attack

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When this Digimon would leave the battle area, return 1 yellow Digimon card from this Digimon's digivolution cards to the hand.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.WhenRemoveField)
        effect3.set_effect_name("EX6-025 Return 1 yellow Digimon card from this Digimon's digivolution cards to the hand")
        effect3.set_effect_description("[All Turns] When this Digimon would leave the battle area, return 1 yellow Digimon card from this Digimon's digivolution cards to the hand.")
        effect3.set_hash_string("AllTurns_EX6_025")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Add To Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking][Once Per Turn] 1 Digimon may gain [Security Attack -1] until the end of your opponent's turn.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnAllyAttack)
        effect4.set_effect_name("EX6-025 Security Attack -1")
        effect4.set_effect_description("[When Attacking][Once Per Turn] 1 Digimon may gain [Security Attack -1] until the end of your opponent's turn.")
        effect4.is_inherited_effect = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("SecurityAttack-1_EX6-025")
        effect4.is_on_attack = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Change Security Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant Security Attack modifier to target permanent
            pass  # descriptive-tagged: change_security_attack

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
