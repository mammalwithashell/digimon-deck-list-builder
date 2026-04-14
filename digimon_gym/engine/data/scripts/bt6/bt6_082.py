from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT6_082(CardScript):
    """BT6-082 Sistermon Blanc | Lv.3

    [All Turns] While you have a Digimon with [Huckmon] in its name or
        [Royal Knight] trait in play, all of your Digimon with [Sistermon]
        in their name gain <Blocker>.
    [On Play] <Draw 1> (Draw 1 card from your deck.)
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [All Turns] Grant Blocker to all own Sistermon ---
        # Aura effect: grants Blocker to ALL own Digimon with [Sistermon] in name
        # (including self and other Sistermon variants like Sistermon Ciel).
        # Condition: owner has a Digimon with [Huckmon] in name or [Royal Knight] trait.
        # Uses _applies_to_all_own_digimon + _keyword_permanent_condition pattern
        # so has_keyword() on other permanents finds this aura.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT6-082 Blocker (aura to all Sistermon)")
        effect0.set_effect_description(
            "[All Turns] While you have a Digimon with [Huckmon] in its name "
            "or [Royal Knight] trait in play, all of your Digimon with "
            "[Sistermon] in their name gain <Blocker>."
        )
        effect0._is_blocker = True
        effect0._applies_to_all_own_digimon = True

        def _sistermon_filter(target_perm) -> bool:
            """Only apply to Digimon with [Sistermon] in their name."""
            if not target_perm.is_digimon:
                return False
            return target_perm.contains_card_name('Sistermon')

        effect0._keyword_permanent_condition = _sistermon_filter

        def condition0(context: Dict[str, Any]) -> bool:
            # Host card must be on field
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            # Check target permanent is a Sistermon (for aura path)
            ctx_perm = context.get('permanent')
            if ctx_perm is not None and not _sistermon_filter(ctx_perm):
                return False
            # Check for Huckmon-name or Royal Knight trait ally
            for p in player.battle_area:
                if not p.is_digimon:
                    continue
                if p.contains_card_name('Huckmon'):
                    return True
                if p.has_trait('Royal Knight'):
                    return True
            return False

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [On Play] Draw 1 ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT6-082 Draw 1")
        effect1.set_effect_description("[On Play] <Draw 1>")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.draw_cards(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
