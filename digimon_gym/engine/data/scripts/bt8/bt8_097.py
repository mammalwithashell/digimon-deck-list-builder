from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....interfaces.modifiers import ModifierType, ModifierEntry
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT8_097(CardScript):
    """BT8-097 Crimson Blaze | Option | Red | Cost:6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── BeforePayCost: variable cost reduction ────────────────────────────
        # Reduce memory cost by 1 for each Digimon opponent has in play.
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
            1. Register CANNOT_PLAY_BY_EFFECT for opponent's Digimon until end of
               their turn.  Uses global ModifierEntry (source_permanent=None) since
               this is an Option card that goes to trash after use.
            2. Delete all opponent Digimon with 6000 DP or less.
            """
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None

            # "Your opponent can't play Digimon by effects until the end of their turn."
            # Register CANNOT_PLAY_BY_EFFECT: blocks effect-based Digimon plays only.
            # Normal hand plays are unaffected. Uses global registration pattern
            # (source_permanent=None) since this Option card doesn't stay on the field.
            if enemy:
                game.modifiers.register(ModifierEntry(
                    modifier_type=ModifierType.CANNOT_PLAY_BY_EFFECT,
                    condition=lambda target, c: (
                        c.get('card') is not None
                        and getattr(c['card'], 'is_digimon', False)
                    ),
                    source_permanent=None,
                    expiry='end_of_opponent_turn',
                    granting_player=player,
                ))

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
