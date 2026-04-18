from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming
from ....interfaces.modifiers import ModifierType

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_010(CardScript):
    """EX10-010 BlackWarGreymon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blast_digivolve
        # Blast Digivolve
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-010 Blast Digivolve")
        effect0.set_effect_description("Blast Digivolve")
        effect0.is_counter_effect = True
        effect0._is_blast_digivolve = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: raid
        # Raid
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-010 Raid")
        effect1.set_effect_description("Raid")
        effect1.is_on_attack = True
        effect1._is_raid = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: blocker
        # Blocker
        effect2 = ICardEffect()
        effect2.set_effect_name("EX10-010 Blocker")
        effect2.set_effect_description("Blocker")
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: reboot
        # Reboot
        effect3 = ICardEffect()
        effect3.set_effect_name("EX10-010 Reboot")
        effect3.set_effect_description("Reboot")
        effect3._is_reboot = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Shared delete logic for On Play and When Digivolving
        def _delete_opp_play_cost_7_or_less(ctx: Dict[str, Any]):
            """Action: Delete 1 opponent's Digimon or Tamer with play cost 7 or less"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return (p.is_digimon or p.is_tamer) and p.top_card.has_play_cost and p.top_card.get_cost_itself <= 7
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Delete 1 Digimon/Tamer with play cost 7 or less
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("EX10-010 Delete 1 Digimon/Tamer play cost 7 or less")
        effect4.set_effect_description("Delete")
        effect4.set_hash_string("EX10_010_OP")
        effect4.is_on_play = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)
        effect4.set_on_process_callback(_delete_opp_play_cost_7_or_less)
        effects.append(effect4)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Delete 1 Digimon/Tamer with play cost 7 or less
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect5.set_effect_name("EX10-010 Delete 1 Digimon/Tamer play cost 7 or less")
        effect5.set_effect_description("Delete")
        effect5.set_hash_string("EX10_010_WD")
        effect5.is_when_digivolving = True

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect5.set_can_use_condition(condition5)
        effect5.set_on_process_callback(_delete_opp_play_cost_7_or_less)
        effects.append(effect5)

        # [All Turns] While your opponent has a Digimon with 13000 DP or more,
        # your opponent's Digimon's effects don't affect this Digimon, and it gets +3000 DP.

        # DP modifier (+3000 when opponent has 13000+ DP Digimon)
        effect6 = ICardEffect()
        effect6.set_effect_name("EX10-010 +3000 DP vs 13000+ opponent")
        effect6.set_effect_description(
            "[All Turns] While your opponent has a Digimon with 13000 DP or more, "
            "this Digimon gets +3000 DP."
        )
        effect6.dp_modifier = 3000

        def _opp_has_13k_dp():
            """Check if opponent has a Digimon with 13000+ DP."""
            player = card.owner if card else None
            if not player or not player.enemy:
                return False
            enemy = player.enemy
            for p in enemy.battle_area:
                if p.is_digimon:
                    dp = p.dp
                    if dp is not None and dp >= 13000:
                        return True
            return False

        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return _opp_has_13k_dp()

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            pass  # Declarative DP modifier

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        # Effect immunity via CANNOT_BE_AFFECTED modifier.
        # Registered once when the card enters the field (On Play / When Digivolving),
        # with a dynamic condition that checks for opponent's 13000+ DP Digimon.
        # expiry='permanent' ensures cleanup when this card leaves the field.
        effect7 = ICardEffect()
        effect7.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect7.set_effect_name("EX10-010 Register effect immunity modifier")
        effect7.set_effect_description(
            "[All Turns] While your opponent has a Digimon with 13000 DP or more, "
            "your opponent's Digimon's effects don't affect this Digimon."
        )
        effect7.set_hash_string("EX10_010_IMMUNITY_OP")
        effect7.is_on_play = True

        def condition7(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect7.set_can_use_condition(condition7)

        def _register_immunity(ctx: Dict[str, Any]):
            perm = ctx.get('permanent') or (card.permanent_of_this_card() if card else None)
            game = ctx.get('game')
            if not (perm and game):
                return

            def _immunity_condition(target_perm, context):
                """Active only when target is this permanent AND opponent has 13k+ DP."""
                if target_perm is not perm:
                    return False
                return _opp_has_13k_dp()

            game.register_modifier(
                perm, ModifierType.CANNOT_BE_AFFECTED,
                condition=_immunity_condition,
                expiry='permanent',
            )

        effect7.set_on_process_callback(_register_immunity)
        effects.append(effect7)

        # When Digivolving version of the immunity registration
        effect8 = ICardEffect()
        effect8.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect8.set_effect_name("EX10-010 Register effect immunity modifier (WD)")
        effect8.set_effect_description(
            "[All Turns] While your opponent has a Digimon with 13000 DP or more, "
            "your opponent's Digimon's effects don't affect this Digimon."
        )
        effect8.set_hash_string("EX10_010_IMMUNITY_WD")
        effect8.is_when_digivolving = True

        def condition8(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect8.set_can_use_condition(condition8)
        effect8.set_on_process_callback(_register_immunity)
        effects.append(effect8)

        return effects
