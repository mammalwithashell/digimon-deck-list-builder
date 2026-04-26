from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_055(CardScript):
    """BT20-055 Invisimon | Lv.6 Black Digimon"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Security][End of Opponent's Turn] Play this card without paying cost ---
        # Modeled as a security effect with OnEndTurn timing — fires at end of opponent's turn
        # when this card is in security, then plays it for free.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEndTurn)
        effect0.set_effect_name("BT20-055 Security: play without cost at end of opponent's turn")
        effect0.set_effect_description(
            "[Security][End of Opponent's Turn] Play this card without paying the cost."
        )
        effect0.is_security_effect = True

        def condition0(context: Dict[str, Any]) -> bool:
            owner = card.owner if card else None
            if not owner:
                return False
            # Must fire at end of opponent's turn only
            if owner.is_my_turn:
                return False
            # Must be in owner's security stack to trigger
            if card not in owner.security_cards:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Play this card from security without paying cost
            # Use game.effect_play_from_security so On Play effects fire
            if card and card in player.security_cards:
                player.security_cards.remove(card)
                game.effect_play_from_security(player, card)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effects 1+2: [On Play][When Digivolving]
        #    <De-Digivolve 2> 1 opponent Digimon.
        #    Flip opponent's top face-down security card face up.
        #    Delete 1 opponent Digimon with 1 or fewer digivolution cards. ---
        def make_enter_field_effect(is_digivolving: bool):
            eff = ICardEffect()
            eff.set_timing(EffectTiming.OnEnterFieldAnyone)
            eff.set_effect_name(
                "BT20-055 De-Digivolve 2 opponent, flip top security face-up, "
                "delete Digimon with <=1 digi-cards"
            )
            eff.set_effect_description(
                "[On Play][When Digivolving] <De-Digivolve 2> 1 of your opponent's Digimon. "
                "Then, flip your opponent's top face-down security card face up. "
                "Then, delete 1 of your opponent's Digimon with 1 or fewer digivolution cards."
            )
            if is_digivolving:
                eff.is_when_digivolving = True
            else:
                eff.is_on_play = True

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
                enemy = player.enemy
                if not enemy:
                    return

                # Step 1: <De-Digivolve 2> 1 opponent Digimon
                def on_de_digivolve(target_perm):
                    removed = target_perm.de_digivolve(2)
                    enemy.trash_cards.extend(removed)

                game.effect_select_opponent_permanent(
                    player, on_de_digivolve,
                    filter_fn=lambda p: p.is_digimon, is_optional=False
                )

                # Step 2: Flip opponent's top face-down security card face up
                # Security stack: index 0 = bottom, last index = top
                for sec_card in reversed(enemy.security_cards):
                    if not enemy.is_security_face_up(sec_card):
                        enemy.face_up_security.add(sec_card)
                        break

                # Step 3: Delete 1 opponent Digimon with 1 or fewer digivolution cards
                # "digivolution cards" = cards under the top card (len(card_sources) - 1)
                def delete_filter(p):
                    if not p.is_digimon:
                        return False
                    digi_card_count = len(getattr(p, 'card_sources', [])) - 1
                    return digi_card_count <= 1

                def on_delete(target_perm):
                    enemy.delete_permanent(target_perm)

                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=delete_filter, is_optional=False
                )

            eff.set_on_process_callback(process)
            return eff

        effects.append(make_enter_field_effect(is_digivolving=False))
        effects.append(make_enter_field_effect(is_digivolving=True))

        # --- Effect 3: [Your Turn] When your Digimon checks a face-up security card,
        #    you may place the top card of this Digimon face-up at the bottom of
        #    your security stack. ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnSecurityCheck)
        effect3.set_effect_name(
            "BT20-055 On face-up security check: place top digi-card face-up to own security bottom"
        )
        effect3.set_effect_description(
            "[Your Turn] When your Digimon checks a face-up security card, you may place "
            "the top card of this Digimon face-up at the bottom of your security stack."
        )
        effect3.is_optional = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # The security check event must be triggered by our player's Digimon
            event_player = context.get('event_player')
            if event_player is None or event_player is not card.owner:
                return False
            # The checked security card must have been face-up before it was checked
            # (security_was_face_up is set by combat.py when passing OnSecurityCheck context)
            if not context.get('security_was_face_up', False):
                return False
            # This permanent must have at least 1 digivolution card to give away
            perm = card.permanent_of_this_card()
            if perm is None or perm.has_no_digivolution_cards:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if not (player and perm):
                return
            if perm.has_no_digivolution_cards:
                return
            # Trash the top digivolution card (topmost under-card), then move it
            # from trash to security face-up at the bottom
            trashed = perm.trash_digivolution_cards(1, from_top=True)
            if trashed:
                top_digi_card = trashed[0]
                if top_digi_card in player.trash_cards:
                    player.trash_cards.remove(top_digi_card)
                player.add_to_security_face_up(top_digi_card, to_top=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
