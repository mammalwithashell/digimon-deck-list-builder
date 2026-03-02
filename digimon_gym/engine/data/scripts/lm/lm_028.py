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
        the hand with the digivolution cost reduced by 3. Then, place this
        card in the battle area.
    [Start of Your Turn] If your opponent has a Digimon, <Delay>
        - Return 1 blue Digimon card from your trash to the top of the deck.
          Then, if you don't have a Digimon, you may play 1 blue Digimon card
          with 2000 DP or less from your trash without paying the cost.
    [Security] You may play 1 blue Digimon card with 2000 DP or less from
        your trash without paying the cost. Then, add this card to the hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Main] Blue Digimon digivolves with cost -3 ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("LM-028 Blue Digimon digivolve cost -3")
        effect0.set_effect_description(
            "[Main] 1 of your blue Digimon may digivolve into a blue Digimon "
            "card in the hand with the digivolution cost reduced by 3."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Digivolve a blue Digimon with reduced cost."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Simplified: grant cost reduction for next digivolve
            # The engine handles digivolution cost reduction via player state
            # For now, apply a temporary cost reduction
            if hasattr(player, '_temp_digivolve_cost_reduction'):
                player._temp_digivolve_cost_reduction += 3
            else:
                player._temp_digivolve_cost_reduction = 3
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Delay ---
        effect1 = ICardEffect()
        effect1.set_effect_name("LM-028 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: Delay effect — return blue Digimon from trash to deck top ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnStartMainPhase)
        effect2.set_effect_name("LM-028 Delay: Return blue Digimon from trash")
        effect2.set_effect_description(
            "[Start of Your Turn] <Delay> Return 1 blue Digimon card from "
            "your trash to the top of the deck."
        )
        effect2._is_delay_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            player = card.owner
            enemy = player.enemy
            if not enemy:
                return False
            # Delay condition: opponent has a Digimon
            has_opp_digimon = any(p.is_digimon for p in enemy.battle_area)
            return has_opp_digimon
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Return 1 blue Digimon from trash to top of deck
            blue_digimon_in_trash = [
                c for c in player.trash_cards
                if c.is_digimon and any(
                    col.name == 'Blue' for col in (c.card_colors or [])
                )
            ]
            if blue_digimon_in_trash:
                target = blue_digimon_in_trash[0]
                player.trash_cards.remove(target)
                player.library_cards.insert(0, target)

            # Then if no Digimon, play blue Digimon DP <= 2000 from trash
            has_digimon = any(p.is_digimon for p in player.battle_area)
            if not has_digimon:
                candidates = [
                    c for c in player.trash_cards
                    if c.is_digimon
                    and any(col.name == 'Blue' for col in (c.card_colors or []))
                    and (c.dp or 0) <= 2000
                ]
                if candidates:
                    to_play = candidates[0]
                    player.trash_cards.remove(to_play)
                    game.effect_play_card_without_cost(player, to_play)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: [Security] Play blue Digimon DP <= 2000 from trash ---
        effect3 = ICardEffect()
        effect3.set_effect_name("LM-028 Security Effect")
        effect3.set_effect_description(
            "[Security] You may play 1 blue Digimon card with 2000 DP or less "
            "from your trash without paying the cost. Then, add this card to "
            "the hand."
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
            candidates = [
                c for c in player.trash_cards
                if c.is_digimon
                and any(col.name == 'Blue' for col in (c.card_colors or []))
                and (c.dp or 0) <= 2000
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
