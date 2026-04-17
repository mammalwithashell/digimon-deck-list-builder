from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT7_032(CardScript):
    """BT7-032 Pulsemon | Lv.3 Yellow Rookie Digimon

    Inherited Effect:
    [When Attacking] [Once Per Turn] If you have 3 security cards,
        gain 2 memory.

    C# reference: BT7_032.cs (uses EffectTiming.OnAllyAttack +
    CanTriggerOnAttack helper which gates to attacker). In the Python
    engine, [When Attacking] maps to EffectTiming.OnUseAttack with
    is_on_attack = True — the engine's _effect_matches_timing gates
    to `perm is attacker`, so no manual attacker check is needed.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Inherited [When Attacking] OPT — gain 2 memory if 3 security ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnUseAttack)
        effect0.set_effect_name("BT7-032 When Attacking: gain 2 memory if 3 security")
        effect0.set_effect_description(
            "[When Attacking] [Once Per Turn] If you have 3 security cards, "
            "gain 2 memory."
        )
        effect0.is_inherited_effect = True
        effect0.is_on_attack = True
        effect0.set_max_count_per_turn(1)
        # Match C# hash_string for cross-implementation traceability
        effect0.set_hash_string("Memory+2_BT7_032")

        def condition0(context: Dict[str, Any]) -> bool:
            # Field presence check (inherited effects only fire when the
            # host permanent is on the field).
            if card and card.permanent_of_this_card() is None:
                return False
            # Owner must exist and have exactly 3 security cards.
            owner = card.owner if card else None
            if not owner:
                return False
            if len(owner.security_cards) != 3:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Gain 2 memory (C#: card.Owner.AddMemory(2))."""
            player = ctx.get('player')
            if player:
                player.add_memory(2)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
