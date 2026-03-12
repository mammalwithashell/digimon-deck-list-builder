from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_054(CardScript):
    """EX11-054 Owen Dreadnought"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: set_memory_3
        # [Start of Your Turn] Set memory to 3 if <= 2
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("EX11-054 Set memory to 3")
        effect0.set_effect_description("[Start of Your Turn] If your memory is at 2 or less, it becomes 3.")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Set memory to 3 if <= 2"""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and game.memory <= 2:
                game.memory = 3
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Shared process for on_play and when_digivolving:
        # When a Digimon with Reptile or Dragonkin enters/digivolves, suspend this tamer,
        # draw 1, then select 1 of your Digimon with <Progress> and give it +3000 DP for the turn.
        def _owen_suspend_draw_dp(ctx: Dict[str, Any]):
            """Action: Suspend this tamer, draw 1, give a Progress Digimon +3000 DP"""
            player = ctx.get('player')
            game = ctx.get('game')
            # Suspend this tamer as cost
            tamer_perm = card.permanent_of_this_card() if card else None
            if tamer_perm and not tamer_perm.is_suspended:
                tamer_perm.suspend()
            if player:
                player.draw_cards(1)
            if player and game:
                def progress_filter(p):
                    return p.is_digimon and getattr(p, '_is_progress', False)
                def on_dp_grant(target_perm):
                    target_perm.change_dp(3000)
                game.effect_select_own_permanent(
                    player, on_dp_grant, filter_fn=progress_filter, is_optional=True)

        def _entering_has_reptile_or_dragonkin(context: Dict[str, Any]) -> bool:
            """Check that the entering/digivolving permanent has Reptile or Dragonkin trait."""
            entering_perm = context.get('permanent')
            if not entering_perm:
                return False
            traits = getattr(entering_perm.top_card, 'card_traits', []) if entering_perm.top_card else []
            traits = traits or []
            return any('Reptile' in t or 'Dragonkin' in t for t in traits)

        # Timing: EffectTiming.OnEnterFieldAnyone — On Play
        # When a Reptile/Dragonkin Digimon is played, by suspending this tamer, draw 1 and give a Progress Digimon +3000 DP.
        effect1_play = ICardEffect()
        effect1_play.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1_play.set_effect_name("EX11-054 By suspending this tamer (On Play), Draw 1 and give a Progress Digimon +3k DP.")
        effect1_play.set_effect_description("[On Play] When a Digimon with [Reptile] or [Dragonkin] trait is played, by suspending this tamer, draw 1. Then give 1 of your <Progress> Digimon +3000 DP for the turn.")
        effect1_play.is_optional = True
        effect1_play.is_on_play = True

        def condition1_play(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            tamer_perm = card.permanent_of_this_card() if card else None
            if tamer_perm and tamer_perm.is_suspended:
                return False
            if not _entering_has_reptile_or_dragonkin(context):
                return False
            return True

        effect1_play.set_can_use_condition(condition1_play)
        effect1_play.set_on_process_callback(_owen_suspend_draw_dp)
        effects.append(effect1_play)

        # Timing: EffectTiming.OnEnterFieldAnyone — When Digivolving
        # When a Reptile/Dragonkin Digimon digivolves, by suspending this tamer, draw 1 and give a Progress Digimon +3000 DP.
        effect1_digi = ICardEffect()
        effect1_digi.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1_digi.set_effect_name("EX11-054 By suspending this tamer (When Digivolving), Draw 1 and give a Progress Digimon +3k DP.")
        effect1_digi.set_effect_description("[When Digivolving] When a Digimon with [Reptile] or [Dragonkin] trait digivolves, by suspending this tamer, draw 1. Then give 1 of your <Progress> Digimon +3000 DP for the turn.")
        effect1_digi.is_optional = True
        effect1_digi.is_when_digivolving = True

        def condition1_digi(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            tamer_perm = card.permanent_of_this_card() if card else None
            if tamer_perm and tamer_perm.is_suspended:
                return False
            if not _entering_has_reptile_or_dragonkin(context):
                return False
            return True

        effect1_digi.set_can_use_condition(condition1_digi)
        effect1_digi.set_on_process_callback(_owen_suspend_draw_dp)
        effects.append(effect1_digi)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("EX11-054 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
