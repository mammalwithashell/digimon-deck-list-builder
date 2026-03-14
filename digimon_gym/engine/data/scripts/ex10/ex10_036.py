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

        # Alternate digivolution: If you have [Close] or [Proganomon], digivolve for cost 6
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-036 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 6
        effect0._alt_digi_name = "Close"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Close') or permanent.contains_card_name('Proganomon'))):
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Keyword: Fragment (3)
        effect_fragment = ICardEffect()
        effect_fragment.set_effect_name("EX10-036 Fragment")
        effect_fragment.set_effect_description("Fragment")
        effect_fragment._is_fragment = True
        effect_fragment._fragment_count = 3

        def condition_fragment(context: Dict[str, Any]) -> bool:
            return True

        effect_fragment.set_can_use_condition(condition_fragment)
        effects.append(effect_fragment)

        # Helper: check if player has at least 3 [Mineral] or [Rock] trait cards in trash
        def _has_mineral_rock_in_trash(player) -> bool:
            if not player:
                return False
            count = 0
            for c in player.trash_cards:
                traits = set(getattr(c, 'card_traits', []) or [])
                if 'Mineral' in traits or 'Rock' in traits:
                    count += 1
                    if count >= 3:
                        return True
            return False

        # Helper: check if player has at least 3 [Mineral] or [Rock] divi-cards across all Digimon
        def _has_mineral_rock_in_sources(player) -> bool:
            if not player:
                return False
            count = 0
            for perm in player.battle_area:
                for c in perm.digivolution_cards:
                    traits = set(getattr(c, 'card_traits', []) or [])
                    if 'Mineral' in traits or 'Rock' in traits:
                        count += 1
                        if count >= 3:
                            return True
            return False

        # [When Digivolving] [When Attacking] By trashing 3 [Mineral] or [Rock] trait cards
        # from any of your Digimon's digivolution cards, delete 1 of your opponent's Digimon
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
                return _has_mineral_rock_in_sources(owner)

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return

                # Collect up to 3 Mineral/Rock divi-cards from any of player's Digimon
                trashed_sources = []
                for field_perm in player.battle_area:
                    if len(trashed_sources) >= 3:
                        break
                    to_trash_from = []
                    for c in list(field_perm.digivolution_cards):
                        traits = set(getattr(c, 'card_traits', []) or [])
                        if 'Mineral' in traits or 'Rock' in traits:
                            to_trash_from.append(c)
                    # Trash up to (3 - already collected) from this Digimon
                    needed = 3 - len(trashed_sources)
                    removed = field_perm.trash_digivolution_cards(
                        min(needed, len(to_trash_from))
                    )
                    trashed_sources.extend(removed)

                if trashed_sources:
                    player.trash_cards.extend(trashed_sources)

                # Select opponent Digimon to delete
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

            effect.set_on_process_callback(process)
            return effect

        effects.append(build_delete_effect(is_when_digivolving=True))
        effects.append(build_delete_effect(is_when_digivolving=False))

        # [When Digivolving] [When Attacking] [Once Per Turn] By placing 3 [Mineral] or [Rock]
        # trait cards from your trash as this Digimon's bottom digivolution cards, it unsuspends.
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
                return _has_mineral_rock_in_trash(owner)

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                perm = ctx.get('permanent')
                game = ctx.get('game')
                if not (player and perm and game):
                    return

                def _mineral_rock_filter(c):
                    traits = set(getattr(c, 'card_traits', []) or [])
                    return 'Mineral' in traits or 'Rock' in traits

                # Place exactly 3 Mineral/Rock cards from trash (player selects)
                placed_holder = [0]

                def _place_one():
                    if placed_holder[0] >= 3:
                        # All 3 placed, unsuspend
                        perm.unsuspend()
                        return

                    from ....data.enums import GamePhase
                    _SEL_TRASH_START = 130
                    valid_trash = []
                    for i, c in enumerate(player.trash_cards):
                        if _mineral_rock_filter(c):
                            valid_trash.append(_SEL_TRASH_START + i)
                    if not valid_trash:
                        return  # Not enough cards, don't unsuspend

                    def on_trash_selected(idx):
                        if idx < len(player.trash_cards):
                            selected = player.trash_cards[idx]
                            player.trash_cards.remove(selected)
                            perm.add_card_source_bottom(selected)
                            placed_holder[0] += 1
                            _place_one()

                    # Must place all 3 (not optional for individual selections)
                    game.request_selection(
                        GamePhase.SelectTrash, player, on_trash_selected,
                        valid_trash, is_optional=False,
                        prompt=f"Select a [Mineral] or [Rock] card from trash ({placed_holder[0]+1}/3).")

                _place_one()

            effect.set_on_process_callback(process)
            return effect

        effects.append(build_unsuspend_effect(is_when_digivolving=True))
        effects.append(build_unsuspend_effect(is_when_digivolving=False))

        return effects
