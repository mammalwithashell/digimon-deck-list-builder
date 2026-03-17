from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_165(CardScript):
    """P-165 ShoeShoemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: security_play
        # Security: Play this card
        effect0 = ICardEffect()
        effect0.set_effect_name("P-165 Security: Play this card")
        effect0.set_effect_description("Security: Play this card")
        effect0.is_security_effect = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Play 1 [Familiar] Token. (Digimon/Yellow/3000 DP/[On Deletion] 1 of your opponent's Digimon gets -3000 DP for the turn.) At the end of your opponent's turn, delete that token.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("P-165 Play 1 [Familiar] Token")
        effect1.set_effect_description("[When Digivolving] Play 1 [Familiar] Token. (Digimon/Yellow/3000 DP/[On Deletion] 1 of your opponent's Digimon gets -3000 DP for the turn.) At the end of your opponent's turn, delete that token.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def _play_familiar_and_schedule_deletion(ctx: Dict[str, Any]):
            """Play 1 Familiar Token and schedule its deletion at end of opponent's turn."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Track battle_area before to identify the new token
            before = set(id(p) for p in player.battle_area)
            game.effect_play_token(player, 'familiar')
            # Find new token
            new_tokens = [p for p in player.battle_area if id(p) not in before]
            for token_perm in new_tokens:
                # Register an OnEndTurn effect that deletes the token at end of opponent's turn
                delete_eff = ICardEffect()
                delete_eff.set_timing(EffectTiming.OnEndTurn)
                delete_eff.set_effect_name("P-165 Delete Familiar Token at end of opponent's turn")
                _token_ref = token_perm  # capture

                def make_delete_cond(t_perm):
                    def cond(context: Dict[str, Any]) -> bool:
                        owner = card.owner if card else None
                        if not owner:
                            return False
                        # Only fire on opponent's turn end
                        if owner.is_my_turn:
                            return False
                        return t_perm in owner.battle_area
                    return cond

                def make_delete_proc(t_perm):
                    def proc(context: Dict[str, Any]) -> None:
                        p = context.get('player')
                        if not p:
                            p = card.owner if card else None
                        if p and t_perm in p.battle_area:
                            p.delete_permanent(t_perm)
                    return proc

                delete_eff.set_can_use_condition(make_delete_cond(_token_ref))
                delete_eff.set_on_process_callback(make_delete_proc(_token_ref))
                # Attach the effect to the token's card source so it fires
                if _token_ref.top_card:
                    _token_ref.top_card._additional_effects = getattr(
                        _token_ref.top_card, '_additional_effects', [])
                    _token_ref.top_card._additional_effects.append(delete_eff)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Familiar Token (When Digivolving), delete at end of opponent's turn"""
            _play_familiar_and_schedule_deletion(ctx)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Play 1 [Familiar] Token, delete at end of opponent's turn.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("P-165 Play 1 [Familiar] Token")
        effect2.set_effect_description("[On Play] Play 1 [Familiar] Token. (Digimon/Yellow/3000 DP/[On Deletion] 1 of your opponent's Digimon gets -3000 DP for the turn.) At the end of your opponent's turn, delete that token.")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Familiar Token (On Play), delete at end of opponent's turn"""
            _play_familiar_and_schedule_deletion(ctx)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: barrier
        # Barrier
        effect3 = ICardEffect()
        effect3.set_effect_name("P-165 Barrier")
        effect3.set_effect_description("Barrier")
        effect3.is_inherited_effect = True
        effect3._is_barrier = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
