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

        def _check_omnimon_or_xantibody_in_sources(permanent):
            """Check if [Omnimon] or [X Antibody] is in this Digimon's digivolution cards.

            C# uses EqualsCardName("Omnimon") || EqualsCardName("X Antibody")
            which checks CARD NAME, not trait.
            """
            if not permanent:
                return False
            # digivolution cards = all card_sources under the top card (bottom-to-top, top is [-1])
            for src in permanent.card_sources[:-1]:
                if src.contains_card_name('Omnimon'):
                    return True
                if src.contains_card_name('X Antibody'):
                    return True
            return False

        def _board_wipe_and_bottom_deck(player, perm, game):
            """Shared: choose 1 own Digimon + 1 opp Digimon to keep, delete rest, then bottom deck 1 opp Digimon."""
            enemy = player.enemy if player else None
            if not enemy:
                return

            def own_digi_filter(p):
                return p.is_digimon

            def on_own_keep(kept_own):
                """After choosing own Digimon to keep, choose opponent's Digimon to keep."""
                def opp_digi_filter(p):
                    return p.is_digimon

                def on_opp_keep(kept_opp):
                    """After choosing opponent's Digimon to keep, delete all others, then bottom deck 1 opp."""
                    # Delete all own Digimon except the one kept
                    own_to_delete = [p for p in list(player.battle_area)
                                     if p.is_digimon and p is not kept_own]
                    for p in own_to_delete:
                        player.delete_permanent(p)
                    # Delete all opponent Digimon except the one kept
                    opp_to_delete = [p for p in list(enemy.battle_area)
                                     if p.is_digimon and p is not kept_opp]
                    for p in opp_to_delete:
                        enemy.delete_permanent(p)
                    # Step 3: Return 1 opponent's Digimon to the bottom of the deck
                    def bottom_filter(p):
                        return p.is_digimon
                    def on_bottom(target_perm):
                        player.enemy.return_permanent_to_deck_bottom(target_perm)
                    game.effect_select_opponent_permanent(
                        player, on_bottom, filter_fn=bottom_filter, is_optional=False)

                # If opponent has Digimon, let player choose which one survives
                opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
                if opp_digimon:
                    game.effect_select_opponent_permanent(
                        player, on_opp_keep, filter_fn=opp_digi_filter, is_optional=False)
                else:
                    # No opponent Digimon — just delete own except kept, then bottom deck
                    own_to_delete = [p for p in list(player.battle_area)
                                     if p.is_digimon and p is not kept_own]
                    for p in own_to_delete:
                        player.delete_permanent(p)
                    # Still try to bottom deck if opponent somehow has Digimon after deletions
                    def bottom_filter(p):
                        return p.is_digimon
                    def on_bottom(target_perm):
                        player.enemy.return_permanent_to_deck_bottom(target_perm)
                    game.effect_select_opponent_permanent(
                        player, on_bottom, filter_fn=bottom_filter, is_optional=False)

            # If player has Digimon, let them choose which one survives
            own_digimon = [p for p in player.battle_area if p.is_digimon]
            if own_digimon:
                game.effect_select_own_permanent(
                    player, on_own_keep, filter_fn=own_digi_filter, is_optional=False)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] If [Omnimon] or [X Antibody] is in this Digimon's digivolution cards, choose 1 of both players' Digimon and delete all other Digimon. Then, return 1 of your opponent's Digimon to the bottom of the deck.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT20-102 Choose 1 of both players' Digimon, delete the rest, then bottom deck 1 opponent's Digimon")
        effect3.set_effect_description("[On Play] If [Omnimon] or [X Antibody] is in this Digimon's digivolution cards, choose 1 of both players' Digimon and delete all other Digimon. Then, return 1 of your opponent's Digimon to the bottom of the deck.")
        effect3.is_on_play = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not permanent:
                return False
            if not _check_omnimon_or_xantibody_in_sources(permanent):
                return False
            return True

        effect3.set_can_use_condition(condition3)

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
        effect4.set_effect_name("BT20-102 Choose 1 of both players' Digimon, delete the rest, then bottom deck 1 opponent's Digimon")
        effect4.set_effect_description("[When Digivolving] If [Omnimon] or [X Antibody] is in this Digimon's digivolution cards, choose 1 of both players' Digimon and delete all other Digimon. Then, return 1 of your opponent's Digimon to the bottom of the deck.")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not permanent:
                return False
            if not _check_omnimon_or_xantibody_in_sources(permanent):
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
            from ....interfaces.modifiers import ModifierType
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def digi_filter(p):
                return p.is_digimon
            def on_grant(target_perm):
                target_perm.grant_keyword('_is_rush')
                game.register_modifier(
                    target_perm, ModifierType.CAN_ATTACK_UNSUSPENDED,
                    value_fn=lambda: True, expiry='end_of_turn')
            game.effect_select_own_permanent(
                player, on_grant, filter_fn=digi_filter, is_optional=True)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
