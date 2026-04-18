from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource
    from ....core.permanent import Permanent


class BT18_082(CardScript):
    """BT18-082 Lucemon: Chaos Mode | Lv.5 Purple/Yellow Digimon

    [On Play] [When Digivolving] Your opponent may delete 1 of their Digimon
        or Tamers. If this effect didn't delete, <Recovery +1 (Deck)> (Place
        the top card of your deck on top of your security stack.) and trash
        the top card of your opponent's security stack.
    [All Turns] [Once Per Turn] When this Digimon would leave the battle area,
        by trashing the bottom card of your security stack, it doesn't leave.

    Alt-digi: <Digivolve> [Lucemon]: Cost 6 (exact-name match, from Lv.3
    plain "Lucemon" permanent).

    Reference: DCGO/Assets/Scripts/CardEffect/BT18/Purple/BT18_082.cs
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ─── Alt-digivolution: from exact "Lucemon" for cost 6 ──────────────
        # C# uses EqualsCardName("Lucemon") — strict equality. Use
        # _alt_digi_condition_fn to enforce exact match (not substring).
        def _exact_lucemon(base_perm: 'Permanent') -> bool:
            top = base_perm.top_card
            if top is None:
                return False
            names = getattr(top, 'card_names', []) or []
            # Exact match: must have a card name equal to "Lucemon"
            return any((n or '').strip() == 'Lucemon' for n in names)

        alt_digi = ICardEffect()
        alt_digi.set_effect_name("BT18-082 Alt-digi: Lucemon cost 6")
        alt_digi.set_effect_description("<Digivolve> [Lucemon]: Cost 6")
        alt_digi._alt_digi_cost = 6
        alt_digi._alt_digi_name = "Lucemon"
        alt_digi._alt_digi_condition_fn = _exact_lucemon

        def condition_alt(context: Dict[str, Any]) -> bool:
            return True
        alt_digi.set_can_use_condition(condition_alt)
        effects.append(alt_digi)

        # ─── Shared logic: opponent chooses to delete own Digimon/Tamer, ────
        # else Recovery +1 and trash opp security top.
        def _run_opp_delete_or_recovery(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if enemy is None:
                return

            def _recovery_and_trash():
                """Recovery +1 + trash top of opponent's security."""
                player.recovery(1)
                if enemy.security_cards:
                    top_sec = enemy.security_cards[0]
                    enemy.trash_security_card(top_sec)

            # Filter: opponent's own Digimon or Tamer
            def deletable_filter(p):
                return getattr(p, 'is_digimon', False) or getattr(p, 'is_tamer', False)

            has_target = any(deletable_filter(p) for p in enemy.battle_area)
            if not has_target:
                # No valid target at all → effect didn't delete → recovery + trash
                _recovery_and_trash()
                return

            # Give opponent the selection (is_optional=True → "may delete")
            def on_opp_selected(target_perm):
                # Opponent chose a target. Attempt delete. If another effect
                # prevents it, delete_permanent returns False → recovery + trash.
                result = enemy.delete_permanent(target_perm)
                if result is False:
                    _recovery_and_trash()

            def on_opp_declined():
                _recovery_and_trash()

            # Build selection manually so we can wire on_decline.
            from ....game.constants import SEL_MY_FIELD_START

            valid = []
            for i, p in enumerate(enemy.battle_area):
                if deletable_filter(p):
                    valid.append(SEL_MY_FIELD_START + i)

            if not valid:
                _recovery_and_trash()
                return

            def on_action(action_id: int):
                idx = action_id - SEL_MY_FIELD_START
                if 0 <= idx < len(enemy.battle_area):
                    on_opp_selected(enemy.battle_area[idx])

            from ....data.enums import GamePhase
            game.request_selection(
                GamePhase.SelectTarget,
                enemy,  # opponent is the selecting player
                on_action,
                valid,
                is_optional=True,
                prompt="Select 1 of your Digimon or Tamers to delete "
                       "(or decline to let opponent gain Recovery +1 and "
                       "trash your top security).",
                on_decline=on_opp_declined,
            )

        # ─── Effect 1: [On Play] ────────────────────────────────────────────
        effect_on_play = ICardEffect()
        effect_on_play.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_on_play.set_effect_name(
            "BT18-082 [On Play] Opp may delete or recovery+trash"
        )
        effect_on_play.set_effect_description(
            "[On Play] Your opponent may delete 1 of their Digimon or Tamers. "
            "If this effect didn't delete, <Recovery +1 (Deck)> and trash "
            "the top card of your opponent's security stack."
        )
        effect_on_play.is_on_play = True

        def condition_on_play(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_on_play.set_can_use_condition(condition_on_play)
        effect_on_play.set_on_process_callback(_run_opp_delete_or_recovery)
        effects.append(effect_on_play)

        # ─── Effect 2: [When Digivolving] ───────────────────────────────────
        effect_when_digi = ICardEffect()
        effect_when_digi.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_when_digi.set_effect_name(
            "BT18-082 [When Digivolving] Opp may delete or recovery+trash"
        )
        effect_when_digi.set_effect_description(
            "[When Digivolving] Your opponent may delete 1 of their Digimon "
            "or Tamers. If this effect didn't delete, <Recovery +1 (Deck)> "
            "and trash the top card of your opponent's security stack."
        )
        effect_when_digi.is_when_digivolving = True

        def condition_when_digi(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_when_digi.set_can_use_condition(condition_when_digi)
        effect_when_digi.set_on_process_callback(_run_opp_delete_or_recovery)
        effects.append(effect_when_digi)

        # ─── Effect 3: [All Turns] [Once Per Turn] Leave prevention ─────────
        # When this Digimon would leave the battle area, by trashing the
        # BOTTOM card of your security stack (index -1), it doesn't leave.
        # Pattern: WhenPermanentWouldBeDeleted + _will_not_be_removed flag.
        effect_leave = ICardEffect()
        effect_leave.set_timing(EffectTiming.WhenPermanentWouldBeDeleted)
        effect_leave.set_effect_name("BT18-082 [All Turns] Trash bottom sec to stay")
        effect_leave.set_effect_description(
            "[All Turns] [Once Per Turn] When this Digimon would leave the "
            "battle area, by trashing the bottom card of your security stack, "
            "it doesn't leave."
        )
        effect_leave.is_optional = True
        effect_leave.set_max_count_per_turn(1)
        effect_leave.set_hash_string("AllTurns_BT18-082_LeaveProtection")

        def condition_leave(context: Dict[str, Any]) -> bool:
            # Card must be on field
            if card is None:
                return False
            own_perm = card.permanent_of_this_card()
            if own_perm is None:
                return False
            # Must be THIS Digimon that would be deleted
            event_perm = context.get('event_permanent', context.get('permanent'))
            if event_perm is not own_perm:
                return False
            # Must have at least 1 security card to pay cost
            owner = card.owner
            if not owner or not owner.security_cards:
                return False
            return True
        effect_leave.set_can_use_condition(condition_leave)

        def process_leave(ctx: Dict[str, Any]):
            """Trash bottom of security (index -1) → prevent deletion.

            IMPORTANT: the engine checks `_will_not_be_removed` synchronously
            after WhenPermanentWouldBeDeleted fires. We must set the flag
            BEFORE the selection opens; if the player declines, we revert it
            via on_decline and additionally undo the cost.
            """
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            own_perm = card.permanent_of_this_card() if card else None
            if own_perm is None:
                return
            if not player.security_cards:
                return

            # Snapshot the bottom security card (index -1 = bottom).
            bottom_sec = player.security_cards[-1]

            # Optimistically set the prevention flag BEFORE opening the
            # branch selection. This mirrors the pattern in BT22-036.
            own_perm._will_not_be_removed = True

            def on_choice(branch: int):
                if branch == 0:
                    # Pay the cost: trash the bottom of security.
                    # Flag is already True → deletion already prevented.
                    if bottom_sec in player.security_cards:
                        player.trash_security_card(bottom_sec)
                else:
                    # Decline: revert the optimistic flag.
                    own_perm._will_not_be_removed = False

            def on_decline_action():
                # Defensive: if the agent hits action 62, revert.
                own_perm._will_not_be_removed = False

            game.effect_choose_branch(
                player, 2,
                callback=on_choice,
                prompt=(
                    "Trash the bottom card of your security stack to prevent "
                    "Lucemon: Chaos Mode from leaving the battle area?"
                ),
                branch_labels=[
                    "Trash bottom security (it doesn't leave)",
                    "Let it leave the battle area",
                ],
            )
            # Attach the on_decline callback to the pending selection
            # so action 62 also reverts the flag.
            if game.pending_selection:
                game.pending_selection.on_decline = on_decline_action

        effect_leave.set_on_process_callback(process_leave)
        effects.append(effect_leave)

        return effects
