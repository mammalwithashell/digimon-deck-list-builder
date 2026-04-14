from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_020(CardScript):
    """BT15-020 Gabumon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT15-020 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 0
        effect0._alt_digi_cost = 0
        effect0._alt_digi_name = "Tsunomon"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] 1 of your Digimon gains <Blocker> until the end of your opponent's turn. Then, if you have a Tamer with [Matt Ishida] in its name, <Draw 1>.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnStartMainPhase)
        effect1.set_effect_name("BT15-020 Your 1 Digimon gains Blocker and Draw 1")
        effect1.set_effect_description("[Start of Your Main Phase] 1 of your Digimon gains <Blocker> until the end of your opponent's turn. Then, if you have a Tamer with [Matt Ishida] in its name, <Draw 1>.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Grant 1 own Digimon Blocker, then Draw 1 if Matt Ishida tamer."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Step 1: Select 1 of your Digimon to gain Blocker until end of opponent's turn
            def digimon_filter(p):
                return p.is_digimon

            def on_grant(target_perm):
                target_perm.grant_keyword('_is_blocker')

            game.effect_select_own_permanent(
                player, on_grant, filter_fn=digimon_filter, is_optional=False)

            # Step 2: Then, if you have a Tamer with [Matt Ishida], Draw 1
            has_matt = any(
                p.is_tamer and p.contains_card_name('Matt Ishida')
                for p in player.battle_area
            )
            if has_matt:
                player.draw_cards(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking][Once Per Turn] When this Digimon attacks a player, <Draw 1>. (Draw 1 card from your deck.)
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name("BT15-020 Draw 1")
        effect2.set_effect_description("[When Attacking] [Once Per Turn] <Draw 1>. (Draw 1 card from your deck.)")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Draw1_BT15_020")
        effect2.is_on_attack = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
