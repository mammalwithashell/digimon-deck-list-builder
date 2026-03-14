from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, GamePhase

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_068(CardScript):
    """EX10-068 Digimon Emperor | Tamer | White | Cost 5

    This card is also treated as [Ken Ichijoji].
    [Start of Your Main Phase] For every 2 colors your opponent's Digimon
        and Tamers have, gain 1 memory.
    [On Play] Delete 1 of your opponent's play cost 5 or lower Digimon.
        Then, by returning 1 Digimon card from your opponent's trash to the
        bottom of the deck, from your hand or trash and without paying the
        cost, you may play 1 level 4 or lower Digimon card with the same
        color as the card this effect returned.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []
        card.also_treated_as_names = ['Ken Ichijoji']

        # --- Effect 0: Also treated as [Ken Ichijoji] (descriptive) ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-068 Also treated as [Ken Ichijoji]")
        effect0.set_effect_description("Also Treated As")

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            pass  # descriptive-tagged: also_treated_as_name
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Start of Your Main Phase] Gain memory for opponent's colors ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnStartMainPhase)
        effect1.set_effect_name("EX10-068 Gain Memory")
        effect1.set_effect_description(
            "[Start of Your Main Phase] For every 2 colors your opponent's "
            "Digimon and Tamers have, gain 1 memory."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Count unique colors across opponent's Digimon and Tamers, gain 1 memory per 2."""
            player = ctx.get('player')
            if not player:
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            unique_colors = set()
            for p in enemy.battle_area:
                if not (p.is_digimon or p.is_tamer):
                    continue
                if p.top_card:
                    colors = getattr(p.top_card, 'card_colors', []) or []
                    for c in colors:
                        unique_colors.add(c)
            memory_gain = len(unique_colors) // 2
            if memory_gain > 0:
                player.add_memory(memory_gain)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [On Play] Delete, return trash to deck, play matching color ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX10-068 Delete 1 Digimon, Return 1 Digimon, Play 1 Digimon")
        effect2.set_effect_description(
            "[On Play] Delete 1 of your opponent's play cost 5 or lower Digimon. "
            "Then, by returning 1 Digimon card from your opponent's trash to the "
            "bottom of the deck, from your hand or trash and without paying the "
            "cost, you may play 1 level 4 or lower Digimon card with the same "
            "color as the card this effect returned."
        )
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Delete cost<=5 opp Digimon. Then return opp trash Digimon to deck bottom, play matching color Lv4-."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            # Step 1: Delete 1 of opponent's play cost 5 or lower Digimon
            def delete_filter(p):
                if not p.is_digimon:
                    return False
                if not p.top_card:
                    return False
                cost = p.top_card.get_cost_itself
                return cost <= 5

            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)

            if any(delete_filter(p) for p in enemy.battle_area):
                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=delete_filter, is_optional=False)

            # Step 2: Optionally return 1 Digimon card from opponent's trash to deck bottom
            # Then play a Lv4- Digimon matching the returned card's color
            opp_trash_digimon = [c for c in enemy.trash_cards if getattr(c, 'is_digimon', False)]
            if not opp_trash_digimon:
                return

            def on_trash_select(selected_trash_card):
                """Return selected card to deck bottom, then play matching color Digimon."""
                returned_colors = set(getattr(selected_trash_card, 'card_colors', []) or [])
                # Return it to deck bottom
                if selected_trash_card in enemy.trash_cards:
                    enemy.trash_cards.remove(selected_trash_card)
                    enemy.library_cards.append(selected_trash_card)

                # Now play 1 Lv4- Digimon with same color from hand or trash
                def play_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    level = getattr(c, 'level', None)
                    if level is None or level > 4:
                        return False
                    card_colors = set(getattr(c, 'card_colors', []) or [])
                    # Must share at least one color with the returned card
                    if not card_colors.intersection(returned_colors):
                        return False
                    return True

                game.effect_play_from_zone(
                    player, 'hand_or_trash', play_filter, free=True, is_optional=True)

            # Select from opponent's trash using SelectTrash phase.
            # The "by returning" clause makes the play conditional on choosing to return.
            from ....game.constants import SEL_TRASH_START as _SEL_TRASH_START
            valid_trash = []
            for i, c in enumerate(enemy.trash_cards):
                if getattr(c, 'is_digimon', False):
                    valid_trash.append(_SEL_TRASH_START + i)

            if valid_trash:
                def on_opp_trash_action(action_id):
                    idx = action_id - _SEL_TRASH_START
                    if idx < len(enemy.trash_cards):
                        on_trash_select(enemy.trash_cards[idx])

                game.request_selection(
                    GamePhase.SelectTrash, player, on_opp_trash_action,
                    valid_trash, is_optional=True,
                    prompt="Select 1 Digimon from opponent's trash to return to deck bottom.")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: Security: Play this card ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("EX10-068 Security: Play this card")
        effect3.set_effect_description("Security: Play this card")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
