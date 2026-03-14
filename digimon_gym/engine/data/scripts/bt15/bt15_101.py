from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_101(CardScript):
    """BT15-101 MetalGarurumon | Lv.6

    Alt digivolve: from Lv.5 [Garurumon] for 4.
    Alt digivolve: If you have [Matt Ishida] and opponent has Digimon with
        10000+ DP, from [Gabumon] for 4 ignoring requirements.
    Evade.
    [When Digivolving] 3 of your opponent's Digimon and Tamers can't suspend
        until the end of their turn.
    [All Turns][Once Per Turn] When this Digimon becomes suspended, you may
        unsuspend it.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt digivolve from Lv.5 Garurumon for 4 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT15-101 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 4
        effect0._alt_digi_level = 5
        effect0._alt_digi_name = "Garurumon"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: Evade ---
        effect1 = ICardEffect()
        effect1.set_effect_name("BT15-101 Evade")
        effect1.set_effect_description("Evade")
        effect1._is_evade = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: [When Digivolving] 3 opponent Digimon/Tamers can't suspend ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT15-101 Opponent's 3 Digimon/Tamers can't suspend")
        effect2.set_effect_description(
            "[When Digivolving] 3 of your opponent's Digimon and Tamers can't "
            "suspend until the end of their turn."
        )
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Select up to 3 opponent Digimon/Tamers, apply CANNOT_SUSPEND."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            # Filter for opponent Digimon and Tamers
            targets = [p for p in enemy.battle_area
                       if p.is_digimon or p.is_tamer]
            if not targets:
                return

            from digimon_gym.engine.interfaces.modifiers import ModifierType

            count = min(3, len(targets))
            for i in range(count):
                remaining = [p for p in enemy.battle_area
                             if p.is_digimon or p.is_tamer]
                if not remaining:
                    break

                def opp_filter(p):
                    return p.is_digimon or p.is_tamer

                def on_select(target_perm):
                    game.register_modifier(
                        ModifierType.CANNOT_SUSPEND, target_perm,
                        value_fn=lambda: True, expiry='end_of_opponent_turn')

                game.effect_select_opponent_permanent(
                    player, on_select,
                    filter_fn=opp_filter,
                    is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: [All Turns][OPT] When THIS Digimon suspends, unsuspend it ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnTappedAnyone)
        effect3.set_effect_name("BT15-101 Unsuspend this Digimon")
        effect3.set_effect_description(
            "[All Turns] [Once Per Turn] When this Digimon becomes suspended, "
            "you may unsuspend it."
        )
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("Unsuspen_BT15_101")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only trigger when THIS Digimon is the one being suspended
            event_perm = context.get('event_permanent')
            owner_perm = card.permanent_of_this_card()
            if event_perm and owner_perm and event_perm is not owner_perm:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Unsuspend this Digimon (self only)."""
            perm = card.permanent_of_this_card() if card else None
            if perm and perm.is_suspended:
                perm.unsuspend()

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
