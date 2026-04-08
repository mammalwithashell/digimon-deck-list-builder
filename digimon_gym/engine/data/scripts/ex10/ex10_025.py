from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_025(CardScript):
    """EX10-025 Sunarizamon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [On Play] You may place 2 cards with the [Mineral] or [Rock] trait from your trash
        # as 1 of your [Mineral] or [Rock] trait Digimon's bottom digivolution cards.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX10-025 Place 2 as bottom digivolution sources")
        effect0.set_effect_description(
            "[On Play] You may place 2 cards with the [Mineral] or [Rock] trait from your trash "
            "as 1 of your [Mineral] or [Rock] trait Digimon's bottom digivolution cards."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def source_filter(source_card) -> bool:
                traits = getattr(source_card, 'card_traits', [])
                return 'Mineral' in traits or 'Rock' in traits

            if not any(source_filter(c) for c in player.trash_cards):
                return

            def target_filter(p):
                return p.is_digimon and (p.has_trait('Mineral') or p.has_trait('Rock'))

            if not any(target_filter(p) for p in player.battle_area):
                return

            # C# order: select up to 2 Mineral/Rock cards from trash FIRST,
            # then select the permanent target to place them under.
            selected_cards_holder = []

            def _select_trash_card():
                if len(selected_cards_holder) >= 2:
                    # Done selecting trash cards, move to permanent selection
                    _select_permanent()
                    return

                from ....data.enums import GamePhase
                from ....game.constants import SEL_TRASH_START

                eligible = [c for c in player.trash_cards if source_filter(c)]
                if not eligible:
                    if selected_cards_holder:
                        _select_permanent()
                    return

                valid_trash = []
                for i, c in enumerate(player.trash_cards):
                    if source_filter(c):
                        valid_trash.append(SEL_TRASH_START + i)
                if not valid_trash:
                    if selected_cards_holder:
                        _select_permanent()
                    return

                max_count = min(2, len(eligible))

                def on_trash_selected(action_id):
                    idx = action_id - SEL_TRASH_START
                    if 0 <= idx < len(player.trash_cards):
                        selected = player.trash_cards[idx]
                        player.trash_cards.remove(selected)
                        selected_cards_holder.append(selected)
                        _select_trash_card()

                def on_trash_declined():
                    # Player declined further selection
                    if selected_cards_holder:
                        _select_permanent()

                game.request_selection(
                    GamePhase.SelectTrash, player, on_trash_selected,
                    valid_trash, is_optional=True,
                    prompt=f"Select a [Mineral] or [Rock] card from trash ({len(selected_cards_holder)+1}/{max_count}).",
                    on_decline=on_trash_declined)

            def _select_permanent():
                if not selected_cards_holder:
                    return

                def on_target(target_perm):
                    for sc in selected_cards_holder:
                        target_perm.add_card_source_bottom(sc)

                def on_perm_declined():
                    # Player declined: return selected cards to trash
                    for sc in selected_cards_holder:
                        player.trash_cards.append(sc)
                    selected_cards_holder.clear()

                game.effect_select_own_permanent(
                    player, on_target, filter_fn=target_filter, is_optional=True)

            _select_trash_card()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Inherited Effect: When effects trash this card from a [Mineral] or [Rock] trait
        # Digimon's digivolution cards, delete 1 of your opponent's Digimon with a play cost
        # of 4 or less.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDigivolutionCardDiscarded)
        effect1.set_effect_name("EX10-025 Delete opponent's Digimon cost 4 or less")
        effect1.set_effect_description(
            "When effects trash this card from a [Mineral] or [Rock] trait Digimon's "
            "digivolution cards, delete 1 of your opponent's Digimon with a play cost of 4 or less."
        )
        effect1.is_inherited_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Check this card was the one trashed
            trashed_cards = context.get('trashed_cards', [])
            if card not in trashed_cards:
                return False
            # Check the parent permanent has [Mineral] or [Rock] trait
            # Use event_permanent (the permanent from which cards were trashed)
            event_perm = context.get('event_permanent')
            if event_perm is None:
                # Fall back to permanent for direct-invoke contexts
                event_perm = context.get('permanent')
            if event_perm is None:
                return False
            if not (event_perm.has_trait('Mineral') or event_perm.has_trait('Rock')):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return p.is_digimon and p.top_card.get_cost_itself <= 4

            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
