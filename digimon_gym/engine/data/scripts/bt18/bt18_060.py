from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT18_060(CardScript):
    """BT18-060 Vemmon | Lv.3 | Black | DP 1000 | Cost 3

    [On Play] Reveal the top 3 cards of your deck. Add 1 card with [Vemmon]
    in its text among them to the hand, and place 1 [Vemmon] among them as the
    bottom digivolution card of 1 of your Digimon. Return the rest to the
    bottom of the deck.

    --- Inherited ---
    [Your Turn] [Once Per Turn] When this Digimon would digivolve into a
    Digimon card with [Vemmon] in its text, reduce the digivolution cost by 1.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _has_vemmon_text(c) -> bool:
            """Card whose effect text mentions Vemmon."""
            return 'Vemmon' in getattr(c, 'card_text', '')

        def _is_vemmon_name(c) -> bool:
            """Card named [Vemmon]."""
            return any('Vemmon' in n for n in getattr(c, 'card_names', []))

        # ─── Effect 0: [On Play] Reveal top 3, add 1 Vemmon-text to hand,
        #     place 1 Vemmon (by name) as bottom digi-card under a Digimon,
        #     rest to deck bottom.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT18-060 Reveal top 3, add Vemmon-text to hand + place Vemmon as bottom digi-card")
        effect0.set_effect_description(
            "[On Play] Reveal the top 3 cards of your deck. Add 1 card with [Vemmon] "
            "in its text among them to the hand, and place 1 [Vemmon] among them as "
            "the bottom digivolution card of 1 of your Digimon. Return the rest to "
            "the bottom of the deck.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            from ....data.enums import GamePhase
            SEL_REVEALED_START = 1700

            # Reveal top 3
            revealed = player.library_cards[:3]
            if not revealed:
                return
            for c in revealed:
                player.library_cards.remove(c)
            pool = list(revealed)

            def _finish(remaining):
                """Return remaining cards to deck bottom."""
                game.revealed_cards = []
                for c in remaining:
                    player.library_cards.append(c)

            def _pass2_place_digi_card(remaining):
                """Pass 2: Place 1 [Vemmon] (by name) from remaining as bottom digi-card."""
                vemmon_in_pool = [c for c in remaining if _is_vemmon_name(c)]
                own_digimon = [p for p in player.battle_area if p.is_digimon]
                if not vemmon_in_pool or not own_digimon:
                    _finish(remaining)
                    return

                game.revealed_cards = list(remaining)
                valid = []
                for i, c in enumerate(remaining):
                    if _is_vemmon_name(c):
                        valid.append(SEL_REVEALED_START + i)

                def on_select_card(action_id: int):
                    idx = action_id - SEL_REVEALED_START
                    if 0 <= idx < len(remaining):
                        selected = remaining.pop(idx)
                        game.revealed_cards = []

                        # Now select which of your Digimon to place it under
                        def on_select_perm(target_perm):
                            target_perm.add_card_source_bottom(selected)

                        game.effect_select_own_permanent(
                            player, on_select_perm,
                            filter_fn=lambda p: p.is_digimon,
                            is_optional=False)
                    _finish(remaining)

                def on_decline():
                    game.revealed_cards = []
                    _finish(remaining)

                game.request_selection(
                    GamePhase.SelectReveal, player, on_select_card, valid,
                    is_optional=True,
                    prompt="Select 1 [Vemmon] to place as a bottom digivolution card of 1 of your Digimon.",
                    on_decline=on_decline)

            # Pass 1: Add 1 card with [Vemmon] in its text to hand
            vemmon_text_in_pool = [c for c in pool if _has_vemmon_text(c)]
            if not vemmon_text_in_pool:
                _pass2_place_digi_card(pool)
                return

            game.revealed_cards = list(pool)
            valid1 = []
            for i, c in enumerate(pool):
                if _has_vemmon_text(c):
                    valid1.append(SEL_REVEALED_START + i)

            def on_select_hand(action_id: int):
                idx = action_id - SEL_REVEALED_START
                if 0 <= idx < len(pool):
                    selected = pool.pop(idx)
                    player.hand_cards.append(selected)
                game.revealed_cards = []
                _pass2_place_digi_card(pool)

            def on_decline1():
                game.revealed_cards = []
                _pass2_place_digi_card(pool)

            game.request_selection(
                GamePhase.SelectReveal, player, on_select_hand, valid1,
                is_optional=True,
                prompt="Select 1 card with [Vemmon] in its text to add to your hand.",
                on_decline=on_decline1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ─── Effect 1 (Inherited): [Your Turn] [Once Per Turn] When this Digimon
        #     would digivolve into a Digimon card with [Vemmon] in its text,
        #     reduce the digivolution cost by 1.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.WhenWouldDigivolve)
        effect1.set_effect_name("BT18-060 Inherited: Digi cost -1 for Vemmon-text")
        effect1.set_effect_description(
            "[Your Turn] [Once Per Turn] When this Digimon would digivolve into a "
            "Digimon card with [Vemmon] in its text, reduce the digivolution cost by 1.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("DigiCostReduce_BT18_060")
        effect1.cost_reduction = 1

        def condition1(context: Dict[str, Any]) -> bool:
            # This card must be in a permanent's digi-stack
            if card and card.permanent_of_this_card() is None:
                return False
            # Must be your turn
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # The card being digivolved into must have [Vemmon] in its text
            card_source = context.get('card_source')
            if card_source is None:
                return False
            if not _has_vemmon_text(card_source):
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
