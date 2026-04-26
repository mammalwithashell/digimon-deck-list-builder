from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_065(CardScript):
    """EX11-065 Close | Tamer

    [Start of Your Main Phase] By trashing 1 [Mineral] or [Rock] trait
    card from your hand or your Digimon's digivolution cards, gain 1 memory.

    [All Turns] When your Digimon are played or digivolve, if any of them
    have the [Mineral] or [Rock] trait, by suspending this Tamer, you may
    place 1 [Mineral] or [Rock] trait card from your hand or trash as any
    of those Digimon's bottom digivolution card.

    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Shared helpers ───────────────────────────────────────────

        def _has_mineral_rock(c) -> bool:
            traits = getattr(c, 'card_traits', []) or []
            return 'Mineral' in traits or 'Rock' in traits

        def _has_mineral_rock_in_hand(player) -> bool:
            if not player:
                return False
            return any(_has_mineral_rock(c) for c in player.hand_cards)

        def _has_mineral_rock_in_sources(player) -> bool:
            """Check if any Digimon on owner's field has Mineral/Rock in digi-sources."""
            if not player:
                return False
            for perm in player.battle_area:
                if not perm.is_digimon:
                    continue
                for c in perm.digivolution_cards:
                    if c is perm.top_card:
                        continue
                    if _has_mineral_rock(c):
                        return True
            return False

        # ══════════════════════════════════════════════════════════════
        # Effect 0: [Start of Your Main Phase]
        # By trashing 1 [Mineral]/[Rock] from hand or digi-sources, +1 memory.
        # C# flow: branch choice ("From hand" / "From digivolution cards" / "Do not trash")
        #   → SelectHandEffect (hand) or SelectTrashDigivolutionCards (sources)
        #   → if trashed, gain 1 memory
        # ══════════════════════════════════════════════════════════════

        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("EX11-065 Trash 1 Mineral/Rock to gain 1 memory")
        effect0.set_effect_description(
            "[Start of Your Main Phase] By trashing 1 [Mineral] or [Rock] "
            "trait card from your hand or your Digimon's digivolution cards, "
            "gain 1 memory."
        )
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner or not owner.is_my_turn:
                return False
            return _has_mineral_rock_in_hand(owner) or _has_mineral_rock_in_sources(owner)

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            from ....game.constants import (
                SEL_TRASH_START, SEL_HAND_START, SOURCES_PER_FIELD,
                ACTION_SPACE_SIZE,
            )
            from ....data.enums import GamePhase

            can_hand = _has_mineral_rock_in_hand(player)
            can_sources = _has_mineral_rock_in_sources(player)

            # ── Sub-routines ──────────────────────────────────────��──

            def _do_trash_from_hand():
                """Select 1 Mineral/Rock from hand to trash, then gain 1 memory."""
                def on_trashed(selected):
                    if selected in player.hand_cards:
                        player.hand_cards.remove(selected)
                        player.trash_cards.append(selected)
                    player.add_memory(1)

                game.effect_select_hand_card(
                    player, _has_mineral_rock, on_trashed, is_optional=True,
                    prompt="Trash 1 [Mineral]/[Rock] card from hand to gain 1 memory.")

            def _do_trash_from_sources():
                """Select 1 Mineral/Rock from a Digimon's digi-sources to trash,
                then gain 1 memory. Uses SelectSource for player selection."""
                # Collect all valid source indices across all Digimon
                valid = []
                source_map = {}  # action_id -> (perm, card_source)
                for fi, perm in enumerate(player.battle_area):
                    if not perm.is_digimon:
                        continue
                    base = 2000 + fi * SOURCES_PER_FIELD
                    for si, cs in enumerate(perm.card_sources):
                        if cs is perm.top_card:
                            continue
                        if _has_mineral_rock(cs):
                            aid = base + si
                            if aid < ACTION_SPACE_SIZE:
                                valid.append(aid)
                                source_map[aid] = (perm, cs)

                if not valid:
                    return

                def on_source_selected(action_id: int):
                    entry = source_map.get(action_id)
                    if not entry:
                        return
                    perm, cs = entry
                    if cs in perm.card_sources:
                        perm.card_sources.remove(cs)
                        player.trash_cards.append(cs)
                        # Fire digivolution card discard timing
                        perm._fire_timing(
                            EffectTiming.OnDigivolutionCardDiscarded,
                            {"permanent": perm, "trashed_cards": [cs]})
                        player.add_memory(1)

                game.request_selection(
                    GamePhase.SelectSource, player, on_source_selected,
                    valid, is_optional=True,
                    prompt="Select 1 [Mineral]/[Rock] digivolution card to trash to gain 1 memory.")

            # ── Branch logic ─────────────────────────────────────────

            if can_hand and can_sources:
                def on_branch(branch: int):
                    if branch == 0:
                        _do_trash_from_hand()
                    elif branch == 1:
                        _do_trash_from_sources()
                    # branch 2 (or decline) = do not trash

                game.effect_choose_branch(
                    player, 2, on_branch,
                    prompt="From which area will you trash a card?",
                    branch_labels=["From hand", "From digivolution cards"])
            elif can_hand:
                _do_trash_from_hand()
            elif can_sources:
                _do_trash_from_sources()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ══════════════════════════════════════════════════════════════
        # Effect 1: [All Turns] When your Digimon are played or digivolve
        # with Mineral/Rock, suspend this Tamer to place 1 Mineral/Rock
        # from hand or trash as bottom digi-source.
        # C# uses OnEnterFieldAnyone only (covers both play and digivolve).
        # C# flow: suspend tamer → select target perm (if >1) →
        #   branch choice ("From hand" / "From trash" / "Do not add") →
        #   select card → place as bottom source
        # ══════════════════════════════════════════════════════════════

        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX11-065 Suspend to place Mineral/Rock under Digimon")
        effect1.set_effect_description(
            "[All Turns] When your Digimon are played or digivolve, if any of "
            "them have the [Mineral] or [Rock] trait, by suspending this Tamer, "
            "you may place 1 [Mineral] or [Rock] trait card from your hand or "
            "trash as any of those Digimon's bottom digivolution card."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            tamer_perm = card.permanent_of_this_card()
            if tamer_perm and tamer_perm.is_suspended:
                return False
            # Played or digivolved permanent must have Mineral/Rock
            event_perm = context.get('played_permanent') or context.get('digivolved_permanent')
            if event_perm is None:
                return False
            if not (event_perm.has_trait('Mineral') or event_perm.has_trait('Rock')):
                return False
            owner = card.owner if card else None
            if not owner or event_perm not in owner.battle_area:
                return False
            # Must have Mineral/Rock in hand or trash
            return any(
                _has_mineral_rock(c)
                for c in list(owner.hand_cards) + list(owner.trash_cards)
            )

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Suspend this tamer as cost
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return
            tamer_perm.suspend()

            event_perm = ctx.get('played_permanent') or ctx.get('digivolved_permanent')
            if not event_perm:
                return

            from ....game.constants import SEL_TRASH_START, ACTION_SPACE_SIZE
            from ....data.enums import GamePhase

            can_hand = _has_mineral_rock_in_hand(player)
            can_trash = any(_has_mineral_rock(c) for c in player.trash_cards)

            # ── Sub-routines ─────────────────────────────────────────

            def _do_place_from_hand(target_perm):
                """Select 1 Mineral/Rock from hand to place as bottom digi-source."""
                def on_hand_selected(selected):
                    if selected in player.hand_cards:
                        player.hand_cards.remove(selected)
                        target_perm.add_card_source_bottom(selected)

                game.effect_select_hand_card(
                    player, _has_mineral_rock, on_hand_selected, is_optional=True,
                    prompt="Select 1 [Mineral]/[Rock] card from hand to place under the Digimon.")

            def _do_place_from_trash(target_perm):
                """Select 1 Mineral/Rock from trash to place as bottom digi-source."""
                valid = []
                for i, c in enumerate(player.trash_cards):
                    if _has_mineral_rock(c):
                        aid = SEL_TRASH_START + i
                        if aid < ACTION_SPACE_SIZE:
                            valid.append(aid)
                if not valid:
                    return

                def on_trash_selected(action_id: int):
                    idx = action_id - SEL_TRASH_START
                    if not (0 <= idx < len(player.trash_cards)):
                        return
                    chosen = player.trash_cards[idx]
                    player.trash_cards.remove(chosen)
                    target_perm.add_card_source_bottom(chosen)

                game.request_selection(
                    GamePhase.SelectTrash, player, on_trash_selected,
                    valid, is_optional=True,
                    prompt="Select 1 [Mineral]/[Rock] card from trash to place under the Digimon.")

            def _do_place(target_perm):
                """Branch between hand and trash, then select card to place."""
                has_hand = any(_has_mineral_rock(c) for c in player.hand_cards)
                has_trash = any(_has_mineral_rock(c) for c in player.trash_cards)

                if has_hand and has_trash:
                    def on_branch(branch: int):
                        if branch == 0:
                            _do_place_from_hand(target_perm)
                        elif branch == 1:
                            _do_place_from_trash(target_perm)

                    game.effect_choose_branch(
                        player, 2, on_branch,
                        prompt="From which area will you select a card?",
                        branch_labels=["From hand", "From trash"])
                elif has_hand:
                    _do_place_from_hand(target_perm)
                elif has_trash:
                    _do_place_from_trash(target_perm)

            # Use event_perm directly (C# selects target if >1 matching permanents,
            # but OnEnterFieldAnyone context gives us the specific permanent)
            _do_place(event_perm)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # NOTE: No separate WhenDigivolving effect needed.
        # OnEnterFieldAnyone fires for both play and digivolve events in DCGO.
        # The C# reference only registers this timing once.

        # ══════════════════════════════════════════════════════════════
        # Effect 2: [Security] Play this card without paying the cost.
        # ══════════════════════════════════════════════════════════════

        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("EX11-065 Security: Play this card")
        effect2.set_effect_description("Security: Play this card without paying the cost.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
