from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_112(CardScript):
    """BT13-112 Omnimon | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _make_effect(is_on_play: bool):
            """Create the shared [On Play]/[When Digivolving] effect."""
            eff = ICardEffect()
            eff.set_timing(EffectTiming.OnEnterFieldAnyone)
            eff.set_effect_name("BT13-112 Delete 1 Digimon or play Royal Knights from breeding")
            if is_on_play:
                eff.set_effect_description(
                    "[On Play] Delete 1 of your opponent's Digimon, or you may "
                    "play 1 of each Digimon with the [Royal Knight] trait and a "
                    "different name from the digivolution cards of your Digimon "
                    "in the breeding area without paying their costs. When a "
                    "Digimon is played by this effect, trash 1 of your Digimon "
                    "in the breeding area, and all of your Digimon gain <Rush> "
                    "for the turn."
                )
                eff.is_on_play = True
            else:
                eff.set_effect_description(
                    "[When Digivolving] Delete 1 of your opponent's Digimon, or "
                    "you may play 1 of each Digimon with the [Royal Knight] "
                    "trait and a different name from the digivolution cards of "
                    "your Digimon in the breeding area without paying their "
                    "costs. When a Digimon is played by this effect, trash 1 of "
                    "your Digimon in the breeding area, and all of your Digimon "
                    "gain <Rush> for the turn."
                )
                eff.is_when_digivolving = True

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                return True

            eff.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return

                def on_branch(branch_index: int):
                    if branch_index == 0:
                        # Branch 0: Delete 1 of your opponent's Digimon
                        def delete_filter(p):
                            return p.is_digimon

                        def on_delete(target_perm):
                            enemy = player.enemy
                            if enemy:
                                enemy.delete_permanent(target_perm)

                        game.effect_select_opponent_permanent(
                            player, on_delete, filter_fn=delete_filter,
                            is_optional=False,
                            prompt="Delete 1 of your opponent's Digimon.")

                    elif branch_index == 1:
                        # Branch 1: Player selects which Royal Knight(s) with different names
                        # to play from the breeding area's digivolution card sources.
                        # C# ref: SelectCardEffect with unique name constraint (canNoSelect=true).
                        breeding = player.breeding_area
                        if not breeding:
                            return

                        top = breeding.top_card

                        def rk_filter(cs):
                            if cs is top:
                                return False
                            if not getattr(cs, 'is_digimon', False):
                                return False
                            if not any('Royal Knight' in t for t in getattr(cs, 'card_traits', [])):
                                return False
                            return True

                        rk_candidates = [cs for cs in breeding.card_sources if rk_filter(cs)]
                        if not rk_candidates:
                            return

                        # Use recursive selection to let player pick one-at-a-time
                        # enforcing unique names (per C# CanTargetCondition_ByPreSelecetedList)
                        played_names: set = set()
                        any_played = [False]

                        def _select_next_rk():
                            # Filter to RK candidates not yet played by name
                            remaining = [cs for cs in breeding.card_sources if rk_filter(cs)
                                         and getattr(cs, 'card_names', [''])[0] not in played_names]
                            if not remaining:
                                _finalize_rk_play()
                                return

                            from ....game.constants import SEL_HAND_START
                            valid = []
                            for i, cs in enumerate(breeding.card_sources):
                                if rk_filter(cs) and getattr(cs, 'card_names', [''])[0] not in played_names:
                                    valid.append(SEL_HAND_START + i)
                            if not valid:
                                _finalize_rk_play()
                                return

                            def on_rk_select(action_id: int):
                                idx = action_id - SEL_HAND_START
                                if not (0 <= idx < len(breeding.card_sources)):
                                    _finalize_rk_play()
                                    return
                                chosen = breeding.card_sources[idx]
                                cs_names = getattr(chosen, 'card_names', [])
                                primary_name = cs_names[0] if cs_names else ''
                                played_names.add(primary_name)
                                breeding.card_sources.remove(chosen)
                                played = player.play_card_from_source(chosen, pay_cost=False)
                                if played:
                                    game.execute_effects(
                                        EffectTiming.OnEnterFieldAnyone,
                                        {"played_card": chosen, "played_permanent": played,
                                         "event_player": player},
                                    )
                                    any_played[0] = True
                                # Continue selecting more
                                _select_next_rk()

                            from ....data.enums import GamePhase
                            game.request_selection(
                                GamePhase.SelectSource, player, on_rk_select, valid,
                                is_optional=True,
                                prompt="Select a [Royal Knight] Digimon from breeding digivolution cards to play (unique names only)."
                            )

                        def _finalize_rk_play():
                            if any_played[0]:
                                # Trash breeding area Digimon
                                if player.breeding_area is not None:
                                    breeding_perm = player.breeding_area
                                    for cs in list(breeding_perm.card_sources):
                                        player.trash_cards.append(cs)
                                    player.breeding_area = None
                                # All of your Digimon gain Rush for the turn
                                for p in list(player.battle_area):
                                    if p.is_digimon:
                                        p.grant_keyword('_is_rush')

                        _select_next_rk()

                game.effect_choose_branch(
                    player, 2, on_branch,
                    prompt="Choose: Delete opponent's Digimon or play Royal Knights from breeding?",
                    branch_labels=[
                        "Delete 1 opponent Digimon",
                        "Play Royal Knights from breeding area",
                    ],
                )

            eff.set_on_process_callback(process)
            return eff

        # effect0: [On Play]
        effect0 = _make_effect(is_on_play=True)
        effects.append(effect0)

        # effect1: [When Digivolving]
        effect1 = _make_effect(is_on_play=False)
        effects.append(effect1)

        return effects
