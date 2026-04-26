from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_028(CardScript):
    """BT16-028 Imperialdramon: Dragon Mode | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Alt-digi from Paildramon (cost 3) ──────────────────────────
        effect0a = ICardEffect()
        effect0a.set_effect_name("BT16-028 Alt-digi from Paildramon")
        effect0a.set_effect_description("Alternate digivolution: Paildramon for cost 3")
        effect0a._alt_digi_cost = 3
        effect0a._alt_digi_name = "Paildramon"

        def condition0a(context: Dict[str, Any]) -> bool:
            return True
        effect0a.set_can_use_condition(condition0a)
        effects.append(effect0a)

        # ── Alt-digi from Dinobeemon (cost 3) ──────────────────────────
        effect0b = ICardEffect()
        effect0b.set_effect_name("BT16-028 Alt-digi from Dinobeemon")
        effect0b.set_effect_description("Alternate digivolution: Dinobeemon for cost 3")
        effect0b._alt_digi_cost = 3
        effect0b._alt_digi_name = "Dinobeemon"

        def condition0b(context: Dict[str, Any]) -> bool:
            return True
        effect0b.set_can_use_condition(condition0b)
        effects.append(effect0b)

        # ── [When Digivolving] ──────────────────────────────────────────
        # 1 of your opponent's Digimon or Tamers can't unsuspend until the
        # end of their turn.  Then, by suspending 1 of their Digimon or
        # Tamers, unsuspend 1 of your Digimon.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT16-028 [When Digivolving] freeze + suspend/unsuspend")
        effect1.set_effect_description(
            "[When Digivolving] 1 of your opponent's Digimon or Tamers "
            "can't unsuspend until the end of their turn. Then, by "
            "suspending 1 of their Digimon or Tamers, unsuspend 1 of "
            "your Digimon.")
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            from engine_py_legacy.engine.interfaces.modifiers import ModifierType

            # Step 1: Select 1 opponent Digimon or Tamer → apply cannot-unsuspend
            # Step 2 is chained inside on_freeze so the selections don't
            # overwrite each other (engine supports one pending_selection).
            def opp_digi_or_tamer(p):
                return p.is_digimon or p.is_tamer

            def on_freeze(target_perm):
                game.register_modifier(
                    target_perm,
                    ModifierType.CANNOT_UNSUSPEND,
                    expiry='end_of_opponent_turn')

                # Step 2 (optional): Suspend 1 of opponent's unsuspended
                # Digimon/Tamers, then unsuspend 1 of your own suspended Digimon
                def opp_unsuspended_digi_or_tamer(p):
                    return (p.is_digimon or p.is_tamer) and not p.is_suspended

                def on_suspend_opp(susp_perm):
                    susp_perm.suspend()
                    # After suspending opponent's permanent, unsuspend own Digimon
                    def own_suspended_digimon(p):
                        return p.is_digimon and p.is_suspended

                    def on_unsuspend_own(own_perm):
                        own_perm.unsuspend()

                    game.effect_select_own_permanent(
                        player, on_unsuspend_own,
                        filter_fn=own_suspended_digimon,
                        is_optional=False,
                        prompt="Select 1 of your suspended Digimon to unsuspend.")

                game.effect_select_opponent_permanent(
                    player, on_suspend_opp,
                    filter_fn=opp_unsuspended_digi_or_tamer,
                    is_optional=True,
                    prompt="Suspend 1 of your opponent's Digimon or Tamers "
                           "to unsuspend 1 of yours.")

            game.effect_select_opponent_permanent(
                player, on_freeze, filter_fn=opp_digi_or_tamer,
                is_optional=False,
                prompt="Select 1 of your opponent's Digimon or Tamers "
                       "(can't unsuspend until end of their turn).")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ── [All Turns] ────────────────────────────────────────────────
        # When an effect plays or digivolves an opponent's Digimon, if you
        # have a Tamer, this Digimon may digivolve into [Imperialdramon:
        # Fighter Mode] in your hand without paying the cost.
        #
        # This is a reactive trigger that fires on OnEnterFieldAnyone
        # WITHOUT is_on_play flag, so it fires for ANY permanent entering
        # the field.  The condition checks whether the entering permanent
        # belongs to the opponent and was played/digivolved by an effect.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT16-028 [All Turns] digivolve into Fighter Mode")
        effect2.set_effect_description(
            "[All Turns] When an effect plays or digivolves an opponent's "
            "Digimon, if you have a Tamer, this Digimon may digivolve "
            "into [Imperialdramon: Fighter Mode] in your hand without "
            "paying the cost.")
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Must be on the field
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            player = context.get('player')
            if not player:
                return False
            # The played/digivolved permanent must belong to opponent
            event_player = context.get('event_player')
            if event_player is None or event_player is player:
                return False
            played_perm = context.get('played_permanent')
            if played_perm is None or not played_perm.is_digimon:
                return False
            # Must have a Tamer on field
            has_tamer = any(p.is_tamer for p in player.battle_area)
            if not has_tamer:
                return False
            # Must have Imperialdramon: Fighter Mode in hand
            has_fighter = any(
                c.contains_card_name('Imperialdramon: Fighter Mode')
                for c in player.hand_cards
            )
            if not has_fighter:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            def digi_filter(c):
                return c.contains_card_name('Imperialdramon: Fighter Mode')

            game.effect_digivolve_from_hand(
                player, perm, digi_filter,
                cost_override=0,
                is_optional=True,
                prompt="Select [Imperialdramon: Fighter Mode] from hand "
                       "to digivolve without paying the cost.")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
