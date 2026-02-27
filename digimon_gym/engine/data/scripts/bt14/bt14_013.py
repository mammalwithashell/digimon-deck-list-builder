from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_013(CardScript):
    """BT14-013 Tyrannomon | Lv.4"""

    @staticmethod
    def _safe_str_list(value: Any) -> List[str]:
        if isinstance(value, list):
            return [str(v) for v in value]
        return []

    @classmethod
    def _matches_tyranno_dino_ceratopsian(cls, obj: Any) -> bool:
        if obj is None:
            return False

        name = str(getattr(obj, "name", "") or getattr(obj, "card_name", "") or getattr(obj, "card_name_eng", ""))
        if "tyrannomon" in name.lower():
            return True

        traits: List[str] = []
        for attr in ("type_eng", "traits", "trait", "card_type", "types"):
            traits.extend(cls._safe_str_list(getattr(obj, attr, None)))

        traits_lower = {t.lower() for t in traits}
        return "dinosaur" in traits_lower or "ceratopsian" in traits_lower

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
            # The reduction applies only to qualifying digivolution targets.
            target = context.get('to_card') or context.get('digivolve_to') or context.get('target_card')
            if target is not None:
                return self._matches_tyranno_dino_ceratopsian(target)
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Effect"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction (variable amount) — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

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
            perm = context.get('permanent') or (card.permanent_of_this_card() if card else None)
            return self._matches_tyranno_dino_ceratopsian(perm)

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Force Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Force attack — target Digimon may attack (requires engine SelectAttack)
            pass  # descriptive-tagged: force_attack

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
