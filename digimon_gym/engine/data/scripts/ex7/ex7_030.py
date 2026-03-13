from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX7_030(CardScript):
    """EX7-030 Cendrillmon | Lv.6, Yellow, Puppet/LIBERATOR, DP 12000, Cost 12

    <Overclock ([Puppet] Trait)>
    [Start of Your Main Phase] [When Digivolving] You may play 1 [Familiar]
        Token. (Digimon/Yellow/3000 DP/[On Deletion] 1 of your opponent's
        Digimon gets -3000 DP for the turn.)
    [When Attacking] 1 of your opponent's Digimon gets -6000 DP for the turn.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Overclock ([Puppet] Trait) ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX7-030 Overclock")
        effect0.set_effect_description("Overclock ([Puppet] Trait)")
        effect0._is_overclock = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [Start of Your Main Phase] Play 1 [Familiar] Token ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnStartMainPhase)
        effect1.set_effect_name("EX7-030 Play Familiar Token (Start of Main Phase)")
        effect1.set_effect_description(
            "[Start of Your Main Phase] You may play 1 [Familiar] Token."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Play 1 Familiar Token."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            game.effect_play_token(player, 'familiar')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [When Digivolving] Play 1 [Familiar] Token ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX7-030 Play Familiar Token (When Digivolving)")
        effect2.set_effect_description(
            "[When Digivolving] You may play 1 [Familiar] Token."
        )
        effect2.is_when_digivolving = True
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Play 1 Familiar Token."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            game.effect_play_token(player, 'familiar')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: [When Attacking] 1 opponent Digimon gets -6000 DP for the turn ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnUseAttack)
        effect3.set_effect_name("EX7-030 Opponent Digimon -6000 DP when attacking")
        effect3.set_effect_description(
            "[When Attacking] 1 of your opponent's Digimon gets -6000 DP for the turn."
        )
        effect3.is_on_attack = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Select 1 opponent Digimon to get -6000 DP."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_select(target_perm):
                target_perm.change_dp(-6000)

            game.effect_select_opponent_permanent(
                player, on_select,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False,
                prompt="Select 1 of your opponent's Digimon to get -6000 DP.",
            )

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
