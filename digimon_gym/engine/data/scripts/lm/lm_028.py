from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class LM_028(CardScript):
    """LM-028 Blue Scramble | Option (Blue, Cost 2)

    [Main] 1 of your blue Digimon may digivolve into a blue Digimon card in
    the hand with the digivolution cost reduced by 3. Then, place this card
    in the battle area.
    [Start of Your Turn] If your opponent has more Digimon than you, by
    trashing this card from the battle area, return 1 of your opponent's
    Digimon to the bottom of the deck.
    [Security] You may play 1 blue Digimon card with 2000 DP or less from
    your trash without paying the cost. Then, add this card to the hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Main] Blue Digimon digivolves with cost -3, then place in battle area ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("LM-028 Blue Digimon digivolve cost -3, then place in battle area")
        effect0.set_effect_description(
            "[Main] 1 of your blue Digimon may digivolve into a blue Digimon card in "
            "the hand with the digivolution cost reduced by 3. Then, place this card "
            "in the battle area."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Filter: own blue Digimon permanents that can digivolve into a blue hand card
            def own_perm_filter(p):
                if not p.is_digimon:
                    return False
                top = p.top_card
                if not top:
                    return False
                colors = [c.name for c in (getattr(top, 'card_colors', None) or [])]
                return 'Blue' in colors

            def hand_card_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = [col.name for col in (getattr(c, 'card_colors', None) or [])]
                return 'Blue' in colors

            # Select 1 of your blue Digimon to digivolve, then digivolve with cost -3
            def on_select_perm(selected_perm):
                game.effect_digivolve_from_hand(
                    player, selected_perm,
                    filter_fn=hand_card_filter,
                    cost_reduction=3,
                    is_optional=True
                )

            game.effect_select_own_permanent(
                player, on_select_perm,
                filter_fn=own_perm_filter,
                is_optional=True
            )

            # Then place this card in the battle area (Delay placement)
            if card and player:
                player.play_card_from_source(card, pay_cost=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Delay keyword factory ---
        effect1 = ICardEffect()
        effect1.set_effect_name("LM-028 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: [Start of Your Turn] Delay effect —
        #    If opponent has a Digimon, trash this card from battle area,
        #    return 1 opponent's Digimon to bottom of deck ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnStartTurn)
        effect2.set_effect_name("LM-028 Delay: Return opponent Digimon to deck bottom")
        effect2.set_effect_description(
            "[Start of Your Turn] If your opponent has more Digimon than you, by trashing "
            "this card from the battle area, return 1 of your opponent's Digimon to the "
            "bottom of the deck."
        )
        effect2._is_delay_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            owner = card.owner if card else None
            if not owner:
                return False
            if not owner.is_my_turn:
                return False
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            enemy = owner.enemy
            if not enemy:
                return False
            # Opponent has at least 1 Digimon
            return any(p.is_digimon for p in enemy.battle_area)

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            # Trash this card from the battle area first
            perm = card.permanent_of_this_card() if card else None
            if perm:
                player.delete_permanent(perm)

            # Then select 1 opponent Digimon to return to bottom of deck
            def target_filter(p):
                return p.is_digimon

            def on_select(target_perm):
                enemy.return_permanent_to_deck_bottom(target_perm)

            game.effect_select_opponent_permanent(
                player, on_select, filter_fn=target_filter, is_optional=False
            )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: [Security] Play blue Digimon DP <= 2000 from trash.
        #    Then add this card to hand. ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("LM-028 Security: Play blue Digimon DP<=2000 from trash")
        effect3.set_effect_description(
            "[Security] You may play 1 blue Digimon card with 2000 DP or less from your "
            "trash without paying the cost. Then, add this card to the hand."
        )
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = [col.name for col in (getattr(c, 'card_colors', None) or [])]
                if 'Blue' not in colors:
                    return False
                dp = getattr(c, 'base_dp', None) or getattr(c, 'dp', None) or 0
                return dp <= 2000

            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True
            )

            # Then add this card to hand
            if card:
                player.hand_cards.append(card)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
