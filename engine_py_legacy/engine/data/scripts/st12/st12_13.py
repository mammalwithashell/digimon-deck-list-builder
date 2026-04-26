from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST12_13(CardScript):
    """ST12-13 Sistermon Ciel | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Reveal the top 3 cards of your deck. Add 1 card with [Huckmon] in its name or [Royal Knight] in its traits among them to your hand. Trash the rest.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("ST12-13 Reveal the top 3 cards of deck")
        effect0.set_effect_description("[On Play] Reveal the top 3 cards of your deck. Add 1 card with [Huckmon] in its name or [Royal Knight] in its traits among them to your hand. Trash the rest.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Reveal top 3, add 1 Huckmon/Royal Knight to hand, trash the rest."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def reveal_filter(c):
                if c.contains_card_name('Huckmon'):
                    return True
                if any('Royal Knight' in t for t in (getattr(c, 'card_traits', []) or [])):
                    return True
                return False

            game.effect_reveal_and_select_multi(
                player, 3,
                [(reveal_filter, 'hand')],
                remaining_placement='trash',
                is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [All Turns] All of your Digimon with [Huckmon] in their names or
        # [Royal Knight] trait gain <Reboot>.
        # Uses aura keyword pattern: _applies_to_all_own_digimon with
        # _keyword_permanent_condition to filter targets by Huckmon/Royal Knight.

        def _is_huckmon_or_royal_knight(target_perm) -> bool:
            """Check if a permanent has Huckmon name or Royal Knight trait."""
            if not target_perm.is_digimon:
                return False
            if target_perm.contains_card_name('Huckmon'):
                return True
            if target_perm.has_trait('Royal Knight'):
                return True
            return False

        effect1 = ICardEffect()
        effect1.set_effect_name("ST12-13 Reboot (grant to Huckmon/Royal Knight)")
        effect1.set_effect_description(
            "[All Turns] All of your Digimon with [Huckmon] in their names or "
            "[Royal Knight] trait gain <Reboot>.")
        effect1._is_reboot = True
        effect1._applies_to_all_own_digimon = True
        effect1._keyword_permanent_condition = _is_huckmon_or_royal_knight

        def condition1(context: Dict[str, Any]) -> bool:
            # Source must be on field
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
