from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_054(CardScript):
    """EX11-054 Owen Dreadnought

    C# reference (behavioral source of truth):
      - [Start of Your Turn] SetMemoryTo3TamerEffect
      - [All Turns] When your Digimon are played or digivolve, if [Reptile] or
        [Dragonkin], by suspending this Tamer, Draw 1. Then 1 of your Digimon
        with <Progress> gets +3000 DP for the turn.
      - [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Effect 0: [Start of Your Turn] Set memory to 3 ──────────
        # C# uses EffectTiming.OnStartTurn (NOT OnStartMainPhase).
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartTurn)
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

        # ── Shared helpers ───────────────────────────────────────────

        def _has_reptile_or_dragonkin(perm) -> bool:
            """Check if a permanent's top card has Reptile or Dragonkin trait."""
            if not perm:
                return False
            top = perm.top_card if hasattr(perm, 'top_card') else None
            if not top:
                return False
            traits = getattr(top, 'card_traits', []) or []
            return any('Reptile' in t or 'Dragonkin' in t for t in traits)

        def _owen_suspend_draw_dp(ctx: Dict[str, Any]):
            """Action: Suspend this tamer, draw 1, give a Progress Digimon +3000 DP."""
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
                    return p.is_digimon and p.has_keyword('_is_progress')
                def on_dp_grant(target_perm):
                    target_perm.change_dp(3000)
                game.effect_select_own_permanent(
                    player, on_dp_grant, filter_fn=progress_filter, is_optional=False)

        # ── Effect 1: [All Turns] On Play — Reptile/Dragonkin ────────
        # Timing: OnEnterFieldAnyone WITHOUT is_on_play flag.
        # This makes the effect fire for ANY permanent entering the field
        # (not just this card's own play). The condition then filters for
        # the entering permanent being a Reptile/Dragonkin Digimon owned
        # by this tamer's controller.
        effect1_play = ICardEffect()
        effect1_play.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1_play.set_effect_name("EX11-054 By suspending this tamer (On Play), Draw 1 and give a Progress Digimon +3k DP.")
        effect1_play.set_effect_description("[All Turns] When your Digimon with [Reptile] or [Dragonkin] is played, by suspending this tamer, draw 1. Then give 1 of your <Progress> Digimon +3000 DP for the turn.")
        effect1_play.is_optional = True

        def condition1_play(context: Dict[str, Any]) -> bool:
            # Tamer must be on field
            if card and card.permanent_of_this_card() is None:
                return False
            # Tamer must be unsuspended (suspend is the cost)
            tamer_perm = card.permanent_of_this_card() if card else None
            if tamer_perm and tamer_perm.is_suspended:
                return False
            # Check the entering permanent is a Digimon with Reptile/Dragonkin
            # owned by this tamer's controller
            entering_perm = context.get('played_permanent')
            if not entering_perm:
                return False
            if not entering_perm.is_digimon:
                return False
            # Must be OUR Digimon (same owner as tamer)
            owner = card.owner if card else None
            if owner and hasattr(entering_perm, 'owner') and entering_perm.owner != owner:
                return False
            if not _has_reptile_or_dragonkin(entering_perm):
                return False
            return True

        effect1_play.set_can_use_condition(condition1_play)
        effect1_play.set_on_process_callback(_owen_suspend_draw_dp)
        effects.append(effect1_play)

        # ── Effect 2: [All Turns] When Digivolving — Reptile/Dragonkin ─
        # Timing: WhenDigivolving WITHOUT is_when_digivolving flag.
        # This makes the effect fire for ANY permanent digivolving on the field.
        # The condition filters for the digivolving permanent being owned by
        # this tamer's controller and having Reptile/Dragonkin.
        effect2_digi = ICardEffect()
        effect2_digi.set_timing(EffectTiming.WhenDigivolving)
        effect2_digi.set_effect_name("EX11-054 By suspending this tamer (When Digivolving), Draw 1 and give a Progress Digimon +3k DP.")
        effect2_digi.set_effect_description("[All Turns] When your Digimon with [Reptile] or [Dragonkin] digivolves, by suspending this tamer, draw 1. Then give 1 of your <Progress> Digimon +3000 DP for the turn.")
        effect2_digi.is_optional = True

        def condition2_digi(context: Dict[str, Any]) -> bool:
            # Tamer must be on field
            if card and card.permanent_of_this_card() is None:
                return False
            # Tamer must be unsuspended
            tamer_perm = card.permanent_of_this_card() if card else None
            if tamer_perm and tamer_perm.is_suspended:
                return False
            # Check the digivolving permanent
            digivolved_perm = context.get('digivolved_permanent')
            if not digivolved_perm:
                return False
            if not digivolved_perm.is_digimon:
                return False
            # Must be OUR Digimon
            owner = card.owner if card else None
            if owner and hasattr(digivolved_perm, 'owner') and digivolved_perm.owner != owner:
                return False
            if not _has_reptile_or_dragonkin(digivolved_perm):
                return False
            return True

        effect2_digi.set_can_use_condition(condition2_digi)
        effect2_digi.set_on_process_callback(_owen_suspend_draw_dp)
        effects.append(effect2_digi)

        # ── Effect 3: [Security] Play this card ─────────────���────────
        effect3 = ICardEffect()
        effect3.set_effect_name("EX11-054 Security: Play this card")
        effect3.set_effect_description("Security: Play this card")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
