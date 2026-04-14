from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_012(CardScript):
    """EX11-012 Medusamon | Lv.6

    <Rush> <Progress>
    [When Digivolving] [End of Attack] You may delete 1 of your opponent's Digimon
    with as much or less DP as this Digimon. Then, by returning 1 card from your
    opponent's trash to the bottom of the deck, they play 1 [Petrification] Token.
    [All Turns] When this Digimon would leave the battle area, by deleting 1 Token,
    it doesn't leave.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Rush ─────────────────────────────────────────────────────────
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-012 Rush")
        effect0.set_effect_description("Rush")
        effect0._is_rush = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # ── Progress ─────────────────────────────────────────────────────
        effect1 = ICardEffect()
        effect1.set_effect_name("EX11-012 Progress")
        effect1.set_effect_description("Progress")
        effect1._is_progress = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # ── Shared process for [When Digivolving] and [End of Attack] ───
        def _shared_process(ctx: Dict[str, Any]):
            """Delete opponent Digimon (optional), then return 1 trash card (optional cost) -> Petrification Token."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            def _after_delete():
                """After optional delete resolves, offer trash return -> token."""
                from ....game.constants import SEL_TRASH_START as _SEL_TRASH
                from ....data.enums import GamePhase

                valid_trash = [_SEL_TRASH + i for i in range(len(enemy.trash_cards))]
                if not valid_trash:
                    return

                def on_trash_selected(action_id):
                    idx = action_id - _SEL_TRASH
                    if 0 <= idx < len(enemy.trash_cards):
                        chosen = enemy.trash_cards[idx]
                        enemy.trash_cards.remove(chosen)
                        enemy.library_cards.append(chosen)
                        # Cost paid — play 1 Petrification Token on opponent's field
                        game.effect_play_token(player, 'petrification', on_opponent_field=True, count=1)

                def on_decline_trash():
                    # Player declined — cost not paid, no token
                    pass

                game.request_selection(
                    GamePhase.SelectTarget, player, on_trash_selected,
                    valid_trash, is_optional=True,
                    prompt="Select 1 card from opponent's trash to return to deck bottom (cost for Petrification Token).",
                    on_decline=on_decline_trash)

            # Step 1: Optional delete of opponent Digimon with DP <= own DP
            def target_filter(p):
                own_perm = card.permanent_of_this_card()
                own_dp = own_perm.dp if own_perm else 0
                return p.is_digimon and (p.dp or 0) <= (own_dp or 0)

            def on_delete(target_perm):
                enemy.delete_permanent(target_perm, is_opponent_effect=True)
                _after_delete()

            def on_decline_delete():
                _after_delete()

            # Check if any valid targets exist
            opp_digimon = [p for p in enemy.battle_area
                           if p.is_digimon and target_filter(p)]
            if opp_digimon:
                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=target_filter, is_optional=True)
                # Attach on_decline to the pending selection
                if game.pending_selection:
                    game.pending_selection.on_decline = on_decline_delete
            else:
                # No valid targets — skip to trash return
                _after_delete()

        # ── [When Digivolving] ───────────────────────────────────────────
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.WhenDigivolving)
        effect2.set_effect_name("EX11-012 WD: Delete, Return Trash, Play Token")
        effect2.set_effect_description(
            "[When Digivolving] You may delete 1 of your opponent's Digimon "
            "with DP <= this Digimon. Then, return 1 opponent trash card to "
            "deck bottom -> play 1 Petrification Token.")
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_shared_process)
        effects.append(effect2)

        # ── [End of Attack] ──────────────────────────────────────────────
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEndAttack)
        effect3.set_effect_name("EX11-012 EoA: Delete, Return Trash, Play Token")
        effect3.set_effect_description(
            "[End of Attack] You may delete 1 of your opponent's Digimon "
            "with DP <= this Digimon. Then, return 1 opponent trash card to "
            "deck bottom -> play 1 Petrification Token.")

        def condition3(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            # Only fire when THIS Digimon was the attacker
            if not getattr(perm, 'is_attacking', False):
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_shared_process)
        effects.append(effect3)

        # ── [All Turns] When would leave, delete 1 Token to not leave ────
        # Uses WhenPermanentWouldBeDeleted timing + _will_not_be_removed flag
        # (DCGO pattern: willBeRemoveField = false)
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.WhenPermanentWouldBeDeleted)
        effect4.set_effect_name("EX11-012 By deleting a token, this does not leave")
        effect4.set_effect_description(
            "[All Turns] When this Digimon would leave the battle area, "
            "by deleting 1 Token, it doesn't leave.")
        effect4.is_optional = True
        effect4.set_hash_string("EX11_012_WHEN_REMOVED")

        _preventing_removal = [False]  # re-entrancy guard

        def condition4(context: Dict[str, Any]) -> bool:
            if _preventing_removal[0]:
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            owner = getattr(card, 'owner', None)
            if not owner:
                return False
            # Must be this Digimon leaving -- check event_permanent (the one being deleted)
            # context['permanent'] is the effect source (always Medusamon)
            # context['event_permanent'] is the permanent that triggered this timing
            leaving_perm = context.get('event_permanent') or context.get('permanent')
            owner_perm = card.permanent_of_this_card()
            if leaving_perm is None or leaving_perm is not owner_perm:
                return False
            # Player must have at least one token on the field (excluding self)
            if not any(getattr(p, 'is_token', False) and p is not owner_perm
                       for p in owner.battle_area):
                return False
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Delete 1 own token, prevent this Digimon from leaving.

            Note: WhenPermanentWouldBeDeleted fires synchronously inside
            delete_permanent(). The _will_not_be_removed flag must be set
            before the method returns, so we auto-select the first available
            token inline rather than creating a pending selection.
            """
            if _preventing_removal[0]:
                return
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            owner_perm = card.permanent_of_this_card()
            if not owner_perm:
                return

            # Find first token on our field
            token_to_delete = None
            for p in player.battle_area:
                if getattr(p, 'is_token', False) and p is not owner_perm:
                    token_to_delete = p
                    break

            if token_to_delete:
                _preventing_removal[0] = True
                try:
                    player.delete_permanent(token_to_delete)
                    # Prevent this Digimon from leaving (DCGO: willBeRemoveField = false)
                    owner_perm._will_not_be_removed = True
                finally:
                    _preventing_removal[0] = False

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
