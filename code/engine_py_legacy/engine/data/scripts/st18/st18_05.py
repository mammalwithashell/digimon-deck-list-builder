from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST18_05(CardScript):
    """ST18-05 Muchomon | Lv.3, Green, Avian, DP 1000, Cost 3

    [All Turns][Once Per Turn] When this Digimon is suspended by an effect,
        1 of your Digimon with [Bird] in any of its traits or with the
        [Vortex Warriors] trait gets +3000 DP until the end of your
        opponent's turn.
    [Inherited] <Piercing>

    NOTE: The "suspended by an effect" qualifier cannot be distinguished from
    a combat-triggered suspend at the engine level — permanent.suspend() fires
    OnTappedAnyone with the same context regardless of cause. The effect is
    implemented using OnTappedAnyone with a check that the tapped permanent is
    this card's own permanent. The "by effect" qualifier is enforced descriptively.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [All Turns][Once Per Turn] When this Digimon is suspended by an effect ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnTappedAnyone)
        effect0.set_effect_name("ST18-05 Suspended by effect: Bird/Vortex Warriors Digimon +3000 DP")
        effect0.set_effect_description(
            "[All Turns][Once Per Turn] When this Digimon is suspended by an "
            "effect, 1 of your Digimon with [Bird] in any of its traits or with "
            "the [Vortex Warriors] trait gets +3000 DP until the end of your "
            "opponent's turn."
        )
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("ST18_05_SuspendEffect")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # The tapped permanent must be this card's own permanent
            tapped_perm = context.get('permanent')
            own_perm = card.permanent_of_this_card() if card else None
            if tapped_perm is not own_perm:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Give +3000 DP until end of opponent's turn to 1 Bird/Vortex Warriors Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                if not p.is_digimon:
                    return False
                top = getattr(p, 'top_card', None)
                # Check top card traits for Bird or Vortex Warriors
                traits = getattr(top, 'card_traits', []) or []
                if any('Bird' in t for t in traits):
                    return True
                if any('Vortex Warriors' in t for t in traits):
                    return True
                # Also check any source card in the stack
                for src in getattr(p, 'source_cards', []):
                    src_traits = getattr(src, 'card_traits', []) or []
                    if any('Bird' in t for t in src_traits):
                        return True
                    if any('Vortex Warriors' in t for t in src_traits):
                        return True
                return False

            def on_grant(target_perm):
                from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CHANGE_DP, target_perm,
                    value_fn=lambda: 3000,
                    expiry='end_of_opponent_turn')

            game.effect_select_own_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Inherited] <Piercing> ---
        effect1 = ICardEffect()
        effect1.set_effect_name("ST18-05 Inherited: Piercing")
        effect1.set_effect_description("[Inherited] <Piercing>")
        effect1.is_inherited_effect = True
        effect1._is_piercing = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
