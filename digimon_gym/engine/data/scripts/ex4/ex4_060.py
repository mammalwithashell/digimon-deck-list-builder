from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX4_060(CardScript):
    """EX4-060 Omnimon Alter-S | Lv.7 Yellow | 15000 DP | Cost 15

    DNA: Blue Lv.6 + Red Lv.6, cost 0 (handled by card data, not script)

    [When Digivolving] Delete 1 of your opponent's Digimon with 8000 DP or
        less, and return 1 of your opponent's level 6 or higher Digimon to the
        bottom of its owner's deck.

    [All Turns] When this Digimon would leave the battle area other than by
        one of your effects, play 1 [BlitzGreymon] and 1 [CresGarurumon] from
        this Digimon's digivolution cards without paying the costs. Then, place
        this Digimon at the bottom of your security stack face down.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Effect 0: [When Digivolving] Delete + Return ────────────────
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name(
            "EX4-060 Delete 1 opp Digimon DP<=8000, return 1 opp Lv6+ to deck bottom"
        )
        effect0.set_effect_description(
            "[When Digivolving] Delete 1 of your opponent's Digimon with 8000 DP "
            "or less, and return 1 of your opponent's level 6 or higher Digimon "
            "to the bottom of its owner's deck."
        )
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            enemy = player.enemy
            if not enemy:
                return False
            # Must have at least one valid target for either part
            has_delete_target = any(
                p.is_digimon and p.dp is not None and p.dp <= 8000
                for p in enemy.battle_area
            )
            has_return_target = any(
                p.is_digimon and (p.level or 0) >= 6
                for p in enemy.battle_area
            )
            return has_delete_target or has_return_target

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            def _do_return_step():
                """Step 2: Return 1 opponent Lv.6+ Digimon to bottom of deck."""
                def lv_filter(p):
                    return p.is_digimon and (p.level or 0) >= 6

                if any(lv_filter(p) for p in enemy.battle_area):
                    def on_return(target):
                        if target:
                            enemy.return_permanent_to_deck_bottom(target)

                    game.effect_select_opponent_permanent(
                        player, on_return,
                        filter_fn=lv_filter,
                        is_optional=False,
                        prompt="Return 1 of your opponent's level 6 or higher "
                               "Digimon to the bottom of its owner's deck.",
                    )

            # Step 1: Delete 1 opponent Digimon with DP <= 8000
            def dp_filter(p):
                return p.is_digimon and p.dp is not None and p.dp <= 8000

            if any(dp_filter(p) for p in enemy.battle_area):
                def on_delete(target):
                    if target:
                        enemy.delete_permanent(target, is_opponent_effect=True)
                    # Chain to step 2 after delete
                    _do_return_step()

                game.effect_select_opponent_permanent(
                    player, on_delete,
                    filter_fn=dp_filter,
                    is_optional=False,
                    prompt="Delete 1 of your opponent's Digimon with 8000 DP or less.",
                )
            else:
                # No delete targets, go straight to return step
                _do_return_step()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ── Effect 1: [All Turns] WhenRemoveField — play from evo + security ──
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.WhenRemoveField)
        effect1.set_effect_name(
            "EX4-060 Play BlitzGreymon and CresGarurumon, place self in security"
        )
        effect1.set_effect_description(
            "[All Turns] When this Digimon would leave the battle area other "
            "than by one of your effects, play 1 [BlitzGreymon] and 1 "
            "[CresGarurumon] from this Digimon's digivolution cards without "
            "paying the costs. Then, place this Digimon at the bottom of your "
            "security stack face down."
        )
        effect1.is_optional = False
        effect1.set_hash_string("PlayAndPutSecurity_EX4_060")

        def condition1(context: Dict[str, Any]) -> bool:
            # Guard: effect_list(timing) returns ALL effects regardless of timing.
            # _execute_deletion_effects_inner may call this condition for
            # OnDestroyedAnyone timing. Reject if we're in that path by
            # checking for the 'deleted_permanent' key (set only by that path).
            if 'deleted_permanent' in context:
                return False
            # After deletion, card.permanent_of_this_card() returns None because
            # the permanent is removed from battle_area. Instead, we check that
            # the card is in trash (just deleted) and NOT by own effect.
            if context.get('is_own_effect', False):
                return False
            # Verify card is in owner's trash (meaning it was just removed)
            owner = card.owner if card else None
            if not owner or card not in owner.trash_cards:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # After deletion, the permanent object still exists in memory
            # with its card_sources list intact. The card_sources are also
            # in player.trash_cards. We find BlitzGreymon and CresGarurumon
            # from the trash (they were evo cards of this Digimon).

            # Step 1: Find and play 1 [BlitzGreymon] from trash
            blitz_candidates = [
                cs for cs in player.trash_cards
                if 'BlitzGreymon' in getattr(cs, 'card_names', [])
            ]
            if blitz_candidates:
                chosen = blitz_candidates[0]
                played = player.play_card_from_source(chosen, pay_cost=False)
                if played and hasattr(game, 'execute_effects'):
                    game.execute_effects(
                        EffectTiming.OnEnterFieldAnyone,
                        {'played_card': chosen, 'played_permanent': played,
                         'event_player': player},
                    )

            # Step 2: Find and play 1 [CresGarurumon] from trash
            cres_candidates = [
                cs for cs in player.trash_cards
                if 'CresGarurumon' in getattr(cs, 'card_names', [])
            ]
            if cres_candidates:
                chosen = cres_candidates[0]
                played = player.play_card_from_source(chosen, pay_cost=False)
                if played and hasattr(game, 'execute_effects'):
                    game.execute_effects(
                        EffectTiming.OnEnterFieldAnyone,
                        {'played_card': chosen, 'played_permanent': played,
                         'event_player': player},
                    )

            # Step 3: Place this Digimon (top card = card) at bottom of
            # security stack face down
            if card and card in player.trash_cards:
                player.trash_cards.remove(card)
                # Bottom of security: insert(0) per add_to_security_from_hand(to_top=False)
                player.security_cards.insert(0, card)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
