from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_070(CardScript):
    """EX9-070 Meat | Option Cost 2 | DM
    While you have [DM] trait Digimon or Tamer, ignore color requirements.
    [Main] Draw 1. Then, place this card in the battle area.
    [Main] <Delay> By placing 1 hand card face down as [DM] Digimon's bottom
    digi card, it may digivolve into a [DM] Digimon in hand with cost -2.
    [Security] Draw 1. Then, place this card in the battle area.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        # Ignore color requirements if DM trait on field
        card._match_color_requirement = False
        effects = []

        # [Main] Draw 1, place in battle area
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("EX9-070 Draw 1, place in battle area")
        effect0.set_effect_description("[Main] Draw 1. Then, place in battle area.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not player:
                return
            player.draw_cards(1)
            if card and player:
                player.play_card_from_source(card, pay_cost=False)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Delay marker
        effect1 = ICardEffect()
        effect1.set_effect_name("EX9-070 Delay")
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

        # [Main] <Delay> Place hand card as DM digi card, digivolve into DM cost -2
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnStartMainPhase)
        effect2.set_effect_name("EX9-070 Delay: Place hand, digivolve DM cost -2")
        effect2.set_effect_description(
            "<Delay> Place 1 hand card as [DM] bottom digi card, digivolve "
            "into [DM] from hand with cost -2."
        )
        effect2._is_delay_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Trash this card from battle area (Delay consumption)
            perm = card.permanent_of_this_card() if card else None
            if perm:
                player.delete_permanent(perm)

            # Place 1 hand card as DM Digimon's bottom digi card
            if player.hand_cards:
                dm_digimon = [p for p in player.battle_area
                             if p.is_digimon and p.has_trait('DM')]
                if dm_digimon:
                    hand_card = player.hand_cards.pop()
                    target_perm = dm_digimon[0]
                    target_perm.add_card_source_bottom(hand_card)

                    # Digivolve into DM from hand with cost -2
                    def digi_filter(c):
                        if not getattr(c, 'is_digimon', False):
                            return False
                        traits = getattr(c, 'card_traits', []) or []
                        return any('DM' in t for t in traits)

                    game.effect_digivolve_from_hand(
                        player, target_perm, digi_filter,
                        cost_reduction=2, is_optional=True)
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # [Security] Draw 1, place in battle area
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("EX9-070 Security: Draw 1, place in battle area")
        effect3.set_effect_description("[Security] Draw 1. Then, place in battle area.")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not player:
                return
            player.draw_cards(1)
            if card and player:
                player.play_card_from_source(card, pay_cost=False)
        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
