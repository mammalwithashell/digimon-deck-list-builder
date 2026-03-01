from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_078(CardScript):
    """BT23-078 Goro Matayoshi"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: gain_memory_tamer
        # [Start of Main] Gain 1 memory if opponent has Digimon
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("BT23-078 Gain 1 memory")
        effect0.set_effect_description("[Start of Your Main Phase] If your opponent has a Digimon in play, gain 1 memory.")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain 1 memory if opponent has Digimon"""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                opponent = game.opponent_player
                if opponent and any(p.is_digimon for p in opponent.battle_area):
                    game.memory += 1
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn] When your Digimon are played or digivolve, if any of them have [Avian], [Bird], [Beast], [Animal] or [Sovereign] in any of their traits (other than [Sea Animal]) or the [CS] trait, by returning this Tamer to the hand, 1 of your Digimon gets +3000 DP for the turn. Then, 1 of your Digimon may attack.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT23-078 By returning tamer to hand, 1 digimon gains +3K DP. then 1 digimon may attack")
        effect1.set_effect_description("[Your Turn] When your Digimon are played or digivolve, if any of them have [Avian], [Bird], [Beast], [Animal] or [Sovereign] in any of their traits (other than [Sea Animal]) or the [CS] trait, by returning this Tamer to the hand, 1 of your Digimon gets +3000 DP for the turn. Then, 1 of your Digimon may attack.")
        effect1.is_optional = True
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP +3000, Force Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(3000)
            # Force attack — target Digimon may attack (requires engine SelectAttack)
            pass  # descriptive-tagged: force_attack

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-078 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
