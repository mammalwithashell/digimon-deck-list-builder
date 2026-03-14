from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_095(CardScript):
    """BT13-095 Marcus Damon | Tamer | Red/Yellow | Cost 5

    [Start of Your Turn] If you have 2 or less memory, set it to 3.
    [On Play] You may suspend this Tamer.
    [All Turns] When this Tamer becomes suspended, 1 of your opponent's
        Digimon gets -3000 DP for the turn. Then, if one of your Digimon
        has [Agumon] or [Greymon] in its name, gain 1 memory.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Start of Your Turn] Set memory to 3 if <= 2 ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartTurn)
        effect0.set_effect_name("BT13-095 Set memory to 3")
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

        # --- Effect 1: [On Play] You may suspend this Tamer. ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT13-095 Suspend this Tamer")
        effect1.set_effect_description("[On Play] You may suspend this Tamer.")
        effect1.is_optional = True
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            # Can only activate if not already suspended
            if perm and perm.is_suspended:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Suspend this Tamer."""
            perm = card.permanent_of_this_card() if card else None
            if perm:
                perm.suspend()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [All Turns] When this Tamer becomes suspended ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnTappedAnyone)
        effect2.set_effect_name("BT13-095 DP -3000 and Memory +1")
        effect2.set_effect_description(
            "[All Turns] When this Tamer becomes suspended, 1 of your opponent's "
            "Digimon gets -3000 DP for the turn. Then, if one of your Digimon has "
            "[Agumon] or [Greymon] in its name, gain 1 memory."
        )

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Must be THIS tamer that became suspended
            ctx_perm = context.get('permanent')
            my_perm = card.permanent_of_this_card()
            if my_perm and ctx_perm and ctx_perm is not my_perm:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Select 1 opponent Digimon: -3000 DP for the turn. Then memory +1 if Agumon/Greymon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            def target_filter(p):
                return p.is_digimon

            def on_select(target_perm):
                target_perm.change_dp(-3000)

            # Select 1 opponent Digimon to give -3000 DP
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
            if opp_digimon:
                game.effect_select_opponent_permanent(
                    player, on_select, filter_fn=target_filter, is_optional=False)

            # Then, if one of your Digimon has [Agumon] or [Greymon] in its name, gain 1 memory
            has_agumon_greymon = any(
                p.is_digimon and p.top_card and (
                    p.top_card.contains_card_name('Agumon') or
                    p.top_card.contains_card_name('Greymon')
                )
                for p in player.battle_area
            )
            if has_agumon_greymon:
                player.add_memory(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: Security: Play this card ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("BT13-095 Security: Play this card")
        effect3.set_effect_description("Security: Play this card")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
