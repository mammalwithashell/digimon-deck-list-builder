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
                return any(
                    'Mineral' in getattr(c, 'card_traits', []) or 'Rock' in getattr(c, 'card_traits', [])
                    for c in owner.trash_cards
                )

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
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

                    def on_trash_selected(idx):
                        if idx < len(player.trash_cards):
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
                        traits = getattr(src, 'card_traits', [])
                        if 'Mineral' in traits or 'Rock' in traits:
                            return True
                return False

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                perm = ctx.get('permanent')
                game = ctx.get('game')
                if not (player and perm and game):
                    return

                from digimon_gym.engine.interfaces.modifiers import ModifierType

                # Count and trash up to 3 Mineral/Rock trait source cards from any of our Digimon
                trashed_count = 0
                for p in list(player.battle_area):
                    if trashed_count >= 3:
                        break
                    if not p.is_digimon:
                        continue
                    for src in list(p.card_sources):
                        if trashed_count >= 3:
                            break
                        if src is p.top_card:
                            continue
                        traits = getattr(src, 'card_traits', [])
                        if 'Mineral' in traits or 'Rock' in traits:
                            p.card_sources.remove(src)
                            player.trash_cards.append(src)
                            trashed_count += 1

                if trashed_count == 0:
                    return

                reduction = trashed_count * 2

                def on_target(target_perm):
                    # Reduce play cost of the specific target by reduction amount
                    # The value_fn must scope to the selected target permanent
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

            effect.set_on_process_callback(process)
            return effect

        effects.append(build_cost_reduce_effect(is_when_digivolving=True))
        effects.append(build_cost_reduce_effect(is_on_attack=True))

        return effects
