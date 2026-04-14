from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_033(CardScript):
    """EX10-033 Pyramidimon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Keyword: Fragment (3)
        effect_fragment = ICardEffect()
        effect_fragment.set_effect_name("EX10-033 Fragment")
        effect_fragment.set_effect_description(
            "<Fragment (3)> (When this Digimon would be deleted, by trashing any 3 of its "
            "digivolution cards, it isn't deleted.)"
        )
        effect_fragment._is_fragment = True

        def condition_fragment(context: Dict[str, Any]) -> bool:
            return True
        effect_fragment.set_can_use_condition(condition_fragment)
        effects.append(effect_fragment)

        # ── Helper: Mineral/Rock trait filter ────────────────────────────
        def _mineral_rock_filter(c):
            traits = getattr(c, 'card_traits', [])
            return 'Mineral' in traits or 'Rock' in traits

        # ── Place effect ─────────────────────────────────────────────────
        # [When Digivolving] [When Attacking] [Once Per Turn] You may place up to 3 [Mineral]
        # or [Rock] trait cards from your trash as this Digimon's bottom digivolution cards.
        def build_place_effect(is_when_digivolving: bool = False, is_on_attack: bool = False):
            effect = ICardEffect()
            if is_when_digivolving:
                effect.set_timing(EffectTiming.OnEnterFieldAnyone)
            else:
                effect.set_timing(EffectTiming.OnUseAttack)
            effect.set_effect_name(
                "EX10-033 Place up to 3 Mineral/Rock cards from trash as bottom sources"
            )
            effect.set_effect_description(
                "[When Digivolving] [When Attacking] [Once Per Turn] You may place up to 3 "
                "[Mineral] or [Rock] trait cards from your trash as this Digimon's bottom "
                "digivolution cards."
            )
            effect.is_optional = True
            effect.set_max_count_per_turn(1)
            effect.set_hash_string("PlaceTrash_EX10_033")
            effect.is_when_digivolving = is_when_digivolving
            effect.is_on_attack = is_on_attack

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                owner = card.owner if card else None
                if not owner:
                    return False
                return any(_mineral_rock_filter(c) for c in owner.trash_cards)

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                perm = ctx.get('permanent')
                game = ctx.get('game')
                if not (player and perm and game):
                    return

                from ....data.enums import GamePhase
                from ....game.constants import SEL_TRASH_START

                placed_holder = [0]

                def _place_one():
                    if placed_holder[0] >= 3:
                        return
                    eligible = [c for c in player.trash_cards if _mineral_rock_filter(c)]
                    if not eligible:
                        return

                    valid_trash = []
                    for i, c in enumerate(player.trash_cards):
                        if _mineral_rock_filter(c):
                            valid_trash.append(SEL_TRASH_START + i)
                    if not valid_trash:
                        return

                    def on_trash_selected(action_id):
                        # FIX: subtract SEL_TRASH_START to get the actual trash index
                        idx = action_id - SEL_TRASH_START
                        if 0 <= idx < len(player.trash_cards):
                            selected = player.trash_cards[idx]
                            player.trash_cards.remove(selected)
                            perm.add_card_source_bottom(selected)
                            placed_holder[0] += 1
                            _place_one()

                    game.request_selection(
                        GamePhase.SelectTrash, player, on_trash_selected,
                        valid_trash, is_optional=True,
                        prompt=f"Select a [Mineral] or [Rock] card from trash ({placed_holder[0]+1}/3).")

                _place_one()

            effect.set_on_process_callback(process)
            return effect

        effects.append(build_place_effect(is_when_digivolving=True))
        effects.append(build_place_effect(is_on_attack=True))

        # ── Cost-reduce effect ───────────────────────────────────────────
        # [When Digivolving] [When Attacking] By trashing up to 3 [Mineral] or [Rock] trait
        # cards from any of your Digimon's digivolution cards, to 1 of your opponent's Digimon,
        # reduce the play cost by 2 until their turn ends for each card trashed.
        def build_cost_reduce_effect(is_when_digivolving: bool = False, is_on_attack: bool = False):
            effect = ICardEffect()
            if is_when_digivolving:
                effect.set_timing(EffectTiming.OnEnterFieldAnyone)
            else:
                effect.set_timing(EffectTiming.OnUseAttack)
            effect.set_effect_name(
                "EX10-033 By trashing up to 3 Mineral/Rock sources, reduce 1 opponent Digimon "
                "play cost by 2 per card until their turn ends"
            )
            effect.set_effect_description(
                "[When Digivolving] [When Attacking] By trashing up to 3 [Mineral] or [Rock] "
                "trait cards from any of your Digimon's digivolution cards, to 1 of your "
                "opponent's Digimon, reduce the play cost by 2 until their turn ends for each "
                "card trashed."
            )
            effect.is_optional = True
            effect.is_when_digivolving = is_when_digivolving
            effect.is_on_attack = is_on_attack

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                owner = card.owner if card else None
                if not owner:
                    return False
                # Need at least 1 Mineral/Rock source in any of our Digimon
                for p in owner.battle_area:
                    if not p.is_digimon:
                        continue
                    for src in p.card_sources:
                        if src is p.top_card:
                            continue
                        if _mineral_rock_filter(src):
                            return True
                return False

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                perm = ctx.get('permanent')
                game = ctx.get('game')
                if not (player and perm and game):
                    return

                from ....data.enums import GamePhase
                from ....game.constants import SOURCES_PER_FIELD, ACTION_SPACE_SIZE
                from ....interfaces.modifiers import ModifierType

                # Track trashed cards across iterative selections
                trashed_cards = []

                def _build_valid_source_indices():
                    """Build valid source action indices across ALL player Digimon."""
                    valid = []
                    for f_idx, ba_perm in enumerate(player.battle_area):
                        if not ba_perm.is_digimon:
                            continue
                        for s_idx, src in enumerate(ba_perm.card_sources):
                            if src is ba_perm.top_card:
                                continue
                            if not _mineral_rock_filter(src):
                                continue
                            action_id = 2000 + f_idx * SOURCES_PER_FIELD + s_idx
                            if action_id < ACTION_SPACE_SIZE:
                                valid.append(action_id)
                    return valid

                def _select_source():
                    if len(trashed_cards) >= 3:
                        _finish_with_target()
                        return

                    valid = _build_valid_source_indices()
                    if not valid:
                        if trashed_cards:
                            _finish_with_target()
                        return

                    def on_source_selected(action_id):
                        normalized = action_id - 2000
                        field_idx = normalized // SOURCES_PER_FIELD
                        source_idx = normalized % SOURCES_PER_FIELD

                        if field_idx < len(player.battle_area):
                            ba_perm = player.battle_area[field_idx]
                            if source_idx < len(ba_perm.card_sources):
                                selected = ba_perm.card_sources[source_idx]
                                # Use trash_specific_digivolution_cards for proper timing
                                removed = ba_perm.trash_specific_digivolution_cards([selected])
                                for c in removed:
                                    player.trash_cards.append(c)
                                    trashed_cards.append(c)

                        # Continue selecting or finish
                        _select_source()

                    def on_decline():
                        if trashed_cards:
                            _finish_with_target()

                    game.request_selection(
                        GamePhase.SelectSource, player, on_source_selected,
                        valid, is_optional=True,
                        prompt=f"Select a [Mineral]/[Rock] digivolution card to trash ({len(trashed_cards)+1}/3).",
                        on_decline=on_decline)

                def _finish_with_target():
                    if not trashed_cards:
                        return
                    reduction = len(trashed_cards) * 2

                    def on_target(target_perm):
                        _target_ref = target_perm
                        game.register_modifier(
                            target_perm,
                            ModifierType.CHANGE_PLAY_COST,
                            value_fn=lambda cur, card_arg, ctx, _r=reduction: cur - _r,
                            condition=lambda p, c, _tp=_target_ref: p is _tp,
                            expiry='end_of_opponent_turn'
                        )

                    game.effect_select_opponent_permanent(
                        player, on_target,
                        filter_fn=lambda p: p.is_digimon,
                        is_optional=False
                    )

                _select_source()

            effect.set_on_process_callback(process)
            return effect

        effects.append(build_cost_reduce_effect(is_when_digivolving=True))
        effects.append(build_cost_reduce_effect(is_on_attack=True))

        return effects
