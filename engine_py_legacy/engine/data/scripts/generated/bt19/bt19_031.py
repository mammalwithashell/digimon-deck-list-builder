from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_031(CardScript):
    """BT19-031 Starmons | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT19-031 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: with [Xros Heart] trait for cost 0
        effect0._alt_digi_cost = 0
        effect0._alt_digi_trait = "Xros Heart"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Xros Heart' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: decoy
        # Decoy
        effect1 = ICardEffect()
        effect1.set_effect_name("BT19-031 Decoy")
        effect1.set_effect_description("Decoy")
        effect1._is_decoy = True

        def condition1(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Xros Heart' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] You may play 1 [ShootingStarmon] from under your Tamers without paying the cost. Then, place 1 [Starmons] and 1 [Pickmons] from your trash as that Digimon's bottom digivolution cards.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT19-031 Play [ShootingStarmon] from under your Tamers")
        effect2.set_effect_description("[On Deletion] You may play 1 [ShootingStarmon] from under your Tamers without paying the cost. Then, place 1 [Starmons] and 1 [Pickmons] from your trash as that Digimon's bottom digivolution cards.")
        effect2.is_optional = True
        effect2.is_on_deletion = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] If this Digimon has the [Xros Heart] trait, 1 of your opponent's Digimon gets -2000 DP for the turn.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT19-031 DP -2000 if this Digimon has [Xros Heart] trait")
        effect3.set_effect_description("[When Attacking] If this Digimon has the [Xros Heart] trait, 1 of your opponent's Digimon gets -2000 DP for the turn.")
        effect3.is_inherited_effect = True
        effect3.set_max_count_per_turn(1)
        effect3.is_on_attack = True
        effect3.dp_modifier = -2000

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: DP -2000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-2000)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
