from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_034(CardScript):
    """BT13-034 Kudamon | Lv.3 Yellow Holy Beast (Vaccine)

    [On Play] Reveal the top 3 cards of your deck. Add 1 yellow Digimon card
        with the [Vaccine] trait and 1 yellow Tamer card among them to your
        hand. Place the rest at the bottom of your deck in any order.

    Inherited: [When Attacking] [Once Per Turn] If the total cards in both
        players' security stacks is 6 or less, 1 of your opponent's Digimon
        gets -2000 DP for the turn.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ----------------------------------------------------------------
        # C1: [On Play] Reveal top 3, add 1 yellow Vaccine Digimon + 1
        # yellow Tamer; rest to deck bottom.
        # ----------------------------------------------------------------
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT13-034 Reveal 3, pick Yellow Vaccine Digimon & Yellow Tamer")
        effect0.set_effect_description(
            "[On Play] Reveal the top 3 cards of your deck. Add 1 yellow "
            "Digimon card with the [Vaccine] trait and 1 yellow Tamer card "
            "among them to your hand. Place the rest at the bottom of your "
            "deck in any order."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Reveal top 3, run two sequential passes: yellow Vaccine digi, then yellow Tamer."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def _has_yellow(c) -> bool:
                colors = getattr(c, 'card_colors', []) or []
                return any(getattr(col, 'name', None) == 'Yellow' for col in colors)

            def reveal_filter_yellow_vaccine_digimon(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not _has_yellow(c):
                    return False
                # [Vaccine] is an attribute, not a type/trait. C# CardTraits
                # merges Form+Attribute+Type; Python card_traits only contains
                # type_eng, so we must check attribute_eng directly.
                base = getattr(c, 'c_entity_base', None)
                attrs = getattr(base, 'attribute_eng', []) if base else []
                return 'Vaccine' in (attrs or [])

            def reveal_filter_yellow_tamer(c):
                if not getattr(c, 'is_tamer', False):
                    return False
                return _has_yellow(c)

            game.effect_reveal_and_select_multi(
                player, 3,
                passes=[
                    (reveal_filter_yellow_vaccine_digimon, 'hand'),
                    (reveal_filter_yellow_tamer, 'hand'),
                ],
                remaining_placement='deck_bottom',
                is_optional=False,
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ----------------------------------------------------------------
        # C2: Inherited [When Attacking] [Once Per Turn]
        # If total sec <= 6, 1 opponent Digimon gets -2000 DP for the turn.
        # ----------------------------------------------------------------
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnUseAttack)
        effect1.set_effect_name("BT13-034 Inherited: When Attacking -2000 DP")
        effect1.set_effect_description(
            "[When Attacking] [Once Per Turn] If the total cards in both "
            "players' security stacks is 6 or less, 1 of your opponent's "
            "Digimon gets -2000 DP for the turn."
        )
        effect1.is_inherited_effect = True
        effect1.is_on_attack = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("DP-2000_BT13_034")

        def condition1(context: Dict[str, Any]) -> bool:
            # Must be on field
            host_perm = card.permanent_of_this_card() if card else None
            if host_perm is None:
                return False
            # Attacker must be the host permanent (when the engine fires this
            # context explicitly). In minimal contexts, allow through.
            ctx_attacker = context.get('attacker')
            if ctx_attacker is not None and ctx_attacker is not host_perm:
                return False
            # Event player must be the host's owner (defensive — normally
            # enforced by engine timing dispatch)
            event_player = context.get('event_player')
            owner = card.owner if card else None
            if event_player is not None and event_player is not owner:
                return False
            if owner is None:
                return False
            enemy = owner.enemy
            if enemy is None:
                return False
            total_sec = len(owner.security_cards) + len(enemy.security_cards)
            if total_sec > 6:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Select 1 opponent Digimon; grant -2000 DP end-of-turn with target-equality guard."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            from ....interfaces.modifiers import ModifierType

            def target_filter(p):
                return p.is_digimon

            def on_target(target_perm):
                # CRITICAL: target-equality condition prevents the modifier
                # from leaking to every permanent queried via get_int_modifier.
                game.register_modifier(
                    target_perm,
                    ModifierType.CHANGE_DP,
                    condition=lambda t, c, _tp=target_perm: t is _tp,
                    value_fn=lambda cur, t, c: cur - 2000,
                    expiry='end_of_turn',
                )

            game.effect_select_opponent_permanent(
                player, on_target,
                filter_fn=target_filter,
                is_optional=False,
                prompt="Select 1 of your opponent's Digimon to give -2000 DP for the turn.",
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
