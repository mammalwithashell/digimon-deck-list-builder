from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_072(CardScript):
    """EX10-072 Spiral Mountain"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Color bypass: "While you don't have [Spiral Mountain] in the battle
        # area, you can ignore this card's color requirements."
        # Setting unconditionally is safe: if a Spiral Mountain is already in
        # the battle area, its black color already satisfies the requirement.
        card._match_color_requirement = False

        # --- Effect 0: [Main] <Draw 2>, then place in battle area ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("EX10-072 <Draw 2>, Place in battle area")
        effect0.set_effect_description(
            "[Main] <Draw 2> (Draw 2 cards from your deck.) Then, place this "
            "card in the battle area."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Draw 2 cards. Placement handled by engine."""
            player = ctx.get('player')
            if player:
                player.draw_cards(2)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Delay marker ---
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-072 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: [End of Opponent's Turn] <Delay> ---
        # "You may play 1 face-up Digimon card with the [Dark Masters] trait
        #  from your security stack without paying the cost. At the end of your
        #  turn, delete the Digimon this effect played."
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEndTurn)
        effect2.set_effect_name("EX10-072 Delay: Play Dark Masters from security")
        effect2.set_effect_description(
            "[End of Opponent's Turn] <Delay> Play 1 face-up Digimon card with "
            "the [Dark Masters] trait from your security stack without paying "
            "the cost. At the end of your turn, delete the Digimon this effect played."
        )
        effect2.is_optional = True

        effect = effect2
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Play 1 face-up [Dark Masters] Digimon from security for free."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def sec_filter(c):
                if getattr(c, 'is_flipped', True):
                    return False
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('Dark Masters' in t for t in traits)

            has_target = any(sec_filter(c) for c in player.security_cards)
            if not has_target:
                return

            def after_selection(selected_card):
                removed = player.remove_from_security(selected_card)
                if removed:
                    played_perm = player.play_card_from_source(removed, pay_cost=False)
                    if played_perm and game:
                        game.execute_effects(EffectTiming.OnEnterFieldAnyone, {
                            'played_card': removed,
                            'played_permanent': played_perm,
                            'event_permanent': played_perm,
                            'event_player': player,
                        })
                        # "At the end of your turn, delete the Digimon this
                        #  effect played." — schedule deletion
                        _schedule_end_of_turn_delete(game, player, played_perm)

            game.effect_select_own_security(
                player, sec_filter, after_selection, is_optional=True,
                prompt="You may play 1 face-up [Dark Masters] Digimon from your security stack without paying the cost.")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: [Security] ---
        # "[Security] You may play 1 Digimon card with the [Dark Masters] trait
        #  from your hand or trash without paying the cost. Then, add this card
        #  to the hand. At turn end, delete the Digimon this effect played."
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("EX10-072 Security: Play Dark Masters from hand/trash")
        effect3.set_effect_description(
            "[Security] You may play 1 Digimon card with the [Dark Masters] "
            "trait from your hand or trash without paying the cost. Then, add "
            "this card to the hand. At turn end, delete the Digimon this effect played."
        )
        effect3.is_security_effect = True

        effect = effect3
        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Play 1 [Dark Masters] Digimon from hand or trash for free, add this to hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('Dark Masters' in t for t in traits)

            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter, free=True, is_optional=True,
                prompt="You may play 1 [Dark Masters] Digimon from your hand or trash without paying the cost.")

            # "Then, add this card to the hand."
            if card in player.trash_cards:
                player.trash_cards.remove(card)
                player.hand_cards.append(card)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects


def _schedule_end_of_turn_delete(game, player, played_perm):
    """Register an end-of-turn effect that deletes the played Digimon.

    Uses a granted effect on the permanent that fires at end of owner's turn.
    Since the engine doesn't have a formal delayed-delete registry, we add the
    effect to the permanent's granted effects list so it fires on the next
    end-of-turn for the owner.
    """
    delete_effect = ICardEffect()
    delete_effect.set_timing(EffectTiming.OnEndTurn)
    delete_effect.set_effect_name("EX10-072 Delete played Digimon at end of turn")

    def delete_condition(context):
        # Only fire on the card owner's turn end
        if not (player and player.is_my_turn):
            return False
        # Only if the permanent is still on the field
        return played_perm in player.battle_area

    delete_effect.set_can_use_condition(delete_condition)

    def delete_process(ctx):
        if played_perm in player.battle_area:
            player.delete_permanent(played_perm)

    delete_effect.set_on_process_callback(delete_process)

    # Grant the effect to the permanent so it activates at end of turn
    # expiry_turn=-1 means permanent (it self-destructs on first activation)
    played_perm._granted_effects.append((delete_effect, -1))
