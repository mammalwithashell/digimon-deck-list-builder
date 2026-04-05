from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT8_097(CardScript):
    """BT8-097 Crimson Blaze | Option | Red | Cost:6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── BeforePayCost: variable cost reduction ────────────────────────────
        # Reduce memory cost by 1 for each Digimon opponent has in play.
        # Uses value_fn so the reduction is computed at cost-calculation time,
        # not as a side-effect mutation inside condition.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("BT8-097 Cost reduction")
        effect0.set_effect_description(
            "Reduce the memory cost by 1 for each Digimon your opponent has in play."
        )
        # cost_reduction = 0 initially; actual value computed in condition
        effect0.cost_reduction = 0

        def condition0(context: Dict[str, Any]) -> bool:
            # LEAK GUARD: only for THIS card
            if context.get('card_source') is not card:
                return False
            player = card.owner if card else None
            if not player:
                return False
            enemy = player.enemy
            if not enemy:
                return False
            opp_digi_count = len([p for p in enemy.battle_area if p.is_digimon])
            if opp_digi_count < 1:
                return False
            # Mutate cost_reduction on the effect object — correct pattern for
            # variable BeforePayCost reductions (mirrors BT8-097 C# ChangeCost pattern).
            effect0.cost_reduction = opp_digi_count
            return True

        effect0.set_can_use_condition(condition0)
        # No process callback needed for pure cost-reduction effects
        effects.append(effect0)

        # ── OptionSkill: [Main] effect ─────────────────────────────────────────
        # "Your opponent can't play Digimon by effects until the end of their turn.
        #  Delete all of your opponent's Digimon with 6000 DP or less."
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("BT8-097 Can't play Digimon by effect; delete ≤6000 DP Digimon")
        effect1.set_effect_description(
            "[Main] Your opponent can't play Digimon by effects until the end of their turn. "
            "Delete all of your opponent's Digimon with 6000 DP or less."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """
            1. Register CANNOT_PUT_ON_FIELD on opponent until end of their turn (engine gap #6
               best-effort — restricts by-effect Digimon plays; normal plays unaffected).
            2. Delete all opponent Digimon with 6000 DP or less.
            """
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None

            # Play restriction (engine gap #6 — best-effort CANNOT_PUT_ON_FIELD tag)
            if enemy:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                # Register a player-level modifier on a sentinel if available,
                # or tag each existing permanent. CANNOT_PUT_ON_FIELD with
                # expiry='end_of_opponent_turn' covers "until end of their turn".
                # We register on the breeding slot permanent as a carrier if it exists,
                # otherwise we attach to each opponent Digimon as a best-effort.
                # The condition function checks the card being played is an effect-play.
                # Since we can't easily distinguish effect-plays in the engine, we use
                # a descriptive tag here (known gap #6).
                for opp_perm in list(enemy.battle_area):
                    game.register_modifier(
                        opp_perm,
                        ModifierType.CANNOT_PUT_ON_FIELD,
                        value_fn=lambda: True,
                        expiry='end_of_opponent_turn',
                    )

            # Delete all opponent Digimon with 6000 DP or less
            if enemy:
                to_delete = [
                    p for p in list(enemy.battle_area)
                    if p.is_digimon and (p.dp or 0) <= 6000
                ]
                for target in to_delete:
                    enemy.delete_permanent(target)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ── Security effect: activate Main effects ─────────────────────────────
        # [Security] Activate this card's [Main] effects.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("BT8-097 Security: Activate Main effects")
        effect2.set_effect_description("[Security] Activate this card's [Main] effects.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Security: re-invoke the Main effect logic."""
            process1(ctx)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
