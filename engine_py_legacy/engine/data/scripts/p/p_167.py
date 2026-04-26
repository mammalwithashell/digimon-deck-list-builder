from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_167(CardScript):
    """P-167 Landramon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _has_mineral_or_rock_trait(c) -> bool:
            return any(
                t in ('Mineral', 'Rock')
                for t in getattr(c, 'card_traits', [])
            )

        def _player_has_any_trashable_source(player) -> bool:
            for perm in player.battle_area:
                if not perm.is_digimon:
                    continue
                if perm.has_no_digivolution_cards:
                    continue
                sources_below = perm.card_sources[:-1] if len(perm.card_sources) > 1 else []
                if any(_has_mineral_or_rock_trait(cs) for cs in sources_below):
                    return True
            return False

        def _perm_has_trashable_source(perm) -> bool:
            if not perm.is_digimon:
                return False
            if perm.has_no_digivolution_cards:
                return False
            sources_below = perm.card_sources[:-1] if len(perm.card_sources) > 1 else []
            return any(_has_mineral_or_rock_trait(cs) for cs in sources_below)

        # Because effect_reveal_and_select doesn't support a post-selection branch
        # (hand vs bottom-digi-source), we implement the reveal manually.

        def _execute_reveal_and_select_branch(player, game, this_perm):
            """
            1. Reveal top 3 of deck.
            2. Among them, pick 1 Mineral/Rock card.
            3. Choose: add to hand OR place as this Digimon's bottom digi-source.
            4. Return the rest to the top or bottom of the deck.
            """
            if not player.library_cards:
                return

            reveal_count = min(3, len(player.library_cards))
            revealed = [player.library_cards.pop(0) for _ in range(reveal_count)]
            mineral_rock_cards = [c for c in revealed if _has_mineral_or_rock_trait(c)]

            if not mineral_rock_cards:
                # No valid targets — return all to top
                for c in reversed(revealed):
                    player.library_cards.insert(0, c)
                return

            # Select 1 Mineral/Rock card from the revealed set
            selected_holder = [None]

            def pick_card(idx):
                # idx maps to mineral_rock_cards
                if idx < len(mineral_rock_cards):
                    selected_holder[0] = mineral_rock_cards[idx]

            # Use choose_branch to pick among matching cards
            if len(mineral_rock_cards) == 1:
                pick_card(0)
                selected = selected_holder[0]
                remaining = [c for c in revealed if c is not selected]
                _apply_branch(player, game, this_perm, selected, remaining)
            else:
                labels = [
                    ', '.join(getattr(c, 'card_names', ['?'])) for c in mineral_rock_cards
                ]

                def on_card_chosen(choice_idx):
                    sel = mineral_rock_cards[choice_idx] if choice_idx < len(mineral_rock_cards) else mineral_rock_cards[0]
                    rem = [c for c in revealed if c is not sel]
                    _apply_branch(player, game, this_perm, sel, rem)

                game.effect_choose_branch(
                    player, len(mineral_rock_cards), on_card_chosen,
                    prompt="Select 1 [Mineral] or [Rock] trait card",
                    branch_labels=labels,
                )

        def _apply_branch(player, game, this_perm, selected, remaining):
            def on_branch(choice_idx):
                if choice_idx == 0:
                    # Add to hand
                    player.hand_cards.append(selected)
                else:
                    # Place as this Digimon's bottom digi-source
                    if this_perm is not None and selected not in this_perm.card_sources:
                        this_perm.add_card_source_bottom(selected)
                    else:
                        player.hand_cards.append(selected)

                # Return remaining to top or bottom (player chooses)
                if remaining:
                    def on_top_bottom(choice_idx):
                        if choice_idx == 0:
                            # Top of deck
                            for c in reversed(remaining):
                                player.library_cards.insert(0, c)
                        else:
                            # Bottom of deck
                            for c in remaining:
                                player.library_cards.append(c)

                    game.effect_choose_branch(
                        player, 2, on_top_bottom,
                        prompt="Return remaining cards to top or bottom of deck?",
                        branch_labels=["Top of deck", "Bottom of deck"],
                    )

            game.effect_choose_branch(
                player, 2, on_branch,
                prompt="Add to hand or place as bottom digivolution card?",
                branch_labels=["Add to hand", "Bottom digivolution card"],
            )

        # ─── Effect 0: [When Digivolving] ─────────────────────────────────
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("P-167 [When Digivolving] Trash source, reveal 3, add/place Mineral/Rock card")
        effect0.set_effect_description(
            "[When Digivolving] By trashing 1 [Mineral] or [Rock] trait card from any of your Digimon's digivolution cards, reveal the top 3 cards of your deck. Among them, add 1 [Mineral] or [Rock] trait card to the hand, or place 1 such card as this Digimon's bottom digivolution card. Return the rest to the top or bottom of the deck."
        )
        effect0.is_optional = True
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            if not _player_has_any_trashable_source(owner):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            this_perm = card.permanent_of_this_card() if card else None

            def trash_src_filter(perm):
                return _perm_has_trashable_source(perm)

            def on_trash_perm(trash_perm):
                sources_below = trash_perm.card_sources[:-1] if len(trash_perm.card_sources) > 1 else []
                mr_sources = [cs for cs in sources_below if _has_mineral_or_rock_trait(cs)]
                if not mr_sources:
                    return

                # Trash the first Mineral/Rock source (player picks via choose_branch
                # only when multiple options exist)
                def do_trash_and_reveal(cs_to_trash):
                    trashed = trash_perm.trash_specific_digivolution_cards([cs_to_trash])
                    player.trash_cards.extend(trashed)
                    _execute_reveal_and_select_branch(player, game, this_perm)

                if len(mr_sources) == 1:
                    do_trash_and_reveal(mr_sources[0])
                else:
                    labels = [', '.join(getattr(c, 'card_names', ['?'])) for c in mr_sources]

                    def on_source_chosen(idx):
                        chosen = mr_sources[idx] if idx < len(mr_sources) else mr_sources[0]
                        do_trash_and_reveal(chosen)

                    game.effect_choose_branch(
                        player, len(mr_sources), on_source_chosen,
                        prompt="Select 1 [Mineral] or [Rock] digivolution card to trash",
                        branch_labels=labels,
                    )

            game.effect_select_own_permanent(
                player, on_trash_perm, filter_fn=trash_src_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ─── Effect 1: [Start of Your Main Phase] ─────────────────────────
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnStartMainPhase)
        effect1.set_effect_name("P-167 [Start of Main Phase] Trash source, reveal 3, add/place Mineral/Rock card")
        effect1.set_effect_description(
            "[Start of Your Main Phase] [When Digivolving] By trashing 1 [Mineral] or [Rock] trait card from any of your Digimon's digivolution cards, reveal the top 3 cards of your deck. Among them, add 1 [Mineral] or [Rock] trait card to the hand, or place 1 such card as this Digimon's bottom digivolution card. Return the rest to the top or bottom of the deck."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            if not owner.is_my_turn:
                return False
            if not _player_has_any_trashable_source(owner):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            this_perm = card.permanent_of_this_card() if card else None

            def trash_src_filter(perm):
                return _perm_has_trashable_source(perm)

            def on_trash_perm(trash_perm):
                sources_below = trash_perm.card_sources[:-1] if len(trash_perm.card_sources) > 1 else []
                mr_sources = [cs for cs in sources_below if _has_mineral_or_rock_trait(cs)]
                if not mr_sources:
                    return

                def do_trash_and_reveal(cs_to_trash):
                    trashed = trash_perm.trash_specific_digivolution_cards([cs_to_trash])
                    player.trash_cards.extend(trashed)
                    _execute_reveal_and_select_branch(player, game, this_perm)

                if len(mr_sources) == 1:
                    do_trash_and_reveal(mr_sources[0])
                else:
                    labels = [', '.join(getattr(c, 'card_names', ['?'])) for c in mr_sources]

                    def on_source_chosen(idx):
                        chosen = mr_sources[idx] if idx < len(mr_sources) else mr_sources[0]
                        do_trash_and_reveal(chosen)

                    game.effect_choose_branch(
                        player, len(mr_sources), on_source_chosen,
                        prompt="Select 1 [Mineral] or [Rock] digivolution card to trash",
                        branch_labels=labels,
                    )

            game.effect_select_own_permanent(
                player, on_trash_perm, filter_fn=trash_src_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ─── Effect 2: Inherited — OnDigivolutionCardDiscarded ─────────────
        # When effects trash this card from digivolution cards of a [Mineral] or [Rock]
        # trait Digimon, <De-Digivolve 1> 1 of your opponent's Digimon.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDigivolutionCardDiscarded)
        effect2.set_effect_name("P-167 De-Digivolve 1 opponent Digimon")
        effect2.set_effect_description(
            "When effects trash this card from digivolution cards of a [Mineral] or [Rock] trait Digimon, <De-Digivolve 1> 1 of your opponent's Digimon."
        )
        effect2.is_inherited_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Must be THIS card that was trashed, not just any card
            trashed_cards = context.get('trashed_cards', [])
            if card not in trashed_cards:
                return False
            # The Digimon whose sources were trashed must have Mineral or Rock trait
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

            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy
                if enemy and removed:
                    enemy.trash_cards.extend(removed)

            game.effect_select_opponent_permanent(
                player, on_de_digivolve,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
