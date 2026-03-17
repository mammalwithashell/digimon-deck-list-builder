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
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.WhenRemoveField)
        effect3.set_effect_name("BT19-075 Delete [Composite] trait digimon to prevent removal")
        effect3.set_effect_description("[All Turns] When this Digimon would leave the battle area, by deleting 1 of your Digimon with the [Composite] trait, it doesn't leave.")
        effect3.is_optional = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # [All Turns] [Once Per Turn] When other Digimon or Tamers are deleted,
        # trash your opponent's top security card.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnDestroyedAnyone)
        effect4.set_effect_name("BT19-075 Trash your opponent's top security card")
        effect4.set_effect_description("[All Turns] [Once Per Turn] When other Digimon or Tamers are deleted, trash your opponent's top security card.")
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("TrashSecurity_BT19-075")
        effect4.is_on_deletion = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Trash opponent's top security card."""
            player = ctx.get('player')
            game = ctx.get('game')
            enemy = player.enemy if player else None
            if enemy and enemy.security_cards:
                trashed = enemy.security_cards.pop(0)
                enemy.trash_cards.append(trashed)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
