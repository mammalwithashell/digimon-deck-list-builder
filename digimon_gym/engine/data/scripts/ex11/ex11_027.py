from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_027(CardScript):
    """EX11-027 Maquinamon | Lv.3

    Alt digivolution: Lv.2 with [Maquinamon] in text for cost 0.

    [On Play] Reveal the top 3 cards of your deck. Add 1 [Maquinamon] and
    1 card with [Maquinamon] in its text among them to the hand. Return the
    rest to the bottom of the deck. Then, you may link this Digimon or 1
    [Maquinamon] in your hand to 1 of your other Digimon without paying the
    cost.

    [All Turns] When this Digimon would leave the battle area, by placing 1
    of its link cards as its bottom digivolution card, it doesn't leave.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _is_maquinamon_name(c) -> bool:
            names = getattr(c, 'card_names', []) or []
            return any('Maquinamon' in n for n in names)

        def _has_maquinamon_text(c) -> bool:
            text = getattr(c, 'card_text', '') or ''
            return 'Maquinamon' in text

        # --- Alt digivolution: Lv.2 with [Maquinamon] in text for cost 0 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-027 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 0
        effect0._alt_digi_level = 2

        def condition0(context: Dict[str, Any]) -> bool:
            # C# checks: Lv.2 AND has [Maquinamon] in text
            permanent = card.permanent_of_this_card() if card else None
            if not permanent or not permanent.top_card:
                return False
            top = permanent.top_card
            text = getattr(top, 'card_text', '') or ''
            return 'Maquinamon' in text
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- On Play: Reveal top 3, add 1 [Maquinamon] AND 1 with [Maquinamon] text ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX11-027 Reveal top 3, Add [Maquinamon] & 1 [Maquinamon] in text, then may link")
        effect1.set_effect_description(
            "[On Play] Reveal the top 3 cards of your deck. Add 1 [Maquinamon] "
            "and 1 card with [Maquinamon] in its text among them to the hand. "
            "Return the rest to the bottom of the deck. Then, you may link this "
            "Digimon or 1 [Maquinamon] in your hand to 1 of your other Digimon "
            "without paying the cost.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Reveal top 3, two-pass selection (1 Maquinamon + 1 Maquinamon-text), rest to deck bottom. Then link."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            # Two-pass reveal: first pick 1 [Maquinamon] by name, then pick 1 with [Maquinamon] in text
            passes = [
                (_is_maquinamon_name, 'hand'),     # Pass 1: add 1 [Maquinamon]
                (_has_maquinamon_text, 'hand'),     # Pass 2: add 1 with [Maquinamon] text
            ]

            game.effect_reveal_and_select_multi(
                player, 3, passes,
                remaining_placement='deck_bottom',
                is_optional=False)

            # Then, may link this Digimon or 1 [Maquinamon] from hand to another Digimon
            _try_link_after_reveal(player, perm, game)

        def _try_link_after_reveal(player, perm, game):
            """May link this Digimon or 1 [Maquinamon] from hand to another Digimon."""
            if not player.battle_area:
                return
            # Check if there's another Digimon to link to
            other_digimon = [p for p in player.battle_area
                            if p.is_digimon and p is not perm]
            if not other_digimon:
                return

            # Try to link from hand first (Maquinamon cards)
            hand_maq = [c for c in player.hand_cards if _is_maquinamon_name(c)]
            if hand_maq:
                def on_hand_selected(selected_card):
                    if selected_card and selected_card in player.hand_cards:
                        player.hand_cards.remove(selected_card)
                        game.effect_link_to_permanent(player, selected_card, is_optional=True)

                game.effect_select_hand_card(
                    player, _is_maquinamon_name, on_hand_selected,
                    is_optional=True,
                    prompt="Select a [Maquinamon] from hand to link to 1 of your other Digimon.")
            elif perm:
                # Link this Digimon itself to another
                game.effect_link_to_permanent(
                    player, card, is_optional=True,
                    filter_fn=lambda p: p.is_digimon and p is not perm,
                    prompt="Select a Digimon to link this Maquinamon to.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- WhenRemoveField: Place 1 link card as bottom digi-card to prevent leaving ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.WhenRemoveField)
        effect2.set_effect_name("EX11-027 Place a linked card into digivolution cards to prevent this digimon from leaving")
        effect2.set_effect_description(
            "[All Turns] When this Digimon would leave the battle area, by placing "
            "1 of its link cards as its bottom digivolution card, it doesn't leave.")
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            leaving_perm = context.get('permanent')
            my_perm = card.permanent_of_this_card() if card else None
            if leaving_perm is not my_perm:
                return False
            if not my_perm or not my_perm.linked_cards:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Place 1 linked card as bottom digivolution card to prevent leaving."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm):
                return
            my_perm = card.permanent_of_this_card() if card else perm
            if my_perm and my_perm.linked_cards:
                linked = my_perm.linked_cards.pop(0)
                my_perm.add_card_source_bottom(linked)
                if my_perm not in player.battle_area:
                    for cs in list(my_perm.card_sources):
                        if cs in player.trash_cards:
                            player.trash_cards.remove(cs)
                    player.battle_area.append(my_perm)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
