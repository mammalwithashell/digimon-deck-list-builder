from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class AD1_010(CardScript):
    """AD1-010 Garurumon | Lv.4 Blue Digimon | DP 5000 | Cost 5

    [On Play] [When Digivolving] <Draw 1>
    [All Turns] When your Digimon or Tamers are played or digivolve,
    if any of them have [Greymon] or [Matt Ishida] in their names,
    this Digimon may digivolve into a Digimon card with [Garurumon]
    in its name in the hand without paying the cost.

    Inherited: <Jamming>
    Alt-Digi: Lv.3 w/[Omnimon] in text or w/[ADVENTURE] trait: Cost 2
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Alt-digi 1: Lv.3 with ADVENTURE trait, cost 2 ──────────────
        eff_alt1 = ICardEffect()
        eff_alt1.set_effect_name("AD1-010 Alt-digi: ADVENTURE trait")
        eff_alt1._alt_digi_cost = 2
        eff_alt1._alt_digi_level = 3
        eff_alt1._alt_digi_trait = "ADVENTURE"

        def cond_alt1(ctx: Dict[str, Any]) -> bool:
            return True
        eff_alt1.set_can_use_condition(cond_alt1)
        effects.append(eff_alt1)

        # ── Alt-digi 2: Lv.3 with Omnimon in text, cost 2 ─────────────
        eff_alt2 = ICardEffect()
        eff_alt2.set_effect_name("AD1-010 Alt-digi: Omnimon in text")
        eff_alt2._alt_digi_cost = 2
        eff_alt2._alt_digi_level = 3
        # Use _alt_digi_condition_fn for HasText check
        eff_alt2._alt_digi_condition_fn = lambda perm: (
            'Omnimon' in (getattr(perm.top_card, 'card_text', '') or '')
            if perm.top_card else False
        )

        def cond_alt2(ctx: Dict[str, Any]) -> bool:
            return True
        eff_alt2.set_can_use_condition(cond_alt2)
        effects.append(eff_alt2)

        # ── [On Play] <Draw 1> ─────────────────────────────────────────
        eff_op = ICardEffect()
        eff_op.set_timing(EffectTiming.OnEnterFieldAnyone)
        eff_op.set_effect_name("AD1-010 OP: Draw 1")
        eff_op.set_effect_description("[On Play] <Draw 1>")
        eff_op.is_on_play = True

        def cond_op(ctx: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        eff_op.set_can_use_condition(cond_op)

        def proc_op(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.draw()
        eff_op.set_on_process_callback(proc_op)
        effects.append(eff_op)

        # ── [When Digivolving] <Draw 1> ────────────────────────────────
        eff_wd = ICardEffect()
        eff_wd.set_timing(EffectTiming.OnEnterFieldAnyone)
        eff_wd.set_effect_name("AD1-010 WD: Draw 1")
        eff_wd.set_effect_description("[When Digivolving] <Draw 1>")
        eff_wd.is_when_digivolving = True

        def cond_wd(ctx: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        eff_wd.set_can_use_condition(cond_wd)

        def proc_wd(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.draw()
        eff_wd.set_on_process_callback(proc_wd)
        effects.append(eff_wd)

        # ── [All Turns] Reactive digivolve (play observer) ─────────────
        # Fires when one of YOUR Digimon or Tamers is PLAYED with
        # [Greymon] or [Matt Ishida] in name.
        eff_play_obs = ICardEffect()
        eff_play_obs.set_effect_name("AD1-010 Reactive digi (play)")
        eff_play_obs.set_effect_description(
            "[All Turns] When your Digimon or Tamers are played, if any of "
            "them have [Greymon] or [Matt Ishida] in their names, this "
            "Digimon may digivolve into a Digimon card with [Garurumon] in "
            "its name in the hand without paying the cost."
        )
        eff_play_obs.is_optional = True
        eff_play_obs._is_play_observer = True

        def cond_play_obs(ctx: Dict[str, Any]) -> bool:
            # Self must be on field as a Digimon
            perm = card.permanent_of_this_card() if card else None
            if not perm or not perm.is_digimon:
                return False
            # The played permanent must have [Greymon] or [Matt Ishida] name
            played_perm = ctx.get('played_permanent')
            if not played_perm:
                return False
            # Must be our permanent (Digimon or Tamer)
            if not (played_perm.is_digimon or played_perm.is_tamer):
                return False
            if not (played_perm.contains_card_name('Greymon')
                    or played_perm.contains_card_name('Matt Ishida')):
                return False
            return True
        eff_play_obs.set_can_use_condition(cond_play_obs)

        def proc_play_obs(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            perm = card.permanent_of_this_card() if card else None
            if not (player and game and perm):
                return

            def hand_filter(c):
                return (getattr(c, 'is_digimon', False)
                        and c.contains_card_name('Garurumon'))

            game.effect_digivolve_from_hand(
                player, perm, hand_filter,
                cost_override=0, is_optional=True,
                prompt="Select a Digimon with [Garurumon] in its name to digivolve into.")
        eff_play_obs.set_on_process_callback(proc_play_obs)
        effects.append(eff_play_obs)

        # ── [All Turns] Reactive digivolve (digivolve observer) ────────
        # Fires when one of YOUR Digimon digivolves and the result has
        # [Greymon] or [Matt Ishida] in name.
        eff_digi_obs = ICardEffect()
        eff_digi_obs.set_effect_name("AD1-010 Reactive digi (digivolve)")
        eff_digi_obs.set_effect_description(
            "[All Turns] When your Digimon or Tamers digivolve, if any of "
            "them have [Greymon] or [Matt Ishida] in their names, this "
            "Digimon may digivolve into a Digimon card with [Garurumon] in "
            "its name in the hand without paying the cost."
        )
        eff_digi_obs.is_optional = True
        eff_digi_obs._is_digivolve_observer = True

        def cond_digi_obs(ctx: Dict[str, Any]) -> bool:
            # Self must be on field as a Digimon
            perm = card.permanent_of_this_card() if card else None
            if not perm or not perm.is_digimon:
                return False
            # The digivolved permanent must have [Greymon] or [Matt Ishida] name
            digi_perm = ctx.get('digivolved_permanent')
            if not digi_perm:
                return False
            if not (digi_perm.contains_card_name('Greymon')
                    or digi_perm.contains_card_name('Matt Ishida')):
                return False
            return True
        eff_digi_obs.set_can_use_condition(cond_digi_obs)

        def proc_digi_obs(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            perm = card.permanent_of_this_card() if card else None
            if not (player and game and perm):
                return

            def hand_filter(c):
                return (getattr(c, 'is_digimon', False)
                        and c.contains_card_name('Garurumon'))

            game.effect_digivolve_from_hand(
                player, perm, hand_filter,
                cost_override=0, is_optional=True,
                prompt="Select a Digimon with [Garurumon] in its name to digivolve into.")
        eff_digi_obs.set_on_process_callback(proc_digi_obs)
        effects.append(eff_digi_obs)

        # ── Inherited: <Jamming> ───────────────────────────────────────
        eff_jam = ICardEffect()
        eff_jam.set_effect_name("AD1-010 Inherited: Jamming")
        eff_jam._is_jamming = True
        eff_jam.is_inherited_effect = True

        def cond_jam(ctx: Dict[str, Any]) -> bool:
            return True
        eff_jam.set_can_use_condition(cond_jam)
        effects.append(eff_jam)

        return effects
