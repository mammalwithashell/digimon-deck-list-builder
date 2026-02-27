from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_082(CardScript):
    """BT14-082 Tai Kamiya"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] 1 of your Digimon with the [Vaccine] trait gets +2000 DP for the turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-082 DP +2000")
        effect0.set_effect_description("[Start of Your Main Phase] 1 of your Digimon with the [Vaccine] trait gets +2000 DP for the turn.")
        effect0.dp_modifier = 2000

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Select 1 of your [Vaccine] Digimon; it gets +2000 DP for the turn."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                if not p:
                    return False
                if getattr(p, 'owner', None) != player:
                    return False
                if not getattr(p, 'is_digimon', lambda: False)():
                    return False
                has_trait = getattr(p, 'has_trait', None)
                if callable(has_trait):
                    return has_trait('Vaccine')
                return False

            def on_select(target_perm):
                target_perm.change_dp(2000)

            game.effect_select_permanent(player, on_select, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnLoseSecurity
        # [Your Turn] When a card is removed from your opponent's security stack, by suspending this Tamer, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-082 Memory +1")
        effect1.set_effect_description("[Your Turn] When a card is removed from your opponent's security stack, by suspending this Tamer, gain 1 memory.")
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend this Tamer; if suspended this way, gain 1 memory."""
            player = ctx.get('player')
            tamer_perm = card.permanent_of_this_card() if card else None
            if not (player and tamer_perm):
                return
            if getattr(tamer_perm, 'is_suspended', False):
                return
            tamer_perm.suspend()
            player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-082 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
