from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_101(CardScript):
    """BT22-101 Kyoko Kuremi | Tamer (Black/Yellow, Cost 5, CS)

    [Start of Your Turn] If you have 2 or less memory, set it to 3.
    [All Turns] When any of your level 4 or higher [CS] trait Digimon are
        deleted, by suspending this Tamer, return 1 [CS] trait Digimon card
        from your trash to the hand.
    [Your Turn] [Once Per Turn] When this Tamer unsuspends, if you don't have
        [Alphamon], this Tamer may digivolve into [Alphamon] in the hand with
        the digivolution cost reduced by 2.
    Security Effect: [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --------------------------------------------------------------
        # C1 — [Start of Your Turn] If memory <= 2, set to 3
        # (fires during OnStartMainPhase like other Tamer memory gates)
        # --------------------------------------------------------------
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("BT22-101 Set memory to 3")
        effect0.set_effect_description(
            "[Start of Your Turn] If you have 2 or less memory, set it to 3."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Set memory to 3 if <= 2."""
            game = ctx.get('game')
            if game and game.memory <= 2:
                game.memory = 3
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --------------------------------------------------------------
        # C2 — [All Turns] When our Lv4+ CS Digimon is deleted,
        # by suspending this Tamer, return 1 CS Digimon from trash to hand.
        # Uses NoTiming + _is_deletion_observer so _fire_deletion_observers()
        # (which scans cross-permanent effects) picks it up. OnDestroyedAnyone
        # fires only from the deleted perm's own card_sources and would never
        # pick up this observer.
        # --------------------------------------------------------------
        effect1 = ICardEffect()
        effect1.set_effect_name(
            "BT22-101 Suspend tamer, add 1 [CS] digimon from trash to hand"
        )
        effect1.set_effect_description(
            "[All Turns] When any of your level 4 or higher [CS] trait Digimon "
            "are deleted, by suspending this Tamer, return 1 [CS] trait Digimon "
            "card from your trash to the hand."
        )
        effect1.is_optional = True
        effect1._is_deletion_observer = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Kyoko must be on field
            tamer_perm = card.permanent_of_this_card() if card else None
            if tamer_perm is None:
                return False
            # Can't pay the suspend cost if already suspended
            if tamer_perm.is_suspended:
                return False
            owner = card.owner if card else None
            if owner is None:
                return False
            # The deleted permanent must be our own Digimon
            deleted_perm = context.get('deleted_permanent')
            if deleted_perm is None:
                return False
            if not getattr(deleted_perm, 'is_digimon', False):
                return False
            # Confirm ownership of the deleted perm: its card_sources[0].owner
            # is the original owner (context['player'] in deletion observers
            # is always the observer's owner, not the deleted owner).
            deleted_owner = (
                deleted_perm.card_sources[0].owner
                if deleted_perm.card_sources else None
            )
            if deleted_owner is not owner:
                return False
            # Level 4 or higher
            top = deleted_perm.top_card
            if top is None:
                return False
            level = getattr(top, 'level', None)
            if level is None or level < 4:
                return False
            # Must have [CS] trait (exact-match on trait list, not substring)
            traits = getattr(top, 'card_traits', []) or []
            if 'CS' not in traits:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Suspend Kyoko, then pick 1 CS Digimon from trash to return to hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            tamer_perm = card.permanent_of_this_card() if card else None
            if tamer_perm is None:
                return
            # Pay cost: suspend this Tamer
            if not tamer_perm.is_suspended:
                tamer_perm.suspend()

            # Select 1 CS trait Digimon card from trash to return to hand
            def trash_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return 'CS' in traits

            def on_select(selected):
                if selected in player.trash_cards:
                    player.trash_cards.remove(selected)
                    player.hand_cards.append(selected)

            # Use the standard trash-select helper so the agent chooses.
            from ....game.effects import SEL_TRASH_START
            from ....data.enums import GamePhase
            valid = []
            for i, tc in enumerate(player.trash_cards):
                if trash_filter(tc):
                    valid.append(SEL_TRASH_START + i)
            if not valid:
                return

            def on_action(action_id):
                idx = action_id - SEL_TRASH_START
                if 0 <= idx < len(player.trash_cards):
                    on_select(player.trash_cards[idx])

            game.request_selection(
                GamePhase.SelectTrash, player, on_action,
                valid, is_optional=False,
                prompt="Select 1 [CS] trait Digimon card from trash to return to hand.",
            )
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --------------------------------------------------------------
        # C3 — [Your Turn] [Once Per Turn] When this Tamer unsuspends, if you
        # don't have [Alphamon], this Tamer may digivolve into [Alphamon] in
        # the hand with the digivolution cost reduced by 2.
        # --------------------------------------------------------------
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUnTappedAnyone)
        effect2.set_effect_name(
            "BT22-101 Digivolve into [Alphamon] (cost -2)"
        )
        effect2.set_effect_description(
            "[Your Turn] [Once Per Turn] When this Tamer unsuspends, if you "
            "don't have [Alphamon], this Tamer may digivolve into [Alphamon] "
            "in the hand with the digivolution cost reduced by 2."
        )
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT22_101_Digivolve")

        def _is_alphamon_exact(cs) -> bool:
            """Exact name match 'Alphamon' — excludes 'Alphamon: Ouryuken' etc."""
            names = getattr(cs, 'card_names', []) or []
            return 'Alphamon' in names

        def condition2(context: Dict[str, Any]) -> bool:
            # Kyoko must be on field
            tamer_perm = card.permanent_of_this_card() if card else None
            if tamer_perm is None:
                return False
            owner = card.owner if card else None
            if owner is None:
                return False
            # [Your Turn] — only fires on owner's turn
            if not owner.is_my_turn:
                return False
            # Only when THIS tamer's permanent is the one unsuspending
            unsuspended_perm = context.get('event_permanent', context.get('permanent'))
            if unsuspended_perm is None:
                return False
            if unsuspended_perm is not tamer_perm:
                return False
            # "if you don't have [Alphamon]" — no Alphamon on owner's battle area
            for p in owner.battle_area:
                top = p.top_card
                if top and _is_alphamon_exact(top):
                    return False
            # Must have at least one Alphamon in hand (otherwise nothing to do)
            if not any(
                getattr(c, 'is_digimon', False) and _is_alphamon_exact(c)
                for c in owner.hand_cards
            ):
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Digivolve Kyoko into an Alphamon card in hand, cost -2."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            tamer_perm = card.permanent_of_this_card() if card else None
            if tamer_perm is None:
                return

            def digi_filter(c):
                return bool(
                    getattr(c, 'is_digimon', False) and _is_alphamon_exact(c)
                )

            game.effect_digivolve_from_hand(
                player, tamer_perm, digi_filter,
                cost_reduction=2, ignore_requirements=True,
                is_optional=True,
                prompt="Select an [Alphamon] from hand to digivolve this Tamer into.",
            )
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --------------------------------------------------------------
        # C4 — [Security] Play this card without paying the cost
        # --------------------------------------------------------------
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("BT22-101 Security: Play this card")
        effect3.set_effect_description(
            "[Security] Play this card without paying the cost."
        )
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Play this tamer from security without paying cost."""
            player = ctx.get('player')
            if player and card:
                player.play_card_from_source(card, pay_cost=False)
        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
