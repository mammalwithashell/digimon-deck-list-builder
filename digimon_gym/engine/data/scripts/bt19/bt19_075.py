from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_075(CardScript):
    """BT19-075 MoonMillenniummon | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: from [Millenniummon] for cost 2
        effect0 = ICardEffect()
        effect0.set_effect_name("BT19-075 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 2
        effect0._alt_digi_name = "Millenniummon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Millenniummon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Shared process for On Play / When Digivolving ---
        def _discard_and_delete_tamers(ctx: Dict[str, Any]):
            """Opponent trashes hand cards until 5 remain. For every 2 trashed, delete 1 of their Tamers."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            hand_count = len(enemy.hand_cards)
            if hand_count <= 5:
                return
            discard_count = hand_count - 5
            trashed_so_far = []

            def _discard_one():
                """Recursively have opponent select cards to discard."""
                if len(trashed_so_far) >= discard_count:
                    # All discards done, now delete tamers
                    _delete_tamers()
                    return

                def on_discard(selected_card):
                    if selected_card in enemy.hand_cards:
                        enemy.hand_cards.remove(selected_card)
                        enemy.trash_cards.append(selected_card)
                        trashed_so_far.append(selected_card)
                    _discard_one()

                game.effect_select_hand_card(
                    enemy, lambda c: True, on_discard,
                    is_optional=False,
                    prompt=f"Select a card to trash ({len(trashed_so_far)+1}/{discard_count}).")

            def _delete_tamers():
                """Delete opponent's tamers: 1 per 2 cards trashed."""
                num_deletes = len(trashed_so_far) // 2
                if num_deletes <= 0:
                    return

                def tamer_filter(p):
                    return p.is_tamer

                has_tamers = any(tamer_filter(p) for p in enemy.battle_area)
                if not has_tamers:
                    return

                deleted_count = [0]

                def _delete_one():
                    if deleted_count[0] >= num_deletes:
                        return
                    remaining_tamers = any(tamer_filter(p) for p in enemy.battle_area)
                    if not remaining_tamers:
                        return

                    def on_delete(target_perm):
                        enemy.delete_permanent(target_perm)
                        deleted_count[0] += 1
                        _delete_one()

                    game.effect_select_opponent_permanent(
                        player, on_delete, filter_fn=tamer_filter, is_optional=False)

                _delete_one()

            _discard_one()

        # [On Play] opponent trashes hand to 5, delete tamers
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT19-075 Your opponent trashes cards in their hand so that 5 remain")
        effect1.set_effect_description("[On Play] Your opponent trashes cards in their hand so that 5 remain. For every 2 trashed by this effect, delete 1 of their Tamers.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_discard_and_delete_tamers)
        effects.append(effect1)

        # [When Digivolving] same effect
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT19-075 Your opponent trashes cards in their hand so that 5 remain")
        effect2.set_effect_description("[When Digivolving] Your opponent trashes cards in their hand so that 5 remain. For every 2 trashed by this effect, delete 1 of their Tamers.")
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_discard_and_delete_tamers)
        effects.append(effect2)

        # [All Turns] When this Digimon would leave the battle area, by deleting 1 of your
        # Digimon with the [Composite] trait, it doesn't leave.
        # C#: HasCompositeTrait checks IsPermanentExistsOnOwnerBattleAreaDigimon + EqualsTraits("Composite")
        # Uses WhenPermanentWouldBeDeleted timing + _will_not_be_removed flag
        # (matches ST19-11 / EX7-027 canonical pattern)
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.WhenPermanentWouldBeDeleted)
        effect3.set_effect_name("BT19-075 Delete [Composite] trait digimon to prevent removal")
        effect3.set_effect_description("[All Turns] When this Digimon would leave the battle area, by deleting 1 of your Digimon with the [Composite] trait, it doesn't leave.")
        effect3.is_optional = True

        def _has_composite(p, my_perm):
            """Check if a permanent is a valid [Composite] sacrifice target."""
            if p is my_perm or not p.is_digimon:
                return False
            top = getattr(p, 'top_card', None)
            if not top:
                return False
            traits = getattr(top, 'card_traits', []) or []
            return any('Composite' in t for t in traits)

        # Guard: prevent re-entrant triggering during on_decline re-deletion
        _preventing = [False]

        def condition3(context: Dict[str, Any]) -> bool:
            if _preventing[0]:
                return False
            my_perm = card.permanent_of_this_card() if card else None
            if not my_perm:
                return False
            # Only triggers for THIS permanent being removed
            event_perm = context.get('event_permanent') or context.get('permanent')
            if event_perm is not my_perm:
                return False
            player = card.owner if card else None
            if not player:
                return False
            # Must have at least 1 own Digimon with [Composite] trait (not self)
            return any(_has_composite(p, my_perm) for p in player.battle_area)
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Delete 1 own [Composite] Digimon to prevent this Digimon from leaving.

            Pattern: Set _will_not_be_removed = True IMMEDIATELY to prevent
            deletion (since delete_permanent checks the flag synchronously after
            firing WhenPermanentWouldBeDeleted), then start async selection.
            If the player declines, undo prevention by deleting the permanent.
            """
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            my_perm = card.permanent_of_this_card() if card else None
            if not my_perm:
                return

            # Immediately prevent deletion — selection will confirm or undo
            my_perm._will_not_be_removed = True

            # Build valid target list
            from ....game.effects import SEL_MY_FIELD_START
            valid = []
            for i, p in enumerate(player.battle_area):
                if _has_composite(p, my_perm):
                    valid.append(SEL_MY_FIELD_START + i)
            if not valid:
                # No valid targets — should not reach here (condition checks)
                my_perm._will_not_be_removed = False
                return

            def on_select(action_id: int):
                idx = action_id - SEL_MY_FIELD_START
                if 0 <= idx < len(player.battle_area):
                    target_perm = player.battle_area[idx]
                    player.delete_permanent(target_perm)

            def on_decline():
                # Player chose not to sacrifice — undo prevention, delete the permanent
                # Set guard to prevent re-entrant triggering of this effect
                _preventing[0] = True
                try:
                    player.delete_permanent(my_perm)
                finally:
                    _preventing[0] = False

            from ....data.enums import GamePhase as GP
            game.request_selection(
                GP.SelectTarget, player, on_select, valid,
                is_optional=True,
                prompt="Select 1 [Composite] Digimon to delete to prevent leaving.",
                on_decline=on_decline,
            )

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # [All Turns] [Once Per Turn] When other Digimon or Tamers are deleted,
        # trash your opponent's top security card.
        # Uses _is_deletion_observer so _fire_deletion_observers() picks it up
        # when ANY Digimon/Tamer is deleted (not just self)
        effect4 = ICardEffect()
        effect4._is_deletion_observer = True
        effect4.set_effect_name("BT19-075 Trash your opponent's top security card")
        effect4.set_effect_description("[All Turns] [Once Per Turn] When other Digimon or Tamers are deleted, trash your opponent's top security card.")
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("TrashSecurity_BT19-075")

        def condition4(context: Dict[str, Any]) -> bool:
            my_perm = card.permanent_of_this_card() if card else None
            if not my_perm:
                return False
            # C#: OtherPermanentDeleted checks (IsDigimon || IsTamer) && permanent != self
            deleted_perm = context.get('deleted_permanent')
            if deleted_perm is my_perm:
                return False
            # Must be a Digimon or Tamer
            if deleted_perm and not (deleted_perm.is_digimon or deleted_perm.is_tamer):
                return False
            # Must have enemy security to trash
            owner = card.owner if card else None
            if not owner or not owner.enemy:
                return False
            if not owner.enemy.security_cards:
                return False
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Trash opponent's top security card."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not player:
                return
            enemy = player.enemy if player else None
            if enemy and enemy.security_cards:
                trashed = enemy.security_cards.pop(0)
                enemy.trash_cards.append(trashed)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
