from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_037(CardScript):
    """BT20-037 Chaosmon: Valdur Arm | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Jogress Condition
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-037 Jogress Condition")
        effect0.set_effect_description("Jogress Condition")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: partition
        # Partition
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-037 Partition")
        effect1.set_effect_description("Partition")
        effect1._is_partition = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: security_attack_plus
        # Security Attack +1
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-037 Security Attack +1")
        effect2.set_effect_description("Security Attack +1")
        effect2._security_attack_modifier = 1

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] For each of this Digimon's level 6 digivolution cards,
        # suspend 1 of your opponent's Digimon or Tamers and gain 1 memory.
        # Then, none of their Digimon or Tamers can activate [On Play] effects
        # or unsuspend until the end of their turn.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT20-037 For each level 6 in sources, suspend 1 Digimon/Tamer and memory +1, Then all opponent Digimon/Tamers can't unsuspend until end of their turn")
        effect3.set_effect_description("[When Digivolving] For each of this Digimon's level 6 digivolution cards, suspend 1 of your opponent's Digimon or Tamers and gain 1 memory. Then, none of their Digimon or Tamers can activate [On Play] effects or unsuspend until the end of their turn.")
        effect3.is_when_digivolving = True
        effect3._is_cannot_unsuspend_player = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """For each level 6 digivolution source: suspend 1 opponent Digimon/Tamer + gain 1 memory.
            Then: all opponent Digimon/Tamers cannot unsuspend until end of their turn."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            # Count level-6 digivolution sources (card_sources[:-1] = everything under top).
            lv6_count = 0
            if perm and len(perm.card_sources) > 1:
                for src in perm.card_sources[:-1]:
                    src_level = getattr(src, 'level', None)
                    if src_level == 6:
                        lv6_count += 1

            # For each level-6 digivolution card: suspend 1 opponent Digimon or Tamer,
            # and gain 1 memory.
            for _ in range(lv6_count):
                def on_suspend(target_perm):
                    target_perm.suspend()

                game.effect_select_opponent_permanent(
                    player, on_suspend,
                    filter_fn=lambda p: True,
                    is_optional=False,
                    prompt="Suspend 1 of your opponent's Digimon or Tamers.")

                player.add_memory(1)

            # Then: grant keyword so the effect marker is visible, then apply
            # CANNOT_UNSUSPEND to ALL opponent Digimon and Tamers until end of their turn.
            if perm:
                perm.grant_keyword('_is_cannot_unsuspend_player')

            from engine_py_legacy.engine.interfaces.modifiers import ModifierType
            enemy = player.enemy if player else None
            if enemy and game:
                for opp_perm in list(enemy.battle_area or []):
                    game.register_modifier(
                        ModifierType.CANNOT_UNSUSPEND, opp_perm,
                        value_fn=lambda: True,
                        expiry='end_of_opponent_turn')

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
