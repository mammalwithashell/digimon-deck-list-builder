from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT17_100(CardScript):
    """BT17-100 Doomsday Clock | Option (White, Cost 3)

    [Security] Add this card to the hand.
    [Main] Play 1 [Diaboromon] Token without paying the cost. Then, place this
        card as the bottom digivolution card of 1 of your [Diaboromon] without
        [Doomsday Clock] in its digivolution cards.
    [Start of Your Turn] If 4 [Doomsday Clock]s are placed in your battle area,
        you win the game.
    Inherited:
        [All Turns] When this Digimon would leave the battle area by an
            opponent's effect, by deleting 1 of your other [Diaboromon],
            prevent it from leaving.
        [End of Opponent's Turn] Place 1 [Doomsday Clock] from this Digimon's
            digivolution cards in the battle area.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Main] Play 1 Diaboromon Token, then place this card
        #     as bottom digi-card of a Diaboromon without Doomsday Clock ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT17-100 Play token, place under Diaboromon")
        effect0.set_effect_description(
            "[Main] Play 1 [Diaboromon] Token without paying the cost. Then, "
            "place this card as the bottom digivolution card of 1 of your "
            "[Diaboromon] without [Doomsday Clock] in its digivolution cards."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Play 1 Diaboromon Token
            game.effect_play_token(player, 'diaboromon')

            # Place this card as bottom digivolution card of a Diaboromon
            # without Doomsday Clock in its digi-cards.
            # After option resolves, the card goes to trash. We need to
            # intercept and place it under a Diaboromon instead.
            def diaboromon_filter(p):
                if not p.contains_card_name('Diaboromon'):
                    return False
                # Check no Doomsday Clock in digivolution cards
                for cs in p.card_sources:
                    names = getattr(cs, 'card_names', []) or []
                    if any('Doomsday Clock' in n for n in names):
                        return False
                return True

            def on_select(target_perm):
                # Find this card in trash (option cards go to trash after use)
                dc_card_in_trash = None
                for tc in player.trash_cards:
                    if tc is card or (hasattr(tc, 'card_id') and tc.card_id == 'BT17-100'
                                     and any('Doomsday Clock' in n for n in getattr(tc, 'card_names', []))):
                        dc_card_in_trash = tc
                        break
                if dc_card_in_trash:
                    player.trash_cards.remove(dc_card_in_trash)
                    target_perm.add_card_source_bottom(dc_card_in_trash)
                    game.logger.log(
                        "[BT17-100] Placed Doomsday Clock as bottom digivolution "
                        "card of Diaboromon."
                    )

            has_target = any(diaboromon_filter(p) for p in player.battle_area)
            if has_target:
                game.effect_select_own_permanent(
                    player, on_select, filter_fn=diaboromon_filter,
                    is_optional=False,
                    prompt="Select a [Diaboromon] without [Doomsday Clock] in its digi-cards."
                )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Security] Add this card to the hand ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("BT17-100 Security: Add to hand")
        effect1.set_effect_description("[Security] Add this card to the hand.")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player and card:
                # Move from trash to hand
                if card in player.trash_cards:
                    player.trash_cards.remove(card)
                player.hand_cards.append(card)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Start of Your Turn] Win condition ---
        # If 4 Doomsday Clocks are placed in your battle area, you win.
        # This is an inherited effect that fires as an option placed in battle area.
        # NOTE: Doomsday Clock as an option card in the battle area is a special
        # mechanic. The win condition check is declarative.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnStartTurn)
        effect2.set_effect_name("BT17-100 Doomsday Clock win condition")
        effect2.set_effect_description(
            "[Start of Your Turn] If 4 [Doomsday Clock]s are placed in your "
            "battle area, you win the game."
        )

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not (owner and owner.is_my_turn):
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Count Doomsday Clocks on our field
            # They could be option permanents or under Digimon digivolution cards
            # that have been placed in the battle area.
            dc_count = 0
            for perm in player.battle_area:
                if perm.top_card:
                    names = getattr(perm.top_card, 'card_names', []) or []
                    if any('Doomsday Clock' in n for n in names):
                        dc_count += 1
            if dc_count >= 4:
                game.logger.log("[BT17-100] 4 Doomsday Clocks in battle area — YOU WIN!")
                game.declare_winner(player)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: Inherited [All Turns] Prevent leaving by opponent effect ---
        # by deleting 1 of your other [Diaboromon]
        # Uses WhenPermanentWouldBeDeleted timing + _will_not_be_removed flag
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.WhenPermanentWouldBeDeleted)
        effect3.set_effect_name("BT17-100 Inherited: Prevent leaving by deleting Diaboromon")
        effect3.set_effect_description(
            "[All Turns] When this Digimon would leave the battle area by an "
            "opponent's effect, by deleting 1 of your other [Diaboromon], "
            "prevent it from leaving."
        )
        effect3.is_inherited_effect = True
        effect3.is_optional = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            own_perm = card.permanent_of_this_card()
            # Only triggers for THIS permanent being deleted
            ctx_perm = context.get('permanent')
            if ctx_perm is not own_perm:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            # Must have another Diaboromon to delete
            return any(
                p is not own_perm and p.is_digimon and p.contains_card_name('Diaboromon')
                for p in owner.battle_area
            )
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            own_perm = card.permanent_of_this_card() if card else None
            if not own_perm:
                return

            def diaboromon_filter(p):
                return (p is not own_perm and p.is_digimon
                        and p.contains_card_name('Diaboromon'))

            def on_diaboromon_selected(target_perm):
                player.delete_permanent(target_perm)
                own_perm._will_not_be_removed = True
                game.logger.log(
                    "[BT17-100] Deleted another Diaboromon to prevent "
                    "this Digimon from leaving the battle area."
                )

            game.effect_select_own_permanent(
                player, on_diaboromon_selected, filter_fn=diaboromon_filter,
                is_optional=False,
                prompt="Select 1 of your other [Diaboromon] to delete (prevents this Digimon from leaving)."
            )

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # --- Effect 4: Inherited [End of Opponent's Turn] Place Doomsday Clock ---
        # from digi-cards to battle area
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEndTurn)
        effect4.set_effect_name("BT17-100 Inherited: Place Doomsday Clock from digi-cards")
        effect4.set_effect_description(
            "[End of Opponent's Turn] Place 1 [Doomsday Clock] from this "
            "Digimon's digivolution cards in the battle area."
        )
        effect4.is_inherited_effect = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            # Must be opponent's turn (end of opponent's turn)
            if owner.is_my_turn:
                return False
            # Must have a Doomsday Clock in digivolution cards
            perm = card.permanent_of_this_card()
            if perm:
                for cs in perm.card_sources:
                    if cs is perm.top_card:
                        continue
                    names = getattr(cs, 'card_names', []) or []
                    if any('Doomsday Clock' in n for n in names):
                        return True
            return False
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return
            # Find a Doomsday Clock in digivolution cards
            for cs in list(perm.card_sources):
                if cs is perm.top_card:
                    continue
                names = getattr(cs, 'card_names', []) or []
                if any('Doomsday Clock' in n for n in names):
                    perm.card_sources.remove(cs)
                    # Play as a permanent in the battle area
                    played = player.play_card_from_source(cs, pay_cost=False)
                    game.logger.log(
                        "[BT17-100] Placed Doomsday Clock from digivolution "
                        "cards into the battle area."
                    )
                    break

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
