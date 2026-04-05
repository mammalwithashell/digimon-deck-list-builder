from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, GamePhase

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX5_015(CardScript):
    """EX5-015 Gabumon (X Antibody) | Lv.3 Blue/White Digimon

    [On Play] [When Digivolving] Reveal the top 4 cards of your deck. Add 2
        cards with [Garurumon] or [X Antibody] in their names among them to the
        hand. Place the rest at the bottom of the deck. If you added cards, trash
        1 card in your hand.

    Inherited:
    [All Turns] [Once Per Turn] When this Digimon with [Garurumon] or [Omnimon]
        in its name would be deleted in battle, by returning 2 non-Digi-Egg cards
        from your trash to the bottom of the deck, prevent that deletion.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        effect0 = ICardEffect()
        effect0.set_effect_name("EX5-015 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 0
        effect0._alt_digi_name = "Tsunomon"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Name filter for reveal ---
        def _name_filter(c):
            """Match cards with [Garurumon] or [X Antibody] in name."""
            names = getattr(c, 'card_names', []) or []
            for name in names:
                if 'Garurumon' in name or 'X Antibody' in name:
                    return True
            return False

        # --- Shared reveal-and-add logic ---
        def _reveal_and_add(ctx: Dict[str, Any]):
            """Reveal top 4, add up to 2 [Garurumon]/[X Antibody] to hand,
            rest to bottom. If any added, trash 1 from hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Use effect_reveal_and_select_multi with 2 passes of same filter
            # Each pass picks 1 card matching the filter -> hand
            # Remaining go to deck_bottom
            game.effect_reveal_and_select_multi(
                player, 4,
                [(_name_filter, 'hand'), (_name_filter, 'hand')],
                remaining_placement='deck_bottom',
                is_optional=True,
            )

            # After reveal resolves, trash 1 from hand (mandatory if cards added,
            # but we make optional since the engine doesn't track "if you added")
            def _trash_filter(c):
                return True

            def _on_trash(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)

            game.effect_select_hand_card(
                player, _trash_filter, _on_trash, is_optional=True)

        # --- Effect 1: [On Play] ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX5-015 On Play: Reveal top 4")
        effect1.set_effect_description(
            "[On Play] Reveal the top 4 cards of your deck. Add 2 cards with "
            "[Garurumon] or [X Antibody] in their names among them to the hand. "
            "Place the rest at the bottom of the deck. If you added cards, "
            "trash 1 card in your hand."
        )
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_reveal_and_add)
        effects.append(effect1)

        # --- Effect 2: [When Digivolving] ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX5-015 When Digivolving: Reveal top 4")
        effect2.set_effect_description(
            "[When Digivolving] Reveal the top 4 cards of your deck. Add 2 cards "
            "with [Garurumon] or [X Antibody] in their names among them to the hand. "
            "Place the rest at the bottom of the deck. If you added cards, "
            "trash 1 card in your hand."
        )
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_reveal_and_add)
        effects.append(effect2)

        # --- Effect 3: Inherited deletion prevention ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.WhenPermanentWouldBeDeleted)
        effect3.set_effect_name("EX5-015 Inherited: Prevent deletion")
        effect3.set_effect_description(
            "[All Turns] [Once Per Turn] When this Digimon with [Garurumon] or "
            "[Omnimon] in its name would be deleted in battle, by returning 2 "
            "non-Digi-Egg cards from your trash to the bottom of the deck, "
            "prevent that deletion."
        )
        effect3.is_inherited_effect = True
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("Substitute_EX5_015")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if not perm:
                return False
            # Must be a Digimon with [Garurumon] or [Omnimon] in name
            if not (perm.contains_card_name('Garurumon') or
                    perm.contains_card_name('Omnimon')):
                return False
            # Must have 2 non-Digi-Egg cards in trash
            owner = card.owner
            if not owner:
                return False
            non_egg_trash = [c for c in owner.trash_cards
                           if not getattr(c, 'is_digi_egg', False)]
            if len(non_egg_trash) < 2:
                return False
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Return 2 non-Digi-Egg cards from trash to deck bottom, prevent deletion."""
            __SEL_TRASH_START = 130
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def trash_filter(c):
                return not getattr(c, 'is_digi_egg', False)

            _count = [0]

            def _select_second(action_id):
                idx = action_id - _SEL_TRASH_START
                if 0 <= idx < len(player.trash_cards):
                    chosen = player.trash_cards[idx]
                    player.trash_cards.remove(chosen)
                    player.library_cards.append(chosen)
                    _count[0] += 1

            def _select_first(action_id):
                idx = action_id - _SEL_TRASH_START
                if 0 <= idx < len(player.trash_cards):
                    chosen = player.trash_cards[idx]
                    player.trash_cards.remove(chosen)
                    player.library_cards.append(chosen)
                    _count[0] += 1

                # Now select second card
                valid2 = []
                for i, c in enumerate(player.trash_cards):
                    if trash_filter(c):
                        valid2.append(_SEL_TRASH_START + i)
                if valid2:
                    game.request_selection(
                        GamePhase.SelectTrash, player, _select_second, valid2,
                        is_optional=False,
                        prompt="Select 2nd non-Digi-Egg card to return to deck bottom."
                    )

            valid1 = []
            for i, c in enumerate(player.trash_cards):
                if trash_filter(c):
                    valid1.append(_SEL_TRASH_START + i)

            if len(valid1) >= 2:
                game.request_selection(
                    GamePhase.SelectTrash, player, _select_first, valid1,
                    is_optional=False,
                    prompt="Select 1st non-Digi-Egg card to return to deck bottom."
                )
            elif len(valid1) >= 1:
                # Only 1 available, auto-select
                _select_first(valid1[0])

            # Prevent the deletion
            ctx['prevent_deletion'] = True

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
