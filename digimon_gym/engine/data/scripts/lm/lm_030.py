from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class LM_030(CardScript):
    """LM-030 Green Scramble (Option)"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Main] 1 of your green Digimon may digivolve into a green Digimon card
        # in the hand with the digivolution cost reduced by 3. Then, place this
        # card in the battle area.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("LM-030 Green Digimon digivolve from hand with cost -3")
        effect0.set_effect_description(
            "[Main] 1 of your green Digimon may digivolve into a green Digimon "
            "card in the hand with the digivolution cost reduced by 3. Then, "
            "place this card in the battle area."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def digi_filter(c):
                colors = [col.name for col in getattr(c, 'card_colors', [])]
                return 'Green' in colors and getattr(c, 'is_digimon', False)

            # Select own green Digimon to digivolve
            def perm_filter(p):
                return p.is_digimon and any(
                    'Green' in col.name
                    for col in getattr(p.top_card, 'card_colors', [])
                ) if p.top_card else False

            def on_select_perm(target_perm):
                game.effect_digivolve_from_hand(
                    player, target_perm, digi_filter,
                    cost_reduction=3, is_optional=True)

            game.effect_select_own_permanent(
                player, on_select_perm, filter_fn=perm_filter, is_optional=True)

            # Then, place this card in the battle area (handled by Delay factory)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # <Delay> factory marker — card stays in battle area after [Main] resolves
        effect1 = ICardEffect()
        effect1.set_effect_name("LM-030 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition_delay(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition_delay)
        effects.append(effect1)

        # [Start of Your Turn] If your opponent has a Digimon, <Delay>
        # Return 1 green Digimon card from your trash to the top of the deck.
        # Then, if you don't have a Digimon, you may play 1 green Digimon card
        # with 2000 DP or less from your trash without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDeclaration)
        effect2.set_effect_name("LM-030 Delay: Return green Digimon from trash to deck top")
        effect2.set_effect_description(
            "[Start of Your Turn] If your opponent has a Digimon, return 1 "
            "green Digimon card from your trash to the top of the deck. Then, "
            "if you don't have a Digimon, you may play 1 green Digimon card "
            "with 2000 DP or less from your trash without paying the cost."
        )

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only activate if opponent has a Digimon
            if card and card.owner:
                enemy = card.owner.enemy
                if enemy:
                    return any(p.is_digimon for p in enemy.battle_area)
            return False

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Return 1 green Digimon card from trash to top of deck
            def return_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = [col.name for col in getattr(c, 'card_colors', [])]
                return 'Green' in colors

            game.effect_return_from_trash_to_deck_top(
                player, return_filter, is_optional=False)

            # Then, if you don't have a Digimon, play green DP<=2000 from trash
            has_digimon = any(p.is_digimon for p in player.battle_area)
            if not has_digimon:
                def play_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    dp = getattr(c, 'dp', 0) or 0
                    if dp > 2000:
                        return False
                    colors = [col.name for col in getattr(c, 'card_colors', [])]
                    return 'Green' in colors

                game.effect_play_from_zone(
                    player, 'trash', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Security Effect [Security] You may play 1 green Digimon card with
        # 2000 DP or less from your trash without paying the cost. Then, add
        # this card to the hand.
        effect3 = ICardEffect()
        effect3.set_effect_name("LM-030 Security: Play green Digimon 2000 DP or less from trash")
        effect3.set_effect_description(
            "Security: You may play 1 green Digimon card with 2000 DP or less "
            "from your trash without paying the cost. Then, add this card to "
            "the hand."
        )
        effect3.is_security_effect = True
        effect3.is_optional = True

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
                dp = getattr(c, 'dp', 0) or 0
                if dp > 2000:
                    return False
                colors = [col.name for col in getattr(c, 'card_colors', [])]
                return 'Green' in colors

            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

            # Then, add this card to the hand
            if card:
                game.effect_add_card_to_hand(player, card)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
