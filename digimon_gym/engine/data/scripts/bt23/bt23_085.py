from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_085(CardScript):
    """BT23-085 Ryuji Mishima"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] If you have a Digimon with the [CS] trait, gain 1 memory.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("BT23-085 Gain 1 memory")
        effect0.set_effect_description("[Start of Your Main Phase] If you have a Digimon with the [CS] trait, gain 1 memory.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check for CS trait Digimon on field
            player = card.owner if card else None
            if player:
                has_cs = any(
                    p.is_digimon and any('CS' in t for t in (getattr(p.top_card, 'card_traits', []) or []))
                    for p in player.battle_area
                )
                if not has_cs:
                    return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Until your opponent's turn ends, their effects can't reduce the DP of 1 of your [Hudie] trait Digimon, and it gains <Reboot> and <Blocker>.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT23-085 Opponent's effects can't reduce DP, and gain <Reboot> and <Blocker>")
        effect1.set_effect_description("[On Play] Until your opponent's turn ends, their effects can't reduce the DP of 1 of your [Hudie] trait Digimon, and it gains <Reboot> and <Blocker>.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain Keyword Immune Dp Minus, Gain Keyword Blocker, Gain Keyword Reboot"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if not p.is_digimon:
                    return False
                traits = getattr(p.top_card, 'card_traits', []) or []
                return any('Hudie' in t for t in traits)
            def on_grant(target_perm):
                target_perm.grant_keyword('_is_immune_dp_minus')
                target_perm.grant_keyword('_is_blocker')
                target_perm.grant_keyword('_is_reboot')
            game.effect_select_own_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns] When any of your [Hudie] trait Digimon suspend, by suspending this Tamer, you may use 1 single-color [CS] trait Option card from your hand without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnTappedAnyone)
        effect2.set_effect_name("BT23-085 By suspending this tamer, use 1 option card")
        effect2.set_effect_description("[All Turns] When any of your [Hudie] trait Digimon suspend, by suspending this Tamer, you may use 1 single-color [CS] trait Option card from your hand without paying the cost.")
        effect2.is_optional = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            tamer_perm = card.permanent_of_this_card() if card else None
            if tamer_perm and tamer_perm.is_suspended:
                return False
            # Check that the tapped permanent is own Hudie Digimon
            tapped_perm = context.get('tapped_permanent') or context.get('permanent')
            if tapped_perm:
                traits = getattr(tapped_perm.top_card, 'card_traits', []) or []
                if not any('Hudie' in t for t in traits):
                    return False
                if not tapped_perm.is_digimon:
                    return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Suspend this Tamer, play 1 CS Option from hand free"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Suspend this Tamer as cost
            tamer_perm = card.permanent_of_this_card() if card else None
            if tamer_perm:
                tamer_perm.suspend()
            # Play 1 single-color CS Option from hand free
            def play_filter(c):
                if not getattr(c, 'is_option', False):
                    return False
                c_traits = getattr(c, 'card_traits', []) or []
                if not any('CS' in t for t in c_traits):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: security_play
        # Security: Play this card
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-085 Security: Play this card")
        effect3.set_effect_description("Security: Play this card")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
