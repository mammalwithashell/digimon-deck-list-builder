from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, CardColor

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_031(CardScript):
    """BT22-031 GoldNumemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: Lv.4 + Yellow for cost 2
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-031 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 2
        effect0._alt_digi_level = 4
        effect0._alt_digi_color = CardColor.Yellow

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        def _build_main_effect(is_on_play=False, is_when_digivolving=False):
            effect = ICardEffect()
            effect.set_timing(EffectTiming.OnEnterFieldAnyone)
            trigger = "[On Play]" if is_on_play else "[When Digivolving]"
            effect.set_effect_name(f"BT22-031 {trigger} SA-2, then digivolve into PlatinumNumemon")
            effect.set_effect_description(
                f"{trigger} Give 1 of your opponent's Digimon <Security A. -2> until their turn ends. "
                "Then, if this Digimon's stack has 2 or more same-level cards, this Digimon may "
                "digivolve into [PlatinumNumemon] in the hand for a digivolution cost of 4, "
                "ignoring digivolution requirements."
            )
            effect.is_on_play = is_on_play
            effect.is_when_digivolving = is_when_digivolving

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                return True

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                perm = ctx.get('permanent')
                game = ctx.get('game')
                if not (player and perm and game):
                    return

                from ....interfaces.modifiers import ModifierType

                # Step 1: Give 1 opponent Digimon Security A. -2
                def target_filter(p):
                    return p.is_digimon

                def on_sa_target(target_perm):
                    target_perm._temp_sa_modifier -= 2

                    # Step 2: Check same-level condition in stack
                    cur_perm = card.permanent_of_this_card() if card else None
                    if not cur_perm:
                        return
                    levels = [
                        getattr(cs, 'level', None)
                        for cs in cur_perm.card_sources
                        if getattr(cs, 'level', None) is not None
                    ]
                    from collections import Counter
                    level_counts = Counter(levels)
                    has_same_level = any(c >= 2 for c in level_counts.values())

                    if has_same_level:
                        def digi_filter(c):
                            if not getattr(c, 'is_digimon', False):
                                return False
                            names = getattr(c, 'card_names', []) or []
                            return any('PlatinumNumemon' in n for n in names)

                        game.effect_digivolve_from_hand(
                            player, cur_perm, digi_filter,
                            cost_override=4,
                            ignore_requirements=True,
                            is_optional=True)

                enemy = player.enemy
                opp_digimon = [p for p in enemy.battle_area if p.is_digimon] if enemy else []
                if opp_digimon:
                    game.effect_select_opponent_permanent(
                        player, on_sa_target, filter_fn=target_filter, is_optional=False)
                else:
                    # No opponent Digimon, still check digivolve condition
                    cur_perm = card.permanent_of_this_card() if card else None
                    if cur_perm:
                        levels = [
                            getattr(cs, 'level', None)
                            for cs in cur_perm.card_sources
                            if getattr(cs, 'level', None) is not None
                        ]
                        from collections import Counter
                        level_counts = Counter(levels)
                        has_same_level = any(c >= 2 for c in level_counts.values())
                        if has_same_level:
                            def digi_filter2(c):
                                if not getattr(c, 'is_digimon', False):
                                    return False
                                names = getattr(c, 'card_names', []) or []
                                return any('PlatinumNumemon' in n for n in names)
                            game.effect_digivolve_from_hand(
                                player, cur_perm, digi_filter2,
                                cost_override=4,
                                ignore_requirements=True,
                                is_optional=True)

            effect.set_on_process_callback(process)
            return effect

        effects.append(_build_main_effect(is_on_play=True))
        effects.append(_build_main_effect(is_when_digivolving=True))

        # Inherited: [Your Turn] [Once Per Turn] When this Digimon would digivolve
        # into a Digimon card with the [CS] trait, reduce the digivolution cost by 1.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.BeforePayCost)
        effect3.set_effect_name("BT22-031 CS digi cost -1")
        effect3.set_effect_description(
            "[Your Turn] [Once Per Turn] When this Digimon would digivolve into a "
            "Digimon card with the [CS] trait, reduce the digivolution cost by 1."
        )
        effect3.is_inherited_effect = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("CSDigiCostReduce_BT22_031")
        effect3.cost_reduction = 1

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check that the card being digivolved into has CS trait
            digi_card = context.get('card_source')
            if digi_card:
                traits = getattr(digi_card, 'card_traits', []) or []
                if not any('CS' == t for t in traits):
                    return False
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
