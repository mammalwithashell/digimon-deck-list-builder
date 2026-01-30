from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class BT23_005(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Main Effect: [Your Turn] When this Digimon would digivolve into a Digimon card with the [Reptile] or [Dragonkin] trait, reduce the digivolution cost by 1.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-005 Main Effect")
        effect1.set_effect_description("[Your Turn] When this Digimon would digivolve into a Digimon card with the [Reptile] or [Dragonkin] trait, reduce the digivolution cost by 1.")

        # Set timing if supported by engine iteration (Player.digivolve iterates WhenWouldDigivolve)
        # Note: ICardEffect doesn't have a 'timing' property itself, the timing is determined by where it's called from.
        # But for 'effect_list(timing)' to return it, we might need to rely on the script organization or just filtering.
        # The engine uses `CardScript.get_card_effects` which returns ALL effects.
        # Then `CardSource.effect_list(timing)` should technically filter.
        # BUT current `CardSource.effect_list` returns EVERYTHING.
        # I need to update `CardSource.effect_list` to filter or just assume the caller filters.
        # Player.digivolve filters by `WhenWouldDigivolve`? No, it asks for `EffectTiming.WhenWouldDigivolve`.
        # I need to ensure `CardSource.effect_list` handles timing?
        # Actually `CardSource.effect_list` calls `script.get_card_effects`.
        # I should probably just return everything and let the caller assume checking properties?
        # WAIT: `Player.digivolve` calls `source.effect_list(EffectTiming.WhenWouldDigivolve)`.
        # But `CardSource.effect_list` implementation is:
        # def effect_list(self, timing):
        #     ...
        #     return script.get_card_effects(self)
        # It ignores the timing arg!
        # So I need to add a way to distinguish timings in the effect itself or filter it.
        # `ICardEffect` doesn't have a `timing` field.
        # Convention: ICardEffect properties like `is_on_attack`, `is_when_digivolving`.
        # I need `is_when_would_digivolve`.
        # But I haven't added `is_when_would_digivolve` to ICardEffect.
        # I should Add it. Or use a generic check.
        # For now, I will add `is_when_would_digivolve` property to ICardEffect via dynamic setter in this script if possible, or assume caller filters by name/type?
        # No, better to add the property to ICardEffect in the previous step... I missed it.
        # I will assume I can add it dynamically or just rely on the fact that `cost_reduction > 0` implies it's for cost reduction timing.

        effect1.cost_reduction = 1

        def condition1(context: Dict[str, Any]) -> bool:
            if not card.owner or not card.owner.is_my_turn:
                return False

            # Check target card traits
            target_card = context.get("card_source")
            if target_card:
                traits = target_card.card_traits
                return "Reptile" in traits or "Dragonkin" in traits
            return False

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Inherited Effect: [Your Turn] This Digimon gets +2000 DP.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-005 Inherited Effect")
        effect2.set_effect_description("[Your Turn] This Digimon gets +2000 DP.")
        effect2.is_inherited_effect = True
        effect2.dp_modifier = 2000

        def condition2(context: Dict[str, Any]) -> bool:
            # Context has "permanent".
            perm = context.get("permanent")
            if perm and perm.top_card and perm.top_card.owner:
                return perm.top_card.owner.is_my_turn
            return False

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
