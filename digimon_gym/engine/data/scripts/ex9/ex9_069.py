from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_069(CardScript):
    """EX9-069 Analog Youth | Tamer Cost 3 | DM
    [Start of Your Main Phase] Place 1 hand card face down as any [DM] Digimon's
    bottom digi card.
    [Your Turn] When face-down cards placed as digi cards, suspend to gain 1
    memory. Then, if 7 or fewer cards in hand, Draw 1.
    [Opponent's Turn] All Digimon with face-down digi cards gain Reboot.
    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Start of Your Main Phase] Place hand card as DM Digimon's bottom digi
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("EX9-069 Start of Main: Place hand as DM digi card")
        effect0.set_effect_description(
            "[Start of Your Main Phase] Place 1 hand card face down as any "
            "[DM] Digimon's bottom digi card."
        )
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.owner and len(card.owner.hand_cards) < 1:
                return False
            # Need a DM Digimon on field
            if card and card.owner:
                has_dm = any(p.is_digimon and p.has_trait('DM')
                            for p in card.owner.battle_area)
                if not has_dm:
                    return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            if player.hand_cards:
                hand_card = player.hand_cards.pop()
                # Find a DM Digimon to place under
                for perm in player.battle_area:
                    if perm.is_digimon and perm.has_trait('DM'):
                        perm.add_card_source_bottom(hand_card)
                        break
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [Your Turn] When face-down cards placed, suspend for memory + draw
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX9-069 Your Turn: Suspend for memory on face-down placement")
        effect1.set_effect_description(
            "[Your Turn] When face-down cards placed, suspend to gain 1 memory "
            "and Draw 1 if 7 or fewer hand cards."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            perm = card.permanent_of_this_card()
            if perm and perm.is_suspended:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not player:
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return
            perm.suspend()
            player.add_memory(1)
            if len(player.hand_cards) <= 7:
                player.draw_cards(1)
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [Opponent's Turn] Digimon with face-down digi cards gain Reboot
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.NoTiming)
        effect2.set_effect_name("EX9-069 Opponent's Turn: Reboot for digi card holders")
        effect2.set_effect_description(
            "[Opponent's Turn] All Digimon with face-down digi cards gain Reboot."
        )
        effect2._is_reboot = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner and card.owner.is_my_turn:
                return False  # Only on opponent's turn
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # [Security] Play this card without paying the cost
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("EX9-069 Security: Play this card free")
        effect3.set_effect_description("[Security] Play this card without paying the cost.")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player and card:
                player.play_card_from_source(card, pay_cost=False)
        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
