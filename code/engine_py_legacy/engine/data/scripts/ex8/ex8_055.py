from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_055(CardScript):
    """EX8-055 Pyramidimon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Keyword: Fragment (3)
        effect_fragment = ICardEffect()
        effect_fragment.set_effect_name("EX8-055 Fragment")
        effect_fragment.set_effect_description(
            "<Fragment (3)> (When this Digimon would be deleted, by trashing any 3 of its "
            "digivolution cards, it isn't deleted.)"
        )
        effect_fragment._is_fragment = True

        def condition_fragment(context: Dict[str, Any]) -> bool:
            return True
        effect_fragment.set_can_use_condition(condition_fragment)
        effects.append(effect_fragment)

        # [When Digivolving] [When Attacking] By trashing any 3 digivolution cards with the
        # [Mineral] or [Rock] trait from your Digimon, this Digimon unsuspends and gains
        # <Security A. +1> for the turn.
        def build_unsuspend_effect(is_when_digivolving: bool = False, is_on_attack: bool = False):
            effect = ICardEffect()
            if is_when_digivolving:
                effect.set_timing(EffectTiming.OnEnterFieldAnyone)
            else:
                effect.set_timing(EffectTiming.OnUseAttack)
            effect.set_effect_name("EX8-055 By trashing 3 Mineral/Rock sources, unsuspend and gain Security A. +1")
            effect.set_effect_description(
                "[When Digivolving] By trashing any 3 digivolution cards with the [Mineral] or "
                "[Rock] trait from your Digimon, this Digimon unsuspends and gains "
                "<Security A. +1> for the turn."
                if is_when_digivolving else
                "[When Attacking] By trashing any 3 digivolution cards with the [Mineral] or "
                "[Rock] trait from your Digimon, this Digimon unsuspends and gains "
                "<Security A. +1> for the turn."
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
                # Count available Mineral/Rock trait source cards across all your Digimon
                count = 0
                for p in owner.battle_area:
                    if not p.is_digimon:
                        continue
                    for src in p.card_sources:
                        if src is p.top_card:
                            continue
                        traits = getattr(src, 'card_traits', [])
                        if 'Mineral' in traits or 'Rock' in traits:
                            count += 1
                return count >= 3

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                perm = ctx.get('permanent')
                game = ctx.get('game')
                if not (player and perm and game):
                    return
                from ....interfaces.modifiers import ModifierType
                from ....data.enums import GamePhase
                from ....game.constants import SOURCES_PER_FIELD

                # Player selects 3 Mineral/Rock digivolution cards from any of
                # their Digimon.  Uses SelectSource phase with recursive callback.
                trashed_holder = [0]  # mutable counter

                def _has_trait(src):
                    traits = getattr(src, 'card_traits', [])
                    return 'Mineral' in traits or 'Rock' in traits

                def _select_next():
                    if trashed_holder[0] >= 3:
                        # All 3 trashed — apply the reward
                        perm.unsuspend()
                        game.register_modifier(
                            perm,
                            ModifierType.CHANGE_SECURITY_ATTACK,
                            value_fn=lambda cur, t, c: cur + 1,
                            expiry='end_of_turn',
                        )
                        return

                    # Build valid SelectSource action IDs across all player Digimon
                    valid = []
                    for fi, fp in enumerate(player.battle_area):
                        if not fp.is_digimon:
                            continue
                        base = 2000 + fi * SOURCES_PER_FIELD
                        for si, src in enumerate(fp.card_sources):
                            if src is fp.top_card:
                                continue
                            if _has_trait(src) and (base + si) < 2168:
                                valid.append(base + si)
                    if not valid:
                        return  # shouldn't happen if condition passed

                    def on_source_selected(action_id):
                        normalized = action_id - 2000
                        field_idx = normalized // SOURCES_PER_FIELD
                        source_idx = normalized % SOURCES_PER_FIELD
                        if field_idx >= len(player.battle_area):
                            return
                        target_perm = player.battle_area[field_idx]
                        if source_idx >= len(target_perm.card_sources):
                            return
                        selected = target_perm.card_sources[source_idx]
                        target_perm.card_sources.remove(selected)
                        player.trash_cards.append(selected)
                        game.execute_effects(EffectTiming.OnDigivolutionCardDiscarded,
                                             {'trashed_cards': [selected],
                                              'permanent': target_perm})
                        trashed_holder[0] += 1
                        _select_next()

                    game.request_selection(
                        GamePhase.SelectSource, player, on_source_selected,
                        valid, is_optional=False,
                        prompt=f"Select a [Mineral] or [Rock] digivolution card to trash ({trashed_holder[0]+1}/3).")

                _select_next()

            effect.set_on_process_callback(process)
            return effect

        effects.append(build_unsuspend_effect(is_when_digivolving=True))
        effects.append(build_unsuspend_effect(is_on_attack=True))

        # [End of Your Turn] [Once Per Turn] You may place up to 3 cards with the [Mineral]
        # or [Rock] trait from your trash as this Digimon's bottom digivolution cards.
        effect_eot = ICardEffect()
        effect_eot.set_timing(EffectTiming.OnEndTurn)
        effect_eot.set_effect_name("EX8-055 Place up to 3 Mineral/Rock cards from trash as bottom sources")
        effect_eot.set_effect_description(
            "[End of Your Turn] [Once Per Turn] You may place up to 3 cards with the [Mineral] "
            "or [Rock] trait from your trash as this Digimon's bottom digivolution cards."
        )
        effect_eot.is_optional = True
        effect_eot.set_max_count_per_turn(1)
        effect_eot.set_hash_string("EOT_EX8_055")

        def condition_eot(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner or not owner.is_my_turn:
                return False
            # Must have at least 1 Mineral/Rock card in trash
            return any(
                'Mineral' in getattr(c, 'card_traits', []) or 'Rock' in getattr(c, 'card_traits', [])
                for c in owner.trash_cards
            )

        effect_eot.set_can_use_condition(condition_eot)

        def process_eot(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            def _mineral_rock_filter(c):
                traits = getattr(c, 'card_traits', [])
                return 'Mineral' in traits or 'Rock' in traits

            # Let agent select up to 3 Mineral/Rock cards from trash
            placed_holder = [0]

            def _place_one():
                if placed_holder[0] >= 3:
                    return
                eligible = [c for c in player.trash_cards if _mineral_rock_filter(c)]
                if not eligible:
                    return

                from ....data.enums import GamePhase
                _SEL_TRASH_START = 130
                valid_trash = []
                for i, c in enumerate(player.trash_cards):
                    if _mineral_rock_filter(c):
                        valid_trash.append(_SEL_TRASH_START + i)
                if not valid_trash:
                    return

                def on_trash_selected(action_id):
                    idx = action_id - _SEL_TRASH_START
                    if 0 <= idx < len(player.trash_cards):
                        selected = player.trash_cards[idx]
                        player.trash_cards.remove(selected)
                        perm.add_card_source_bottom(selected)
                        placed_holder[0] += 1
                        _place_one()  # recurse for next selection

                game.request_selection(
                    GamePhase.SelectTrash, player, on_trash_selected,
                    valid_trash, is_optional=True,
                    prompt=f"Select a [Mineral] or [Rock] card from trash to place as bottom digivolution card ({placed_holder[0]+1}/3).")

            _place_one()

        effect_eot.set_on_process_callback(process_eot)
        effects.append(effect_eot)

        return effects
