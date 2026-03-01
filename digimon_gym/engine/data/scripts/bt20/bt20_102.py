from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_102(CardScript):
    """BT20-102 Omnimon (X Antibody) | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-102 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Omnimon] for cost 2
        effect0._alt_digi_cost = 2
        effect0._alt_digi_name = "Omnimon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Omnimon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: raid
        # Raid
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-102 Raid")
        effect1.set_effect_description("Raid")
        effect1.is_on_attack = True
        effect1._is_raid = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: piercing
        # Piercing
        effect_p = ICardEffect()
        effect_p.set_effect_name("BT20-102 Piercing")
        effect_p.set_effect_description("Piercing")
        effect_p._is_piercing = True
        def condition_p(context: Dict[str, Any]) -> bool:
            return True
        effect_p.set_can_use_condition(condition_p)
        effects.append(effect_p)

        # Factory effect: blocker
        # Blocker
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-102 Blocker")
        effect2.set_effect_description("Blocker")
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] If [Omnimon] or [X Antibody] is in this Digimon's digivolution cards, choose 1 of both players' Digimon and delete all other Digimon. Then, return 1 of your opponent's Digimon to the bottom of the deck.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT20-102 Choos 1 of both players' Digimon, delete the rest, then bottom deck 1 opponents Digimon")
        effect3.set_effect_description("[On Play] If [Omnimon] or [X Antibody] is in this Digimon's digivolution cards, choose 1 of both players' Digimon and delete all other Digimon. Then, return 1 of your opponent's Digimon to the bottom of the deck.")
        effect3.is_on_play = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if permanent:
                if not any(src.contains_card_name('Omnimon') for src in permanent.digivolution_cards):
                    return False
            else:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def _board_wipe_and_bottom_deck(player, perm, game):
            """Shared: choose 1 own Digimon + 1 opp Digimon to keep, delete rest, then bottom deck 1 opp Digimon."""
            enemy = player.enemy if player else None
            if not enemy:
                return
            # Step 1: Choose 1 own Digimon to keep (self by default)
            self_perm = card.permanent_of_this_card() if card else None
            # Delete all own Digimon except self
            own_to_delete = [p for p in list(player.battle_area)
                             if p.is_digimon and p is not self_perm]
            for p in own_to_delete:
                player.delete_permanent(p)
            # Step 2: Choose 1 opponent Digimon to keep, delete rest
            def opp_keep_filter(p):
                return p.is_digimon
            def on_keep(kept_perm):
                opp_to_delete = [p for p in list(enemy.battle_area)
                                 if p.is_digimon and p is not kept_perm]
                for p in opp_to_delete:
                    enemy.delete_permanent(p)
                # Step 3: Return 1 opp Digimon to deck bottom
                def bottom_filter(p):
                    return p.is_digimon
                def on_bottom(target_perm):
                    if target_perm in enemy.battle_area:
                        enemy.battle_area.remove(target_perm)
                        if game and hasattr(game, 'cleanup_modifiers_for_permanent'):
                            game.cleanup_modifiers_for_permanent(target_perm)
                        for cs in target_perm.card_sources:
                            enemy.library_cards.append(cs)
                game.effect_select_opponent_permanent(
                    player, on_bottom, filter_fn=bottom_filter, is_optional=True)
            game.effect_select_opponent_permanent(
                player, on_keep, filter_fn=opp_keep_filter, is_optional=False)

        def process3(ctx: Dict[str, Any]):
            """Action: Board wipe (keep 1 each), then bottom deck 1 opp Digimon (On Play)"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            _board_wipe_and_bottom_deck(player, perm, game)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If [Omnimon] or [X Antibody] is in this Digimon's digivolution cards, choose 1 of both players' Digimon and delete all other Digimon. Then, return 1 of your opponent's Digimon to the bottom of the deck.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT20-102 Choos 1 of both players' Digimon, delete the rest, then bottom deck 1 opponents Digimon")
        effect4.set_effect_description("[When Digivolving] If [Omnimon] or [X Antibody] is in this Digimon's digivolution cards, choose 1 of both players' Digimon and delete all other Digimon. Then, return 1 of your opponent's Digimon to the bottom of the deck.")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if permanent:
                if not any(src.contains_card_name('Omnimon') for src in permanent.digivolution_cards):
                    return False
            else:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Board wipe (keep 1 each), then bottom deck 1 opp Digimon (WhenDigivolving)"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            _board_wipe_and_bottom_deck(player, perm, game)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] [Once Per Turn] 1 of your Digimon may gain <Rush> for the turn and attack without suspending.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnEndTurn)
        effect5.set_effect_name("BT20-102 Gain Rush and attack without suspending")
        effect5.set_effect_description("[End of Your Turn] [Once Per Turn] 1 of your Digimon may gain <Rush> for the turn and attack without suspending.")
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("Rush_BT20_102")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: 1 of your Digimon gains Rush + attack without suspending"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def digi_filter(p):
                return p.is_digimon
            def on_grant(target_perm):
                target_perm.grant_keyword('_is_rush')
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CAN_ATTACK_UNSUSPENDED, target_perm,
                    value_fn=lambda: True, expiry='end_of_turn')
            game.effect_select_own_permanent(
                player, on_grant, filter_fn=digi_filter, is_optional=True)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
