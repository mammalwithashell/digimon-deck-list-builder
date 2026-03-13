from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_058(CardScript):
    """BT21-058 Snatchmon | Lv.4

    [On Play][When Digivolving] Reveal the top 3 cards of your deck. Add 1
    card with [Vemmon] in its text among them to the hand. Place the rest at
    the bottom of the deck in any order. Then, you may place 1 [Vemmon] from
    your trash as this Digimon's bottom digivolution card.

    --- Inherited ---
    [All Turns] [Once Per Turn] When any [Vemmon] are returned to the bottom
    of the deck from this Digimon's digivolution cards, delete 1 of your
    opponent's Digimon with a play cost of 4 or less.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _has_vemmon_text(c) -> bool:
            return 'Vemmon' in getattr(c, 'card_text', '')

        def _is_vemmon_name(c) -> bool:
            return any('Vemmon' in n for n in getattr(c, 'card_names', []))

        def _make_on_enter_effect(is_when_digivolving: bool) -> ICardEffect:
            """Factory for On Play / When Digivolving shared effect."""
            effect = ICardEffect()
            effect.set_timing(EffectTiming.OnEnterFieldAnyone)
            effect.set_effect_name("BT21-058 Reveal top 3, add Vemmon-text to hand, place Vemmon from trash")
            if is_when_digivolving:
                effect.is_when_digivolving = True
            else:
                effect.is_on_play = True
            effect.set_effect_description(
                "[On Play][When Digivolving] Reveal the top 3 cards of your deck. Add 1 "
                "card with [Vemmon] in its text among them to the hand. Place the rest at "
                "the bottom of the deck. Then, you may place 1 [Vemmon] from your trash "
                "as this Digimon's bottom digivolution card.")

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                return True

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                perm = ctx.get('permanent')
                game = ctx.get('game')
                if not (player and game):
                    return

                # Step 1: Reveal top 3 cards
                revealed = []
                for _ in range(min(3, len(player.library_cards))):
                    revealed.append(player.library_cards.pop(0))
                if not revealed:
                    return

                # Step 2: Add 1 card with Vemmon text to hand, rest to deck bottom
                def reveal_filter(c):
                    return _has_vemmon_text(c)

                def on_revealed(selected, remaining):
                    player.hand_cards.append(selected)
                    for c in remaining:
                        player.library_cards.append(c)
                    # Step 3: Place 1 [Vemmon] from trash as bottom digi-card
                    _place_vemmon_from_trash(player, perm, game)

                def on_no_select():
                    # No Vemmon-text card found; all go to deck bottom
                    for c in revealed:
                        player.library_cards.append(c)
                    _place_vemmon_from_trash(player, perm, game)

                vemmon_text_cards = [c for c in revealed if _has_vemmon_text(c)]
                if vemmon_text_cards:
                    game.effect_reveal_and_select(
                        player, 3, reveal_filter, on_revealed, is_optional=True)
                else:
                    on_no_select()

            def _place_vemmon_from_trash(player, perm, game):
                """Place 1 [Vemmon] from trash as this Digimon's bottom digi-card."""
                if perm is None:
                    return
                qualifying = [c for c in player.trash_cards if _is_vemmon_name(c)]
                if not qualifying:
                    return
                chosen = qualifying[0]
                player.trash_cards.remove(chosen)
                perm.add_card_source_bottom(chosen)

            effect.set_on_process_callback(process)
            return effect

        effects.append(_make_on_enter_effect(is_when_digivolving=False))
        effects.append(_make_on_enter_effect(is_when_digivolving=True))

        # ─── Inherited: [All Turns] [Once Per Turn] When any [Vemmon] are returned
        #     to deck bottom from this Digimon's digi-cards, delete 1 opponent Digimon
        #     with play cost 4 or less.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDigivolutionCardReturnToDeckBottom)
        effect2.set_effect_name("BT21-058 Delete 1 opponent Digimon with play cost 4 or less")
        effect2.set_effect_description("[All Turns] [Once Per Turn] When any [Vemmon] are returned to the bottom of the deck from this Digimon's digivolution cards, delete 1 of your opponent's Digimon with a play cost of 4 or less.")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Delete_BT21_058")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # The returned card must be [Vemmon]
            returned_card = context.get('returned_card')
            if returned_card is None:
                return False
            if not _is_vemmon_name(returned_card):
                return False
            # Must be from THIS Digimon's digi-cards
            ctx_perm = context.get('permanent')
            my_perm = card.permanent_of_this_card()
            if ctx_perm is not None and my_perm is not None and ctx_perm is not my_perm:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Delete 1 opponent Digimon with play cost <= 4."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                if not p.is_digimon:
                    return False
                top = p.top_card
                if top is None:
                    return False
                cost = getattr(top, 'get_cost_itself', 99)
                return cost <= 4

            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
