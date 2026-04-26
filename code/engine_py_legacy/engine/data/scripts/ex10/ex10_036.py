from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_036(CardScript):
    """EX10-036 Magneticdramon | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Alt-digi: While you have [Close], [Proganomon]: Cost 6 ──
        # C# behavior: PermanentCondition requires base to be "Proganomon",
        # Condition() checks owner has a permanent with card name "Close".
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-036 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 6
        effect0._alt_digi_name = "Proganomon"

        def _alt_digi_close_check(base_perm) -> bool:
            """Check that the owner has a 'Close' tamer on field."""
            owner = base_perm.owner if base_perm else None
            if not owner:
                return False
            for perm in owner.battle_area:
                if perm.contains_card_name('Close'):
                    return True
            return False

        effect0._alt_digi_condition_fn = _alt_digi_close_check

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not permanent:
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # ── Keyword: Fragment (3) ───────────────────────────────────
        effect_fragment = ICardEffect()
        effect_fragment.set_effect_name("EX10-036 Fragment")
        effect_fragment.set_effect_description("Fragment")
        effect_fragment._is_fragment = True
        effect_fragment._fragment_count = 3

        def condition_fragment(context: Dict[str, Any]) -> bool:
            return True

        effect_fragment.set_can_use_condition(condition_fragment)
        effects.append(effect_fragment)

        # ── Helpers ─────────────────────────────────────────────────

        def _is_mineral_or_rock(c) -> bool:
            """Check if a card has [Mineral] or [Rock] trait."""
            traits = getattr(c, 'card_traits', []) or []
            return any('Mineral' in t or 'Rock' in t for t in traits)

        def _count_mineral_rock_in_trash(player) -> int:
            if not player:
                return 0
            return sum(1 for c in player.trash_cards if _is_mineral_or_rock(c))

        def _count_mineral_rock_in_sources(player) -> int:
            """Count Mineral/Rock digivolution cards across ALL own Digimon
            (excluding top cards which are the Digimon themselves)."""
            if not player:
                return 0
            count = 0
            for perm in player.battle_area:
                top = perm.top_card
                for c in perm.digivolution_cards:
                    if c is top:
                        continue
                    if _is_mineral_or_rock(c):
                        count += 1
            return count

        # ── [When Digivolving] [When Attacking] Delete effect ──────
        # By trashing 3 [Mineral] or [Rock] trait cards from any of your
        # Digimon's digivolution cards, delete 1 of your opponent's Digimon
        # and trash their top security card.

        def build_delete_effect(is_when_digivolving: bool):
            effect = ICardEffect()
            if is_when_digivolving:
                effect.set_timing(EffectTiming.OnEnterFieldAnyone)
                effect.is_when_digivolving = True
            else:
                effect.set_timing(EffectTiming.OnUseAttack)
                effect.is_on_attack = True
            effect.set_effect_name("EX10-036 Trash 3 sources, delete 1 opponent Digimon and trash top security")
            effect.set_effect_description(
                "[When Digivolving] [When Attacking] By trashing 3 [Mineral] or [Rock] trait "
                "cards from any of your Digimon's digivolution cards, delete 1 of your opponent's "
                "Digimon and trash their top security card."
            )
            effect.is_optional = True

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                owner = card.owner if card else None
                return _count_mineral_rock_in_sources(owner) >= 3

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return

                from ....data.enums import GamePhase
                from ....game.constants import SOURCES_PER_FIELD

                trashed_cards = []

                def _select_next_source():
                    """Request player to select next Mineral/Rock digi source to trash."""
                    if len(trashed_cards) >= 3:
                        # All 3 trashed — now move trashed cards to trash and proceed
                        player.trash_cards.extend(trashed_cards)
                        _do_delete_and_security()
                        return

                    # Build valid source indices across all own Digimon
                    valid = []
                    for fi, perm in enumerate(player.battle_area):
                        top = perm.top_card
                        base = 2000 + fi * SOURCES_PER_FIELD
                        for si, cs in enumerate(perm.card_sources):
                            if cs is top:
                                continue
                            if _is_mineral_or_rock(cs) and (base + si) < 2168:
                                valid.append(base + si)
                    if not valid:
                        # Not enough sources left — abort
                        if trashed_cards:
                            player.trash_cards.extend(trashed_cards)
                        return

                    def on_source_selected(action_id):
                        normalized = action_id - 2000
                        field_idx = normalized // SOURCES_PER_FIELD
                        source_idx = normalized % SOURCES_PER_FIELD
                        if field_idx < len(player.battle_area):
                            src_perm = player.battle_area[field_idx]
                            if source_idx < len(src_perm.card_sources):
                                selected = src_perm.card_sources[source_idx]
                                src_perm.card_sources.remove(selected)
                                trashed_cards.append(selected)
                        _select_next_source()

                    game.request_selection(
                        GamePhase.SelectSource, player, on_source_selected,
                        valid, is_optional=False,
                        prompt=f"Select a [Mineral] or [Rock] digivolution card to trash ({len(trashed_cards)+1}/3).")

                def _do_delete_and_security():
                    """After trashing 3 sources, delete 1 opponent Digimon and trash top security."""
                    def on_delete(target_perm):
                        enemy = player.enemy if player else None
                        if enemy:
                            enemy.delete_permanent(target_perm)
                        # Trash opponent's top security card
                        if enemy and enemy.security_cards:
                            trashed = enemy.security_cards.pop(0)
                            enemy.trash_cards.append(trashed)

                    game.effect_select_opponent_permanent(
                        player, on_delete,
                        filter_fn=lambda p: p.is_digimon,
                        is_optional=False,
                        prompt="Select 1 opponent Digimon to delete.",
                    )

                _select_next_source()

            effect.set_on_process_callback(process)
            return effect

        effects.append(build_delete_effect(is_when_digivolving=True))
        effects.append(build_delete_effect(is_when_digivolving=False))

        # ── [When Digivolving] [When Attacking] [Once Per Turn] Unsuspend ──
        # By placing 3 [Mineral] or [Rock] trait cards from your trash as this
        # Digimon's bottom digivolution cards, it unsuspends.

        def build_unsuspend_effect(is_when_digivolving: bool):
            effect = ICardEffect()
            if is_when_digivolving:
                effect.set_timing(EffectTiming.OnEnterFieldAnyone)
                effect.is_when_digivolving = True
            else:
                effect.set_timing(EffectTiming.OnUseAttack)
                effect.is_on_attack = True
            effect.set_effect_name("EX10-036 Place 3 Mineral/Rock from trash as bottom sources, then unsuspend")
            effect.set_effect_description(
                "[When Digivolving] [When Attacking] [Once Per Turn] By placing 3 [Mineral] or "
                "[Rock] trait cards from your trash as this Digimon's bottom digivolution cards, "
                "it unsuspends."
            )
            effect.is_optional = True
            effect.set_max_count_per_turn(1)
            effect.set_hash_string("UNSUSPEND_EX10_036")

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                owner = card.owner if card else None
                return _count_mineral_rock_in_trash(owner) >= 3

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                perm = ctx.get('permanent')
                game = ctx.get('game')
                if not (player and perm and game):
                    return

                from ....data.enums import GamePhase
                from ....game.constants import SEL_TRASH_START

                placed_count = [0]

                def _place_next():
                    if placed_count[0] >= 3:
                        # All 3 placed — unsuspend
                        perm.unsuspend()
                        return

                    valid = [SEL_TRASH_START + i
                             for i, c in enumerate(player.trash_cards)
                             if _is_mineral_or_rock(c)]
                    if not valid:
                        # Not enough cards, don't unsuspend
                        return

                    def on_trash_selected(action_id):
                        # action_id is the raw value including SEL_TRASH_START offset
                        idx = action_id - SEL_TRASH_START
                        if 0 <= idx < len(player.trash_cards):
                            selected = player.trash_cards[idx]
                            player.trash_cards.remove(selected)
                            perm.add_card_source_bottom(selected)
                            placed_count[0] += 1
                        _place_next()

                    game.request_selection(
                        GamePhase.SelectTrash, player, on_trash_selected,
                        valid, is_optional=False,
                        prompt=f"Select a [Mineral] or [Rock] card from trash ({placed_count[0]+1}/3).")

                _place_next()

            effect.set_on_process_callback(process)
            return effect

        effects.append(build_unsuspend_effect(is_when_digivolving=True))
        effects.append(build_unsuspend_effect(is_when_digivolving=False))

        return effects
