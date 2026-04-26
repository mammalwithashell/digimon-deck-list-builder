from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX4_051(CardScript):
    """EX4-051 BlitzGreymon | Lv.6 Black/Red | Cyborg | 12000 DP | Cost 12

    [When Digivolving] Activate 1 of the effects below:
        - De-Digivolve 1 3 of your opponent's Digimon.
        - Digivolve 1 of your other Digimon into a level 6 or lower Digimon
          card with [Garurumon] in its name in your hand without paying its cost.
        - This Digimon and another of your Digimon may DNA digivolve into
          a Digimon card in the hand.
    Inherited: [When Attacking] [Once Per Turn] If this Digimon has [Omnimon]
        in its name, trash the top card of your opponent's security stack.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [When Digivolving] Choose 1 of 3 effects ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX4-051 When Digivolving: Choose 1 of 3 effects")
        effect0.set_effect_description(
            "[When Digivolving] Activate 1 of the effects below: "
            "- De-Digivolve 1 3 of your opponent's Digimon. "
            "- Digivolve 1 of your other Digimon into a level 6 or lower "
            "Digimon card with [Garurumon] in its name in your hand without "
            "paying its cost. "
            "- This Digimon and another of your Digimon may DNA digivolve "
            "into a Digimon card in the hand."
        )
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Choose and activate 1 of 3 effects."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return
            enemy = player.enemy
            if not enemy:
                return

            def on_branch(choice: int):
                if choice == 0:
                    # Branch 0: De-Digivolve 1 three of opponent's Digimon
                    opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
                    if not opp_digimon:
                        return

                    if len(opp_digimon) <= 3:
                        # 3 or fewer: de-digivolve all of them
                        for target_perm in list(opp_digimon):
                            if target_perm in enemy.battle_area:
                                removed = target_perm.de_digivolve(1)
                                for rc in removed:
                                    enemy.trash_cards.append(rc)
                    else:
                        # More than 3: player must select 3 targets
                        remaining = [0]  # mutable counter

                        def _select_next_target(already_selected):
                            if remaining[0] >= 3:
                                return

                            def target_filter(p):
                                return (p.is_digimon
                                        and p in enemy.battle_area
                                        and p not in already_selected)

                            def on_target(target_perm):
                                already_selected.append(target_perm)
                                removed = target_perm.de_digivolve(1)
                                for rc in removed:
                                    enemy.trash_cards.append(rc)
                                remaining[0] += 1
                                _select_next_target(already_selected)

                            game.effect_select_opponent_permanent(
                                player, on_target,
                                filter_fn=target_filter,
                                is_optional=False,
                                prompt="Select an opponent's Digimon to De-Digivolve 1.")

                        _select_next_target([])

                elif choice == 1:
                    # Branch 1: Digivolve 1 of your other Digimon into a
                    # Lv.6 or lower [Garurumon] from hand without paying cost
                    own_perm = card.permanent_of_this_card() if card else None

                    def garurumon_filter(c):
                        if not getattr(c, 'is_digimon', False):
                            return False
                        level = getattr(c, 'level', None)
                        if level is None or level > 6:
                            return False
                        names = getattr(c, 'card_names', []) or []
                        return any('Garurumon' in n for n in names)

                    def other_filter(p):
                        return p.is_digimon and p is not own_perm

                    def on_target_selected(target_perm):
                        game.effect_digivolve_from_hand(
                            player, target_perm, garurumon_filter,
                            cost_override=0, ignore_requirements=True,
                            is_optional=True)

                    game.effect_select_own_permanent(
                        player, on_target_selected,
                        filter_fn=other_filter,
                        is_optional=False,
                        prompt="Select 1 of your other Digimon to digivolve into Garurumon.")

                elif choice == 2:
                    # Branch 2: DNA digivolve this Digimon + another into
                    # a Digimon card in hand
                    def dna_filter(c):
                        if not getattr(c, 'is_digimon', False):
                            return False
                        entity = getattr(c, 'c_entity_base', None)
                        if entity and entity.dna_costs:
                            return True
                        return False

                    game.effect_dna_digivolve_from_hand(
                        player, dna_filter, is_optional=True)

            game.effect_choose_branch(
                player, 3,
                callback=on_branch,
                branch_labels=[
                    "De-Digivolve 1 up to 3 opponent Digimon",
                    "Digivolve other Digimon into Garurumon",
                    "DNA Digivolve from hand",
                ])

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Inherited [When Attacking] [Once Per Turn] ---
        # If this Digimon has [Omnimon] in its name, trash top security.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnUseAttack)
        effect1.set_effect_name("EX4-051 Inherited: Trash security if Omnimon")
        effect1.set_effect_description(
            "[When Attacking] [Once Per Turn] If this Digimon has [Omnimon] "
            "in its name, trash the top card of your opponent's security stack."
        )
        effect1.is_inherited_effect = True
        effect1.is_on_attack = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("TrashSecurity_EX4_051")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card() if card else None
            ctx_perm = context.get('attacker') or context.get('permanent')
            if perm and ctx_perm and ctx_perm is not perm:
                return False
            # Check if this Digimon has Omnimon in its name
            if perm and perm.top_card:
                return perm.contains_card_name('Omnimon')
            return False
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Trash the top card of opponent's security stack."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            if enemy.security_cards:
                top_sec = enemy.security_cards.pop(0)
                enemy.trash_cards.append(top_sec)
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
