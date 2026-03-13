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
                        # Branch 1: Play 1 of each Royal Knight with a different
                        # name from the breeding area's digivolution card sources,
                        # then trash breeding area Digimon and grant Rush.
                        breeding = player.breeding_area
                        if not breeding:
                            return

                        # Collect all under-cards (digivolution sources — skip top_card)
                        top = breeding.top_card
                        played_names: set = set()
                        any_played = False

                        for cs in list(breeding.card_sources):
                            if cs is top:
                                continue
                            if not getattr(cs, 'is_digimon', False):
                                continue
                            if not any('Royal Knight' in t
                                       for t in getattr(cs, 'card_traits', [])):
                                continue
                            # Unique name check
                            cs_names = getattr(cs, 'card_names', [])
                            if not cs_names:
                                continue
                            primary_name = cs_names[0]
                            if primary_name in played_names:
                                continue
                            played_names.add(primary_name)

                            # Remove from breeding stack and play free
                            breeding.card_sources.remove(cs)
                            played = player.play_card_from_source(cs, pay_cost=False)
                            if played:
                                game.execute_effects(
                                    EffectTiming.OnEnterFieldAnyone,
                                    {"played_card": cs, "played_permanent": played,
                                     "event_player": player},
                                )
                                any_played = True

                        if any_played:
                            # Trash 1 of your Digimon in the breeding area
                            if player.breeding_area is not None:
                                breeding_perm = player.breeding_area
                                for cs in list(breeding_perm.card_sources):
                                    player.trash_cards.append(cs)
                                player.breeding_area = None

                            # All of your Digimon gain Rush for the turn
                            for p in list(player.battle_area):
                                if p.is_digimon:
                                    p.grant_keyword('_is_rush')

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
