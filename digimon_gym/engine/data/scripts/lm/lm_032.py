from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class LM_032(CardScript):
    """LM-032 Purple Scramble | Option Purple Cost 2
    [Main] 1 of your purple Digimon may digivolve into a purple Digimon card
    in hand with cost -3. Then, place this card in the battle area.
    [Start of Your Turn] If opponent has a Digimon, <Delay>:
    Return 1 purple Digimon from trash to top of deck. Then, if you don't have
    a Digimon, play 1 purple Digimon with 2000 DP or less from trash free.
    [Security] Play 1 purple Digimon with 2000 DP or less from trash free.
    Then, add this card to hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Main] Purple Digimon digivolve from hand cost -3, then place in battle area
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("LM-032 Main: Digivolve purple cost -3")
        effect0.set_effect_description(
            "[Main] 1 of your purple Digimon may digivolve into a purple "
            "Digimon in hand with cost -3. Then, place in battle area."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            from ....data.enums import CardColor

            # Find a purple Digimon to digivolve
            own_purple = [p for p in player.battle_area
                         if p.is_digimon and p.top_card and
                         CardColor.Purple in (getattr(p.top_card, 'card_colors', []) or [])]
            if own_purple:
                target = own_purple[0]

                def purple_digi_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    colors = getattr(c, 'card_colors', []) or []
                    return CardColor.Purple in colors

                game.effect_digivolve_from_hand(
                    player, target, purple_digi_filter,
                    cost_reduction=3, is_optional=True)

            # Place this card in battle area
            if card and player:
                player.play_card_from_source(card, pay_cost=False)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Delay marker
        effect1 = ICardEffect()
        effect1.set_effect_name("LM-032 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # <Delay> at Start of Your Turn
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnStartTurn)
        effect2.set_effect_name("LM-032 Delay: Return purple from trash + play")
        effect2.set_effect_description(
            "<Delay> Return 1 purple Digimon from trash to deck top. "
            "If no Digimon, play 1 purple 2000 DP or less from trash."
        )
        effect2._is_delay_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            # Opponent must have a Digimon
            if card and card.owner:
                enemy = card.owner.enemy
                if enemy and not any(p.is_digimon for p in enemy.battle_area):
                    return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            from ....data.enums import CardColor

            # Trash this card (Delay consumption)
            perm = card.permanent_of_this_card() if card else None
            if perm:
                player.delete_permanent(perm)

            # Return 1 purple Digimon from trash to top of deck
            for c in list(player.trash_cards):
                if getattr(c, 'is_digimon', False):
                    colors = getattr(c, 'card_colors', []) or []
                    if CardColor.Purple in colors:
                        player.trash_cards.remove(c)
                        player.library_cards.insert(0, c)
                        break

            # If no Digimon, play 1 purple 2000 DP or less from trash
            has_digimon = any(p.is_digimon for p in player.battle_area)
            if not has_digimon:
                def small_purple_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    dp = getattr(c, 'dp', None) or 0
                    if dp > 2000:
                        return False
                    colors = getattr(c, 'card_colors', []) or []
                    return CardColor.Purple in colors

                game.effect_play_from_zone(
                    player, 'trash', small_purple_filter, free=True, is_optional=True)
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # [Security] Play 1 purple 2000 DP or less from trash, add this to hand
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("LM-032 Security: Play small purple, add to hand")
        effect3.set_effect_description(
            "[Security] Play 1 purple Digimon 2000 DP or less from trash free. "
            "Then, add this card to hand."
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
            from ....data.enums import CardColor

            def small_purple_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                dp = getattr(c, 'dp', None) or 0
                if dp > 2000:
                    return False
                colors = getattr(c, 'card_colors', []) or []
                return CardColor.Purple in colors

            game.effect_play_from_zone(
                player, 'trash', small_purple_filter, free=True, is_optional=True)

            # Add this card to hand
            if card and card in player.trash_cards:
                player.trash_cards.remove(card)
                player.hand_cards.append(card)
        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
