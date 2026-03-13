from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT17_078(CardScript):
    """BT17-078 Omnimon | Lv.7 White Digimon

    DNA Digivolution: Lv.6 w/[Greymon] in name + Lv.6 w/[Garurumon] in name
        for 0.
    <Raid> <Blocker>
    [Hand] [Counter] <Blast DNA Digivolve> (WarGreymon + MetalGarurumon)
    [On Play] If DNA digivolving, choose 1 opponent's Digimon and return
        that Digimon and all of their Digimon with the same level to the
        bottom of the deck. Then, delete 1 of their Digimon.
    [When Digivolving] Same as On Play.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Note: DNA digivolution conditions (Lv.6 Greymon + Lv.6 Garurumon)
        # are registered on the card entity via dna_costs in card_database.
        # The script doesn't need to define them.

        # --- Effect 0: <Raid> ---
        effect_raid = ICardEffect()
        effect_raid.set_effect_name("BT17-078 Raid")
        effect_raid.set_effect_description("<Raid>")
        effect_raid._is_raid = True

        def condition_raid(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            return perm is not None
        effect_raid.set_can_use_condition(condition_raid)
        effects.append(effect_raid)

        # --- Effect 1: <Blocker> ---
        effect_blocker = ICardEffect()
        effect_blocker.set_effect_name("BT17-078 Blocker")
        effect_blocker.set_effect_description("<Blocker>")
        effect_blocker._is_blocker = True

        def condition_blocker(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            return perm is not None
        effect_blocker.set_can_use_condition(condition_blocker)
        effects.append(effect_blocker)

        # --- Effect 2: <Blast DNA Digivolve> ---
        effect_blast = ICardEffect()
        effect_blast.set_effect_name("BT17-078 Blast DNA Digivolve")
        effect_blast.set_effect_description(
            "[Hand] [Counter] <Blast DNA Digivolve> (WarGreymon + MetalGarurumon)"
        )
        effect_blast.is_counter_effect = True
        effect_blast._is_blast_digivolve = True
        effect_blast._is_blast_dna = True

        def condition_blast(context: Dict[str, Any]) -> bool:
            return True
        effect_blast.set_can_use_condition(condition_blast)
        effects.append(effect_blast)

        # --- Shared On Play / When Digivolving process ---
        def _process_on_play_digivolve(ctx: Dict[str, Any]):
            """If DNA digivolving: bottom-deck all opponent Digimon of chosen
            level, then delete 1 opponent Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            is_dna = ctx.get('is_dna_digivolve', False)
            enemy = player.enemy
            if not enemy:
                return

            def opp_digimon_filter(p):
                return p.is_digimon

            if is_dna:
                # Step 1: Choose 1 opponent Digimon, bottom deck all same level
                def on_select_level_target(target_perm):
                    if not target_perm or not target_perm.top_card:
                        return
                    target_level = target_perm.level
                    if target_level is None:
                        return
                    # Collect all opponent Digimon with same level
                    to_return = [
                        p for p in list(enemy.battle_area)
                        if p.is_digimon and p.level == target_level
                    ]
                    for p in to_return:
                        enemy.return_permanent_to_deck_bottom(p)

                has_opp_digimon = any(opp_digimon_filter(p) for p in enemy.battle_area)
                if has_opp_digimon:
                    game.effect_select_opponent_permanent(
                        player, on_select_level_target,
                        filter_fn=opp_digimon_filter,
                        is_optional=False)

            # Step 2: Then, delete 1 of their Digimon (happens regardless
            # of DNA status per C# code: the "Then" after the if-block)
            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)

            has_opp_digimon = any(opp_digimon_filter(p) for p in enemy.battle_area)
            if has_opp_digimon:
                game.effect_select_opponent_permanent(
                    player, on_delete,
                    filter_fn=opp_digimon_filter,
                    is_optional=False)

        # --- Effect 3: [On Play] ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT17-078 On Play: bottom deck + delete")
        effect3.set_effect_description(
            "[On Play] If DNA Digivolving, bottom deck opponent Digimon of "
            "same level. Then, delete 1 opponent Digimon."
        )
        effect3.is_on_play = True

        def condition3(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            return perm is not None and perm.is_digimon
        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_process_on_play_digivolve)
        effects.append(effect3)

        # --- Effect 4: [When Digivolving] same as On Play ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT17-078 When Digivolving: bottom deck + delete")
        effect4.set_effect_description(
            "[When Digivolving] If DNA Digivolving, bottom deck opponent "
            "Digimon of same level. Then, delete 1 opponent Digimon."
        )
        effect4.is_when_digivolving = True

        def condition4(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            return perm is not None and perm.is_digimon
        effect4.set_can_use_condition(condition4)
        effect4.set_on_process_callback(_process_on_play_digivolve)
        effects.append(effect4)

        return effects
