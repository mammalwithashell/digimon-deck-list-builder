from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_102(CardScript):
    """BT24-102 Homeros"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] Gain 1 memory. Then, if you have 5 or more memory, suspend this Tamer and ＜Draw 1＞.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("BT24-102 Gain 1 Memory. If 5+ Memory, suspend and Draw 1.")
        effect0.set_effect_description("[Start of Your Main Phase] Gain 1 memory. Then, if you have 5 or more memory, suspend this Tamer and ＜Draw 1＞.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1, Gain 1 memory, Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)
            if player and game and game.memory >= 5 and perm:
                perm.suspend()
                player.draw_cards(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: dp_modifier_all
        # All your Digimon DP modifier
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-102 All your Digimon DP modifier")
        effect1.set_effect_description("All your Digimon DP modifier")
        effect1.dp_modifier = 1000
        effect1._applies_to_all_own_digimon = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] By suspending this Tamer, you may activate 1 [On Play] or [When Digivolving] effect of 1 of your [Olympos XII] trait Digimon.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEndTurn)
        effect2.set_effect_name("BT24-102 Suspend this tamer to use an On Play or When Digivolving")
        effect2.set_effect_description("[End of Your Turn] By suspending this Tamer, you may activate 1 [On Play] or [When Digivolving] effect of 1 of your [Olympos XII] trait Digimon.")
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm or tamer_perm.is_suspended:
                return False  # Can't pay cost if already suspended
            player = card.owner
            # Check for Olympos XII Digimon on field
            for p in player.battle_area:
                if not p.is_digimon:
                    continue
                for cs in p.card_sources:
                    traits = getattr(cs, 'card_traits', []) or []
                    if any('Olympos XII' in t for t in traits):
                        return True
            return False

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Suspend Homeros, reactivate On Play/When Digivolving of Olympos XII."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            # Cost: suspend this Tamer
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return
            tamer_perm.suspend()

            # Select Olympos XII Digimon to reactivate
            def olympos_filter(p):
                if not p.is_digimon:
                    return False
                for cs in p.card_sources:
                    traits = getattr(cs, 'card_traits', []) or []
                    if any('Olympos XII' in t for t in traits):
                        return True
                return False

            def on_selected(target):
                # Re-trigger On Play / When Digivolving effects
                game.execute_effects(
                    EffectTiming.OnEnterFieldAnyone,
                    {"played_card": target.top_card,
                     "played_permanent": target,
                     "event_player": player,
                     "permanent": target},
                )

            game.effect_select_own_permanent(
                player, on_selected, filter_fn=olympos_filter,
                is_optional=True,
                prompt="Select an Olympos XII Digimon to reactivate.")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: security_play
        # Security: Play this card
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-102 Security: Play this card")
        effect3.set_effect_description("Security: Play this card")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return False  # Security effects handled by engine
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
