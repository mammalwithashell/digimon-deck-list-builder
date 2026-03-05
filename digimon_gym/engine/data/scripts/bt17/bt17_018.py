from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT17_018(CardScript):
    """BT17-018 Gallantmon: Crimson Mode | Lv.7

    [Hand] [Counter] <Blast Digivolve>
    [On Play] [When Digivolving] Choose any number of your opponent's Digimon
        whose total DP adds up to 15000 or less and delete them.
    [When Attacking] [Once Per Turn] For every 10 cards in both players' trash,
        trash 1 card from the top of your opponent's security stack.
    Inherited: Ace Overflow <-5>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Blast Digivolve ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT17-018 Blast Digivolve")
        effect0.set_effect_description("[Hand] [Counter] <Blast Digivolve>")
        effect0.is_counter_effect = True
        effect0._is_blast_digivolve = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Helper: shared deletion logic for On Play / When Digivolving ---
        def _delete_up_to_15000(ctx: Dict[str, Any]):
            """Delete opponent Digimon with total DP <= 15000."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            remaining_dp = 15000
            to_delete = []

            def select_next():
                nonlocal remaining_dp
                candidates = [
                    p for p in enemy.battle_area
                    if p.is_digimon and p not in to_delete
                    and (p.dp or 0) <= remaining_dp
                ]
                if not candidates:
                    # No more valid targets — execute deletions
                    for target in to_delete:
                        if target in enemy.battle_area:
                            enemy.delete_permanent(target)
                    return

                def target_filter(p):
                    return (p.is_digimon and p not in to_delete
                            and (p.dp or 0) <= remaining_dp)

                def on_select(target_perm):
                    nonlocal remaining_dp
                    to_delete.append(target_perm)
                    remaining_dp -= (target_perm.dp or 0)
                    select_next()

                def on_done():
                    # Player chose to stop selecting — execute deletions
                    for target in to_delete:
                        if target in enemy.battle_area:
                            enemy.delete_permanent(target)

                game.effect_select_opponent_permanent(
                    player, on_select, filter_fn=target_filter,
                    is_optional=True)
                # Set on_decline on the pending selection for when player passes
                if game.pending_selection:
                    game.pending_selection.on_decline = on_done

            select_next()

        # --- Effect 1: [On Play] Delete opponent Digimon totaling <= 15000 DP ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT17-018 Delete opponent Digimon totaling 15000 DP")
        effect1.set_effect_description(
            "[On Play] Choose any number of your opponent's Digimon "
            "whose total DP adds up to 15000 or less and delete them."
        )
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_delete_up_to_15000)
        effects.append(effect1)

        # --- Effect 2: [When Digivolving] Same deletion effect ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT17-018 Delete opponent Digimon totaling 15000 DP (digivolve)")
        effect2.set_effect_description(
            "[When Digivolving] Choose any number of your opponent's Digimon "
            "whose total DP adds up to 15000 or less and delete them."
        )
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_delete_up_to_15000)
        effects.append(effect2)

        # --- Effect 3: [When Attacking] [Once Per Turn] Trash security ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnTappedAnyone)
        effect3.set_effect_name("BT17-018 Trash security per 10 cards in trash")
        effect3.set_effect_description(
            "[When Attacking] [Once Per Turn] For every 10 cards in both "
            "players' trash, trash 1 card from the top of your opponent's "
            "security stack."
        )
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("WhenAttacking_BT17-018_TrashSecurity")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card() if card else None
            ctx_perm = context.get('attacking_permanent') or context.get('permanent')
            if perm and ctx_perm and perm != ctx_perm:
                return False
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Trash opponent security: 1 per 10 cards in both trash piles."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            total_trash = len(player.trash_cards) + len(enemy.trash_cards)
            security_to_trash = total_trash // 10
            for _ in range(security_to_trash):
                if enemy.security_cards:
                    trashed = enemy.security_cards.pop(0)
                    enemy.trash_cards.append(trashed)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # --- Inherited: Ace Overflow <-5> ---
        # Engine handles ACE Overflow via _apply_ace_overflow in player.py
        # when the card leaves the field. The is_ace and ace_overflow_cost
        # attributes on the card entity base drive this behavior automatically.
        effect_ace = ICardEffect()
        effect_ace.set_effect_name("BT17-018 Ace Overflow")
        effect_ace.set_effect_description("Ace Overflow <-5>")
        effect_ace.is_inherited_effect = True
        pass  # descriptive-tagged: ace_overflow

        def condition_ace(context: Dict[str, Any]) -> bool:
            return True
        effect_ace.set_can_use_condition(condition_ace)
        effects.append(effect_ace)

        return effects
