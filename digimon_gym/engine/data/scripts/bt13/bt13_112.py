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
                        # Any opponent Digimon — no trait restriction
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
                        # Branch 1: Play 1 of each Royal Knight with a
                        # different name from breeding digi-sources, then
                        # trash breeding and grant Rush to all your Digimon.
                        pass  # descriptive-tagged: play_from_breeding_digi_sources
                        # Full implementation requires iterating
                        # player.breeding_area.card_sources[:-1] for unique-
                        # named Royal Knight Digimon, playing each, trashing
                        # the breeding area Digimon, and granting Rush.
                        # The trash + Rush portion is implemented below for
                        # when the play-from-sources mechanic is available.
                        _trash_breeding_and_grant_rush(player, game)

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

        def _trash_breeding_and_grant_rush(player, game):
            """When a Digimon is played by this effect, trash 1 of your
            Digimon in the breeding area, and all of your Digimon gain
            <Rush> for the turn."""
            # Trash breeding area Digimon
            if player.breeding_area is not None:
                breeding_perm = player.breeding_area
                # Move all cards to trash
                for cs in list(breeding_perm.card_sources):
                    player.trash_cards.append(cs)
                player.breeding_area = None

            # All of your Digimon gain Rush for the turn
            for p in list(player.battle_area):
                if p.is_digimon:
                    p.grant_keyword('_is_rush')

        # effect0: [On Play]
        effect0 = _make_effect(is_on_play=True)
        effects.append(effect0)

        # effect1: [When Digivolving]
        effect1 = _make_effect(is_on_play=False)
        effects.append(effect1)

        return effects
