from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_050(CardScript):
    """BT23-050 Ankylomon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-050 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.3 from [Armadillomon] for cost 2
        effect0._alt_digi_cost = 2
        effect0._alt_digi_level = 3
        effect0._alt_digi_name = "Armadillomon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Armadillomon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blocker
        # Blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-050 Blocker")
        effect1.set_effect_description("Blocker")
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] 1 of your opponent's Digimon gets -2000 DP until their turn ends. Then, if it's your turn, 2 of your Digimon may DNA digivolve into [Shakkoumon] in the hand.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT23-050 Give -2000 DP, then if its your turn, you may DNA")
        effect2.set_effect_description("[On Play] 1 of your opponent's Digimon gets -2000 DP until their turn ends. Then, if it's your turn, 2 of your Digimon may DNA digivolve into [Shakkoumon] in the hand.")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: DP -2000, DNA digivolve into Shakkoumon"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            # Player chooses 1 opponent Digimon to give -2000 DP
            def dp_filter(p):
                return p.is_digimon and p.dp is not None
            def on_dp_reduce(target_perm):
                target_perm.change_dp(-2000)
                # Then, if it's your turn, 2 of your Digimon may DNA digivolve
                if player.is_my_turn:
                    def shakkoumon_filter(c):
                        return any('Shakkoumon' in n for n in getattr(c, 'card_names', []))
                    game.effect_dna_digivolve_from_hand(
                        player, shakkoumon_filter, is_optional=True,
                        prompt="Select Shakkoumon from hand for DNA digivolve.")
            game.effect_select_opponent_permanent(
                player, on_dp_reduce, filter_fn=dp_filter, is_optional=False,
                prompt="Select 1 opponent Digimon to give -2000 DP.")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Same effect as On Play
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT23-050 Give -2000 DP, then if its your turn, you may DNA")
        effect3.set_effect_description("[When Digivolving] 1 of your opponent's Digimon gets -2000 DP until their turn ends. Then, if it's your turn, 2 of your Digimon may DNA digivolve into [Shakkoumon] in the hand.")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: DP -2000, DNA digivolve into Shakkoumon"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def dp_filter(p):
                return p.is_digimon and p.dp is not None
            def on_dp_reduce(target_perm):
                target_perm.change_dp(-2000)
                if player.is_my_turn:
                    def shakkoumon_filter(c):
                        return any('Shakkoumon' in n for n in getattr(c, 'card_names', []))
                    game.effect_dna_digivolve_from_hand(
                        player, shakkoumon_filter, is_optional=True,
                        prompt="Select Shakkoumon from hand for DNA digivolve.")
            game.effect_select_opponent_permanent(
                player, on_dp_reduce, filter_fn=dp_filter, is_optional=False,
                prompt="Select 1 opponent Digimon to give -2000 DP.")

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
