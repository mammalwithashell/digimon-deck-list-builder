from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....interfaces.modifiers import ModifierType
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_028(CardScript):
    """EX10-028 Landramon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _has_mineral_or_rock_trait(c) -> bool:
            return any(
                t in ('Mineral', 'Rock')
                for t in getattr(c, 'card_traits', [])
            )

        def _perm_has_trashable_source(perm) -> bool:
            """Return True if perm has at least 1 Mineral/Rock digi-source to trash."""
            if perm.has_no_digivolution_cards:
                return False
            # card_sources is bottom-to-top; top card is the Digimon itself — skip it
            sources_below = perm.card_sources[:-1] if len(perm.card_sources) > 1 else []
            return any(_has_mineral_or_rock_trait(cs) for cs in sources_below)

        def _player_has_any_trashable_source(player) -> bool:
            return any(_perm_has_trashable_source(p) for p in player.battle_area if p.is_digimon)

        def build_main_effect(is_on_play: bool):
            eff = ICardEffect()
            eff.set_timing(EffectTiming.OnEnterFieldAnyone)
            trigger_label = "[On Play]" if is_on_play else "[When Digivolving]"
            eff.set_effect_name(
                f"EX10-028 {trigger_label} Trash 1 Mineral/Rock source, grant Reboot+Blocker+3000DP"
            )
            eff.set_effect_description(
                f"{trigger_label} By trashing any 1 card with the [Mineral] or [Rock] trait from your Digimon's digivolution cards, 1 of your Digimon with the [Mineral] or [Rock] trait gains <Reboot>, <Blocker> and +3000 DP until your opponent's turn ends."
            )
            eff.is_optional = True
            if is_on_play:
                eff.is_on_play = True
            else:
                eff.is_when_digivolving = True

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                owner = card.owner if card else None
                if not owner:
                    return False
                if not _player_has_any_trashable_source(owner):
                    return False
                return True

            eff.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return

                # Step 1: Select which of YOUR Digimon to trash a source from.
                def trash_source_filter(perm):
                    return perm.is_digimon and _perm_has_trashable_source(perm)

                def on_trash_perm_selected(trash_perm):
                    # Find Mineral/Rock sources in the stack (below top card).
                    sources_below = trash_perm.card_sources[:-1] if len(trash_perm.card_sources) > 1 else []
                    mineral_rock_sources = [
                        cs for cs in sources_below if _has_mineral_or_rock_trait(cs)
                    ]
                    if not mineral_rock_sources:
                        return

                    def _do_trash_and_grant(cs_to_trash):
                        trashed = trash_perm.trash_specific_digivolution_cards([cs_to_trash])
                        if not trashed:
                            return  # cut-in saved the card
                        player.trash_cards.extend(trashed)

                        # Step 2: Select a Mineral/Rock Digimon to receive the buff.
                        def grant_filter(perm2):
                            return perm2.is_digimon and (
                                perm2.has_trait('Mineral') or perm2.has_trait('Rock')
                            )

                        def on_grant_selected(target_perm):
                            game.register_modifier(
                                target_perm,
                                ModifierType.GRANT_REBOOT,
                                expiry='end_of_opponent_turn',
                            )
                            game.register_modifier(
                                target_perm,
                                ModifierType.GRANT_BLOCKER,
                                expiry='end_of_opponent_turn',
                            )
                            game.register_modifier(
                                target_perm,
                                ModifierType.CHANGE_DP,
                                value_fn=lambda cur, t, c: cur + 3000,
                                expiry='end_of_opponent_turn',
                            )

                        game.effect_select_own_permanent(
                            player, on_grant_selected, filter_fn=grant_filter, is_optional=False)

                    # Player selects which source to trash if multiple options
                    if len(mineral_rock_sources) == 1:
                        _do_trash_and_grant(mineral_rock_sources[0])
                    else:
                        labels = [
                            ', '.join(getattr(c, 'card_names', ['?'])) for c in mineral_rock_sources
                        ]

                        def on_source_chosen(idx):
                            chosen = mineral_rock_sources[idx] if idx < len(mineral_rock_sources) else mineral_rock_sources[0]
                            _do_trash_and_grant(chosen)

                        game.effect_choose_branch(
                            player, len(mineral_rock_sources), on_source_chosen,
                            prompt="Select 1 [Mineral] or [Rock] digivolution card to trash",
                            branch_labels=labels,
                        )

                game.effect_select_own_permanent(
                    player, on_trash_perm_selected, filter_fn=trash_source_filter, is_optional=True)

            eff.set_on_process_callback(process)
            return eff

        effects.append(build_main_effect(is_on_play=True))
        effects.append(build_main_effect(is_on_play=False))

        # Timing: EffectTiming.OnDigivolutionCardDiscarded
        # Inherited: When effects trash this card from a [Mineral] or [Rock] trait
        # Digimon's digivolution cards, delete 1 of your opponent's Digimon with a
        # play cost of 4 or less.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDigivolutionCardDiscarded)
        effect2.set_effect_name("EX10-028 Delete opponent Digimon cost 4 or less")
        effect2.set_effect_description(
            "When effects trash this card from a [Mineral] or [Rock] trait Digimon's digivolution cards, delete 1 of your opponent's Digimon with a play cost of 4 or less."
        )
        effect2.is_inherited_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Check this card was the one trashed
            trashed_cards = context.get('trashed_cards', [])
            if card not in trashed_cards:
                return False
            # Check the permanent has [Mineral] or [Rock] trait
            event_perm = context.get('event_permanent')
            if event_perm is None:
                return False
            if not (event_perm.has_trait('Mineral') or event_perm.has_trait('Rock')):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return p.is_digimon and p.top_card.get_cost_itself <= 4

            def on_delete(target):
                enemy = player.enemy
                if enemy:
                    enemy.delete_permanent(target)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
