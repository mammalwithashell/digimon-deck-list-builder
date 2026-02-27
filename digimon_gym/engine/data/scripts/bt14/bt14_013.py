from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_013(CardScript):
    """BT14-013 Tyrannomon | Lv.4"""

    @staticmethod
    def _name_or_trait_match(name: str, traits: List[str]) -> bool:
        n = (name or "").lower()
        t = [str(x).lower() for x in (traits or [])]
        if "tyrannomon" in n:
            return True
        return "dinosaur" in t or "ceratopsian" in t

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

            # Apply only when the digivolution target matches Tyrannomon/Dinosaur/Ceratopsian.
            target = context.get('to_card') or context.get('digivolve_to') or context.get('target_card')
            if target is None:
                return False

            target_name = getattr(target, 'card_name_eng', None) or getattr(target, 'name', '')
            target_traits = getattr(target, 'type_eng', None) or getattr(target, 'traits', [])
            return self._name_or_trait_match(target_name, target_traits)

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

            # This inherited effect checks the current Digimon's own name/traits.
            perm = context.get('permanent')
            if perm is None:
                return False

            p_name = getattr(perm, 'card_name_eng', None) or getattr(perm, 'name', '')
            p_traits = getattr(perm, 'type_eng', None) or getattr(perm, 'traits', [])
            return self._name_or_trait_match(p_name, p_traits)

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
