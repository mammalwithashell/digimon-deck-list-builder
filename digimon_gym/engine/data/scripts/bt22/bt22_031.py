from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_031(CardScript):
    """BT22-031 GoldNumemon | Lv.4

    [On Play] [When Digivolving] Give 1 of your opponent's Digimon
    <Security A. -2> until their turn ends. Then, if this Digimon's stack
    has 2 or more same-level cards, this Digimon may digivolve into
    [PlatinumNumemon] in the hand for a digivolution cost of 4, ignoring
    digivolution requirements.

    Inherited: [Your Turn] [Once Per Turn] When this Digimon would digivolve
    into a Digimon card with the [CS] trait, reduce the digivolution cost by 1.

    Alt-digivolve (DCGO C# BT22_031):
        (Lv.4 w/ [Numemon] in name) OR (Lv.3 w/ [CS] trait) for cost 2.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Alt-digi 0a: Lv.4 Numemon → cost 2 ---
        effect0a = ICardEffect()
        effect0a.set_effect_name("BT22-031 Alt digi from Lv.4 [Numemon]")
        effect0a.set_effect_description(
            "Alternate digivolution: from Lv.4 w/ [Numemon] in name for cost 2")
        effect0a._alt_digi_cost = 2
        effect0a._alt_digi_level = 4
        effect0a._alt_digi_name = "Numemon"

        def condition0a(context: Dict[str, Any]) -> bool:
            return True
        effect0a.set_can_use_condition(condition0a)
        effects.append(effect0a)

        # --- Alt-digi 0b: Lv.3 w/ [CS] trait → cost 2 ---
        effect0b = ICardEffect()
        effect0b.set_effect_name("BT22-031 Alt digi from Lv.3 [CS]")
        effect0b.set_effect_description(
            "Alternate digivolution: from Lv.3 w/ [CS] trait for cost 2")
        effect0b._alt_digi_cost = 2
        effect0b._alt_digi_level = 3
        effect0b._alt_digi_trait = "CS"

        def condition0b(context: Dict[str, Any]) -> bool:
            return True
        effect0b.set_can_use_condition(condition0b)
        effects.append(effect0b)

        # --- Main effect factory: [On Play] and [When Digivolving] ---
        def _build_main_effect(is_on_play: bool = False,
                               is_when_digivolving: bool = False) -> ICardEffect:
            effect = ICardEffect()
            effect.set_timing(EffectTiming.OnEnterFieldAnyone)
            trigger = "[On Play]" if is_on_play else "[When Digivolving]"
            effect.set_effect_name(
                f"BT22-031 {trigger} SA-2, then may digivolve into PlatinumNumemon")
            effect.set_effect_description(
                f"{trigger} Give 1 of your opponent's Digimon <Security A. -2> "
                "(This Digimon checks 2 additional security cards.) until their "
                "turn ends. Then, if this Digimon's stack has 2 or more same-"
                "level cards, this Digimon may digivolve into [PlatinumNumemon] "
                "in the hand for a digivolution cost of 4, ignoring digivolution "
                "requirements."
            )
            effect.is_on_play = is_on_play
            effect.is_when_digivolving = is_when_digivolving

            def condition(context: Dict[str, Any]) -> bool:
                # Must be on field to trigger (On Play / When Digivolving)
                if card and card.permanent_of_this_card() is None:
                    return False
                return True

            effect.set_can_use_condition(condition)

            def _has_same_level_pair(perm) -> bool:
                """Match C#: StackCards.Filter(!IsFlipped).GroupBy(Level).Any(Count>=2)"""
                from collections import Counter
                levels = []
                for cs in perm.card_sources:
                    if getattr(cs, 'is_flipped', False):
                        continue
                    lvl = getattr(cs, 'level', None)
                    if lvl is None:
                        continue
                    levels.append(lvl)
                counts = Counter(levels)
                return any(c >= 2 for c in counts.values())

            def _platinum_filter(c) -> bool:
                if not getattr(c, 'is_digimon', False):
                    return False
                names = getattr(c, 'card_names', []) or []
                return any('PlatinumNumemon' in n for n in names)

            def _maybe_digivolve_platinum(player, game, cur_perm):
                """If stack has a same-level pair, optionally digivolve into
                [PlatinumNumemon] from hand for cost 4, ignoring requirements."""
                if not _has_same_level_pair(cur_perm):
                    return
                # Only offer if player actually has a PlatinumNumemon in hand
                if not any(_platinum_filter(c) for c in player.hand_cards):
                    return
                game.effect_digivolve_from_hand(
                    player, cur_perm, _platinum_filter,
                    cost_override=4,
                    ignore_requirements=True,
                    is_optional=True,
                    prompt="You may digivolve into [PlatinumNumemon] for cost 4.",
                )

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                perm = ctx.get('permanent')
                game = ctx.get('game')
                if not (player and perm and game):
                    return

                from ....interfaces.modifiers import ModifierType

                cur_perm = card.permanent_of_this_card() if card else perm

                # Step 1: Give 1 opponent Digimon <Security A. -2> until their
                # turn ends. Selection is MANDATORY if a valid target exists;
                # only skip if no opponent Digimon exists (per C# HasMatch guard).
                enemy = player.enemy
                opp_digimon = (
                    [p for p in enemy.battle_area if p.is_digimon]
                    if enemy else []
                )

                def _register_sa_minus_2(target_perm):
                    # Target-equality guard so the modifier only applies to THIS
                    # specific permanent (otherwise CHANGE_SECURITY_ATTACK
                    # with no condition applies to every queried permanent).
                    game.register_modifier(
                        target_perm,
                        ModifierType.CHANGE_SECURITY_ATTACK,
                        condition=lambda t, c, _tp=target_perm: t is _tp,
                        value_fn=lambda cur, t, c: cur - 2,
                        expiry='end_of_opponent_turn',
                    )

                def _on_sa_target(target_perm):
                    _register_sa_minus_2(target_perm)
                    # Step 2: Same-level check + optional PlatinumNumemon digivolve
                    if cur_perm is not None:
                        _maybe_digivolve_platinum(player, game, cur_perm)

                if opp_digimon:
                    game.effect_select_opponent_permanent(
                        player, _on_sa_target,
                        filter_fn=lambda p: p.is_digimon,
                        is_optional=False,
                        prompt="Select 1 opponent Digimon to give <Security A. -2>.",
                    )
                else:
                    # No opponent Digimon — skip SA selection, still check the
                    # "Then" clause (same-level / PlatinumNumemon digivolve).
                    if cur_perm is not None:
                        _maybe_digivolve_platinum(player, game, cur_perm)

            effect.set_on_process_callback(process)
            return effect

        effects.append(_build_main_effect(is_on_play=True))
        effects.append(_build_main_effect(is_when_digivolving=True))

        # --- Inherited: [Your Turn] [Once Per Turn] When this Digimon would
        #     digivolve into a Digimon card with the [CS] trait, reduce the
        #     digivolution cost by 1.
        #
        #     Uses WhenWouldDigivolve timing; Player.digivolve() scans
        #     inherited effects in the stack and applies cost_reduction.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.WhenWouldDigivolve)
        effect3.set_effect_name("BT22-031 Inherited CS digi cost -1")
        effect3.set_effect_description(
            "[Your Turn] [Once Per Turn] When this Digimon would digivolve "
            "into a Digimon card with the [CS] trait, reduce the digivolution "
            "cost by 1."
        )
        effect3.is_inherited_effect = True
        effect3.cost_reduction = 1
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("BT22_031_InhCSDigiCost")

        def condition3(context: Dict[str, Any]) -> bool:
            # Must be in a stack on the field
            if card and card.permanent_of_this_card() is None:
                return False
            # Must be owner's turn
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # The card being digivolved into must have [CS] trait
            digi_card = context.get('card_source')
            if digi_card is None:
                return False
            traits = getattr(digi_card, 'card_traits', []) or []
            if not any(t == 'CS' for t in traits):
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
