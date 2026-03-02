from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class RB1_035(CardScript):
    """RB1-035 Hokuto Amanokawa | Tamer

    [Start of Your Turn] If your opponent has 3 or more Tamers in play,
        gain 1 memory.
    [All Turns] When an opponent plays a Digimon, by suspending this Tamer,
        gain 1 memory if that Digimon is level 4 or higher, and <Draw 1>
        if it is level 3.
    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Start of Your Turn] Memory gain if opponent has 3+ Tamers ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("RB1-035 Gain 1 memory if opponent has 3+ Tamers")
        effect0.set_effect_description(
            "[Start of Your Turn] If your opponent has 3 or more Tamers "
            "in play, gain 1 memory."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            game = context.get('game')
            if game and game.current_player != player:
                return False
            enemy = player.enemy
            if not enemy:
                return False
            tamer_count = sum(1 for p in enemy.battle_area if p.is_tamer)
            return tamer_count >= 3
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                game.set_memory(game.memory + 1)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [All Turns] When opponent plays Digimon, suspend to gain memory/draw ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("RB1-035 Suspend when opponent plays Digimon")
        effect1.set_effect_description(
            "[All Turns] When an opponent plays a Digimon, by suspending "
            "this Tamer, gain 1 memory if that Digimon is level 4 or higher, "
            "and <Draw 1> if it is level 3."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm or tamer_perm.is_suspended:
                return False
            player = card.owner if card else None
            if not player:
                return False
            # Must be triggered by opponent playing a Digimon
            trigger_perm = context.get('permanent')
            if not trigger_perm:
                return False
            if trigger_perm.owner == player:
                return False
            if not trigger_perm.is_digimon:
                return False
            ctx_on_play = context.get('is_on_play', False)
            if not ctx_on_play:
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
            # Check the played Digimon's level
            trigger_perm = ctx.get('permanent')
            if not trigger_perm:
                return
            level = trigger_perm.level or 0
            if level >= 4:
                game.set_memory(game.memory + 1)
            elif level == 3:
                player.draw_cards(1)
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Play this card without paying the cost ---
        effect2 = ICardEffect()
        effect2.set_effect_name("RB1-035 Security Effect")
        effect2.set_effect_description("[Security] Play this card without paying the cost.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and card:
                game.effect_play_card_from_security(player, card)
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
