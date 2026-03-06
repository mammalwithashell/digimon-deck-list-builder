from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_060(CardScript):
    """BT20-060 Alphamon: Ouryuken | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Jogress Condition
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-060 Jogress Condition")
        effect0.set_effect_description("Jogress Condition")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blast_dna_digivolve
        # Blast DNA Digivolve
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-060 Blast DNA Digivolve")
        effect1.set_effect_description("Blast DNA Digivolve")
        effect1.is_counter_effect = True
        effect1._is_blast_dna_digivolve = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] 1 of your opponent's Digimon gets -15000 DP until the end of their turn. Then, if DNA digivolving, trash your opponent's top security card and <Recovery +1 (Deck)>.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT20-060 One of your opponent's Digimon gets -15000 DP")
        effect2.set_effect_description("[On Play] 1 of your opponent's Digimon gets -15000 DP until the end of their turn. Then, if DNA digivolving, trash your opponent's top security card and <Recovery +1 (Deck)>.")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Select opponent Digimon for -15000 DP, then if DNA digivolving trash security + recovery"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_target(target_perm):
                target_perm.change_dp(-15000)

                # Then, if DNA digivolving, trash opponent's top security card and Recovery +1
                # TODO: check is_dna_digivolve in context — On Play doesn't receive it directly
                # For On Play, DNA digivolving is not applicable (only When Digivolving gets the flag)

            enemy = player.enemy if player else None
            has_target = enemy and any(p.is_digimon for p in enemy.battle_area)
            if has_target:
                game.effect_select_opponent_permanent(
                    player, on_target,
                    filter_fn=lambda p: p.is_digimon,
                    is_optional=False,
                    prompt="Select 1 of your opponent's Digimon to give -15000 DP.")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] 1 of your opponent's Digimon gets -15000 DP until the end of their turn. Then, if DNA digivolving, trash your opponent's top security card and <Recovery +1 (Deck)>.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT20-060 One of your opponent's Digimon gets -15000 DP")
        effect3.set_effect_description("[When Digivolving] 1 of your opponent's Digimon gets -15000 DP until the end of their turn. Then, if DNA digivolving, trash your opponent's top security card and <Recovery +1 (Deck)>.")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Select opponent Digimon for -15000 DP, then if DNA digivolving trash security + recovery"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_target(target_perm):
                target_perm.change_dp(-15000)

                # Then, if DNA digivolving, trash opponent's top security card and Recovery +1
                if ctx.get('is_dna_digivolve'):
                    enemy = player.enemy if player else None
                    if enemy and enemy.security_cards:
                        enemy.trash_security_card(enemy.security_cards[0])
                    player.recovery(1)

            enemy = player.enemy if player else None
            has_target = enemy and any(p.is_digimon for p in enemy.battle_area)
            if has_target:
                game.effect_select_opponent_permanent(
                    player, on_target,
                    filter_fn=lambda p: p.is_digimon,
                    is_optional=False,
                    prompt="Select 1 of your opponent's Digimon to give -15000 DP.")

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnLoseSecurity
        # [All Turns] (Once Per Turn) When security stacks are removed from, gain 3 memory.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnLoseSecurity)
        effect4.set_effect_name("BT20-060 Gain 3 memory")
        effect4.set_effect_description("[All Turns] [Once Per Turn] When security stacks are removed from, gain 3 memory.")
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("RemovedSec_BT20_060")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Gain 3 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(3)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Inherited: Ace Overflow <-5>
        # Engine handles ACE Overflow via _apply_ace_overflow in player.py
        # when the card leaves the field. The is_ace and ace_overflow_cost
        # attributes on the card entity base drive this behavior automatically.
        effect5 = ICardEffect()
        effect5.set_effect_name("BT20-060 Ace Overflow")
        effect5.set_effect_description("Ace Overflow <-5>")
        effect5.is_inherited_effect = True
        pass  # descriptive-tagged: ace_overflow

        def condition5(context: Dict[str, Any]) -> bool:
            return True
        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
