from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class LM_031(CardScript):
    """LM-031 Black Scramble | Option (Black, Cost 2)

    [Main] 1 of your black Digimon may digivolve into a black Digimon card in
        the hand with the digivolution cost reduced by 3. Then, place this
        card in the battle area.
    [Start of Your Turn] If your opponent has a Digimon, <Delay>
        - Return 1 black Digimon card from your trash to the top of the deck.
          Then, if you don't have a Digimon, you may play 1 black Digimon card
          with 2000 DP or less from your trash without paying the cost.
    [Security] You may play 1 black Digimon card with 2000 DP or less from
        your trash without paying the cost. Then, add this card to the hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Main] Black Digimon digivolves with cost -3 ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("LM-031 Digivolve 1 black Digimon with cost reduced by 3")
        effect0.set_effect_description(
            "[Main] 1 of your black Digimon may digivolve into a black "
            "Digimon card in the hand with the digivolution cost reduced by 3. "
            "Then, place this card in the battle area."
        )
        effect0.cost_reduction = 3

        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Filter for black Digimon cards in hand
            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not any(col.name == 'Black' for col in getattr(c, 'card_colors', [])):
                    return False
                return True
            # Select one of your black Digimon on the field to digivolve
            def own_filter(p):
                if not p.is_digimon:
                    return False
                top = p.top_card
                if top and hasattr(top, 'card_colors'):
                    if not any(col.name == 'Black' for col in top.card_colors):
                        return False
                return True
            def on_select(target_perm):
                if game:
                    game.effect_digivolve_from_hand(
                        player, target_perm, digi_filter, is_optional=True)
            game.effect_select_own_permanent(
                player, on_select, filter_fn=own_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Delay marker ---
        effect1 = ICardEffect()
        effect1.set_effect_name("LM-031 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            # [Start of Your Turn] If your opponent has a Digimon
            enemy = card.owner.enemy if card and card.owner else None
            if not enemy:
                return False
            has_opp_digimon = any(
                p.is_digimon for p in (enemy.battle_area or [])
            )
            if not has_opp_digimon:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: Delay effect — return black Digimon from trash, play ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnStartTurn)
        effect2.set_effect_name(
            "LM-031 Return 1 black Digimon from trash to deck top, then play from trash"
        )
        effect2.set_effect_description(
            "Return 1 black Digimon card from your trash to the top of the "
            "deck. Then, if you don't have a Digimon, you may play 1 black "
            "Digimon card with 2000 DP or less from your trash without paying "
            "the cost."
        )
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Return to deck top, Play Card"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Return 1 black Digimon from trash to top of deck
            for i, c in enumerate(player.trash_cards):
                if getattr(c, 'is_digimon', False):
                    colors = getattr(c, 'card_colors', [])
                    if any(col.name == 'Black' for col in colors):
                        returned = player.trash_cards.pop(i)
                        player.deck_cards.insert(0, returned)
                        break
            # Then, if you don't have a Digimon, play 1 black Digimon
            # with 2000 DP or less from trash without paying cost
            has_digimon = any(
                p.is_digimon for p in (player.battle_area or [])
            )
            if not has_digimon:
                def play_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    if not any(
                        col.name == 'Black'
                        for col in getattr(c, 'card_colors', [])
                    ):
                        return False
                    if getattr(c, 'dp', None) is not None and c.dp > 2000:
                        return False
                    return True
                game.effect_play_from_zone(
                    player, 'trash', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: [Security] Play black Digimon DP <= 2000 from trash ---
        effect3 = ICardEffect()
        effect3.set_effect_name("LM-031 Security Effect")
        effect3.set_effect_description(
            "[Security] You may play 1 black Digimon card with 2000 DP or "
            "less from your trash without paying the cost. Then, add this "
            "card to the hand."
        )
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Security — play from trash, add to hand"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            candidates = [
                c for c in player.trash_cards
                if getattr(c, 'is_digimon', False)
                and any(
                    col.name == 'Black'
                    for col in getattr(c, 'card_colors', [])
                )
                and (getattr(c, 'dp', None) is None or c.dp <= 2000)
            ]
            if candidates:
                to_play = candidates[0]
                player.trash_cards.remove(to_play)
                game.effect_play_card_without_cost(player, to_play)
            # Add this card to hand
            if card:
                player.hand_cards.append(card)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
