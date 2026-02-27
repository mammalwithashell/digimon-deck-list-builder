from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_013(CardScript):
    """BT14-013 Tyrannomon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] For the turn, when this Digimon would digivolve into a card with [Tyrannomon] in its name, or the [Dinosaur] or [Ceratopsian] trait, reduce the digivolution cost by 1.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-013 Reduce digivolution cost")
        effect1.set_effect_description("[Start of Your Main Phase] For the turn, when this Digimon would digivolve into a card with [Tyrannomon] in its name, or the [Dinosaur] or [Ceratopsian] trait, reduce the digivolution cost by 1.")
        effect1.cost_reduction = 1

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False

            # Only applies to this Digimon's digivolution, and only into valid targets.
            source_perm = context.get('permanent') or context.get('source_permanent')
            self_perm = card.permanent_of_this_card() if card else None
            if self_perm is not None and source_perm is not None and source_perm is not self_perm:
                return False

            target = (
                context.get('target_card')
                or context.get('digivolve_to')
                or context.get('to_card')
                or context.get('evo_target')
            )
            if target is None:
                # If target is unavailable in this timing context, keep effect usable and let engine consume cost_reduction.
                return True

            name = str(getattr(target, 'card_name_eng', '') or getattr(target, 'name', '') or '')
            traits = list(getattr(target, 'type_eng', []) or getattr(target, 'traits', []) or [])
            if 'Tyrannomon' in name:
                return True
            return ('Dinosaur' in traits) or ('Ceratopsian' in traits)

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Effect"""
            # Cost reduction (variable amount) — handled via cost_reduction property
            pass

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn][Once Per Turn] If this Digimon has [Tyrannomon] in its name, or the [Dinosaur] or [Ceratopsian] trait, it may attack.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-013 This Digimon attacks")
        effect2.set_effect_description("[End of Your Turn][Once Per Turn] If this Digimon has [Tyrannomon] in its name, or the [Dinosaur] or [Ceratopsian] trait, it may attack.")
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Attack_BT14_013")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False

            # Inherited effect checks the host Digimon.
            host = context.get('permanent') or card.permanent_of_this_card()
            if host is None:
                return False

            name = str(getattr(host, 'name', '') or getattr(host, 'card_name_eng', '') or '')
            traits = list(getattr(host, 'type_eng', []) or getattr(host, 'traits', []) or [])
            if 'Tyrannomon' in name:
                return True
            return ('Dinosaur' in traits) or ('Ceratopsian' in traits)

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Allow this Digimon to attack at end of turn."""
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm is None or game is None:
                return

            # Use optional engine hook when present.
            grant_cb = getattr(game, 'grant_extra_attack_permission', None)
            if callable(grant_cb):
                grant_cb(perm, source='BT14-013')
                return

            # Fallback flag for engines that inspect per-permanent markers.
            setattr(perm, 'can_attack_end_of_turn_bt14_013', True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
