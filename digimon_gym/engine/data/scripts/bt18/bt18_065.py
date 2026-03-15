from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT18_065(CardScript):
    """BT18-065 Snatchmon | Lv.4 | Black | DP 6000 | Cost 6

    DigiXros -1: 4 [Vemmon]. When this would be played, you may place specified
    cards from your hand/battle area under it. Each reduces the play cost.

    While you have no Digimon other than [Vemmon], cards from your trash may
    also be placed for this card's DigiXros.

    [When Digivolving] You may place up to 2 [Vemmon] from your trash as this
    Digimon's bottom digivolution cards.

    [End of Your Turn] If this Digimon has 4 or more digivolution cards, this
    Digimon may digivolve into a Digimon card with [Vemmon] in its text in
    your hand.

    --- Inherited ---
    [All Turns] [Once Per Turn] When [Vemmon] returns to the bottom of the
    deck from this Digimon's digivolution cards, unsuspend this Digimon and it
    gains Blocker until the end of your opponent's turn.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _is_vemmon_name(c) -> bool:
            return any('Vemmon' in n for n in getattr(c, 'card_names', []))

        def _has_vemmon_text(c) -> bool:
            return 'Vemmon' in getattr(c, 'card_text', '')

        def _only_vemmon_digimon(player) -> bool:
            """Returns True if player has no Digimon other than [Vemmon] on field."""
            for p in player.battle_area:
                if p.is_digimon and not p.contains_card_name('Vemmon'):
                    return False
            return True

        # DigiXros -1: 4 [Vemmon] cost reduction is now handled by the engine's
        # DigiXrosCost system (parsed from xros_req metadata). The engine presents
        # material selection via SelectMaterial phase and applies cost reduction
        # automatically. The "trash zone if only Vemmon on field" extension is a
        # TODO for a future script-level zone override hook.

        # ─── Effect 1: [When Digivolving] Place up to 2 [Vemmon] from trash as
        #     bottom digi-cards.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT18-065 When Digivolving: place up to 2 Vemmon from trash")
        effect1.set_effect_description(
            "[When Digivolving] You may place up to 2 [Vemmon] from your trash as "
            "this Digimon's bottom digivolution cards.")
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return

            # Auto-place up to 2 [Vemmon] from trash as bottom digi-cards
            placed = 0
            for c in list(player.trash_cards):
                if placed >= 2:
                    break
                if _is_vemmon_name(c):
                    player.trash_cards.remove(c)
                    perm.add_card_source_bottom(c)
                    placed += 1

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ─── Effect 2: [End of Your Turn] If 4+ digi-cards (5+ in card_sources),
        #     may digivolve into a Vemmon-text card from hand.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEndTurn)
        effect2.set_effect_name("BT18-065 End of Turn: Digivolve into Vemmon-text from hand")
        effect2.set_effect_description(
            "[End of Your Turn] If this Digimon has 4 or more digivolution cards, "
            "this Digimon may digivolve into a Digimon card with [Vemmon] in its "
            "text in your hand.")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Must be your turn
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            perm = card.permanent_of_this_card()
            if perm is None:
                return False
            # 4+ digivolution cards means 5+ total in card_sources (top card + 4 digi-cards)
            if len(perm.card_sources) < 5:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return

            def digi_filter(c):
                return getattr(c, 'is_digimon', False) and _has_vemmon_text(c)

            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True,
                prompt="Select a Digimon card with [Vemmon] in its text from your hand to digivolve into.")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # ─── Effect 3 (Inherited): [All Turns] [Once Per Turn] When [Vemmon]
        #     returns to deck bottom from this Digimon's digi-cards, unsuspend
        #     this Digimon and grant Blocker until end of opponent's turn.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDigivolutionCardReturnToDeckBottom)
        effect3.set_effect_name("BT18-065 Inherited: Unsuspend + Blocker on Vemmon return")
        effect3.set_effect_description(
            "[All Turns] [Once Per Turn] When [Vemmon] returns to the bottom of the "
            "deck from this Digimon's digivolution cards, unsuspend this Digimon and "
            "it gains Blocker until the end of your opponent's turn.")
        effect3.is_inherited_effect = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("Unsuspend_Blocker_BT18_065")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # The returned card must be [Vemmon]
            returned_card = context.get('returned_card')
            if returned_card is None:
                return False
            if not _is_vemmon_name(returned_card):
                return False
            # The permanent in context must be THIS Digimon
            ctx_perm = context.get('permanent')
            my_perm = card.permanent_of_this_card()
            if ctx_perm is not None and my_perm is not None and ctx_perm is not my_perm:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            # Unsuspend THIS Digimon (the one containing this inherited card)
            my_perm = card.permanent_of_this_card()
            if my_perm:
                my_perm.unsuspend()
                # Grant Blocker until end of opponent's turn
                from ....interfaces.modifiers import ModifierType
                game.register_modifier(
                    my_perm, ModifierType.GRANT_BLOCKER,
                    value_fn=lambda: True, expiry='end_of_opponent_turn')

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
