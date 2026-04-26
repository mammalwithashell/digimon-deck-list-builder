from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST15_14(CardScript):
    """ST15-14 Tai Kamiya | Tamer (Red, Cost 4)

    [Start of Your Turn] If you have 2 or less memory, set it to 3.
    [All Turns] When an attack target is switched, by suspending this Tamer,
        <Draw 1> and 1 of your Digimon gets +2000 DP for the turn.
    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Start of Your Turn] Set memory to 3 ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("ST15-14 Set memory to 3")
        effect0.set_effect_description(
            "[Start of Your Turn] If you have 2 or less memory, set it to 3."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and game.memory <= 2:
                game.memory = 3

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [All Turns] When attack target switched ---
        # By suspending this Tamer, Draw 1 and +2000 DP to 1 Digimon
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnAttackTargetChanged)
        effect1.set_effect_name("ST15-14 Attack target switched: Draw 1 + DP boost")
        effect1.set_effect_description(
            "[All Turns] When an attack target is switched, by suspending "
            "this Tamer, <Draw 1> and 1 of your Digimon gets +2000 DP "
            "for the turn."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            tamer_perm = card.permanent_of_this_card()
            if tamer_perm and tamer_perm.is_suspended:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return
            # Cost: suspend this Tamer
            tamer_perm.suspend()
            # Draw 1
            player.draw()
            # +2000 DP to 1 of your Digimon for the turn
            def on_select(target_perm):
                target_perm.change_dp(2000)

            game.effect_select_own_permanent(
                player, on_select,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False,
                prompt="Select 1 of your Digimon to get +2000 DP."
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Play this card without paying the cost ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("ST15-14 Security: Play free")
        effect2.set_effect_description(
            "[Security] Play this card without paying the cost."
        )
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and card:
                player.play_card_from_source(card, pay_cost=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
