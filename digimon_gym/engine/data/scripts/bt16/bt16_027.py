from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_027(CardScript):
    """BT16-027 Imperialdramon: Fighter Mode | Lv.6 Blue/Green ACE

    [Hand] [Counter] <Blast Digivolve>
    [On Play] Return 1 of your opponent's Digimon with as many or fewer
        digivolution cards as this Digimon to the bottom of the deck.
    [When Digivolving] Same as On Play.
    [End of Attack] [Once Per Turn] Unsuspend this Digimon. Then, if
        [Imperialdramon: Dragon Mode] is in this Digimon's digivolution cards,
        return 1 of your opponent's suspended Digimon to the bottom of the deck.
    Inherited: Ace Overflow <-4>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alternate digivolution from [Imperialdramon: Dragon Mode] ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT16-027 Alternate digivolution requirement")
        effect0.set_effect_description(
            "Alternate digivolution: from [Imperialdramon: Dragon Mode] for cost 2"
        )
        effect0._alt_digi_cost = 2
        effect0._alt_digi_name = "Imperialdramon: Dragon Mode"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.contains_card_name('Imperialdramon: Dragon Mode')):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: Blast Digivolve ---
        effect1 = ICardEffect()
        effect1.set_effect_name("BT16-027 Blast Digivolve")
        effect1.set_effect_description("[Hand] [Counter] <Blast Digivolve>")
        effect1.is_counter_effect = True
        effect1._is_blast_digivolve = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Shared helper: return 1 opponent Digimon with <= this Digimon's digi-card count to deck bottom ---
        def _bottom_deck_opponent_by_digi_count(ctx: Dict[str, Any]):
            """Select 1 opponent Digimon with DigivolutionCards.Count <= this Digimon's, return to deck bottom."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return
            # DigivolutionCards in DCGO = cards under the top card = card_sources[:-1]
            my_digi_count = max(len(perm.card_sources) - 1, 0)

            def target_filter(p):
                if not p.is_digimon:
                    return False
                opp_digi_count = max(len(p.card_sources) - 1, 0)
                return opp_digi_count <= my_digi_count

            def on_bottom_deck(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.return_permanent_to_deck_bottom(target_perm)

            game.effect_select_opponent_permanent(
                player, on_bottom_deck,
                filter_fn=target_filter,
                is_optional=False
            )

        # --- Effect 2: [On Play] Bottom deck opponent Digimon ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT16-027 On Play: Bottom deck opponent Digimon")
        effect2.set_effect_description(
            "[On Play] Return 1 of your opponent's Digimon with as many or fewer "
            "digivolution cards as this Digimon to the bottom of the deck."
        )
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_bottom_deck_opponent_by_digi_count)
        effects.append(effect2)

        # --- Effect 3: [When Digivolving] Bottom deck opponent Digimon ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT16-027 When Digivolving: Bottom deck opponent Digimon")
        effect3.set_effect_description(
            "[When Digivolving] Return 1 of your opponent's Digimon with as many or fewer "
            "digivolution cards as this Digimon to the bottom of the deck."
        )
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_bottom_deck_opponent_by_digi_count)
        effects.append(effect3)

        # --- Effect 4: [End of Attack] [Once Per Turn] Unsuspend self, then conditional bottom deck ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEndAttack)
        effect4.set_effect_name(
            "BT16-027 End of Attack: Unsuspend / Bottom deck suspended Digimon"
        )
        effect4.set_effect_description(
            "[End of Attack] [Once Per Turn] Unsuspend this Digimon. Then, if "
            "[Imperialdramon: Dragon Mode] is in this Digimon's digivolution cards, "
            "return 1 of your opponent's suspended Digimon to the bottom of the deck."
        )
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("EndofAttack_BT16_027")

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Step 1: Unsuspend this Digimon.
               Step 2: If [Imperialdramon: Dragon Mode] is in digi-sources, bottom deck 1 suspended opp Digimon.
            """
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return
            # Step 1: Unsuspend this Digimon
            perm.unsuspend()
            # Step 2: Check if [Imperialdramon: Dragon Mode] is among this Digimon's digivolution cards
            # Digivolution cards = card_sources[:-1] (everything under the top card)
            has_dragon_mode = any(
                src.contains_card_name('Imperialdramon: Dragon Mode') or
                src.contains_card_name('Imperialdramon Dragon Mode')
                for src in perm.card_sources[:-1]
            ) if perm.card_sources and len(perm.card_sources) > 1 else False
            if has_dragon_mode:
                def suspended_filter(p):
                    return p.is_digimon and p.is_suspended

                def on_bottom_deck(target_perm):
                    enemy = player.enemy if player else None
                    if enemy:
                        enemy.return_permanent_to_deck_bottom(target_perm)

                game.effect_select_opponent_permanent(
                    player, on_bottom_deck,
                    filter_fn=suspended_filter,
                    is_optional=False
                )

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # --- Inherited: Ace Overflow <-4> ---
        # Engine handles ACE Overflow automatically via is_ace / ace_overflow_cost on the
        # card entity base. This inherited effect is descriptive — no process callback needed.
        effect_ace = ICardEffect()
        effect_ace.set_effect_name("BT16-027 Ace Overflow <-4>")
        effect_ace.set_effect_description("Ace Overflow <-4>")
        effect_ace.is_inherited_effect = True

        def condition_ace(context: Dict[str, Any]) -> bool:
            return True
        effect_ace.set_can_use_condition(condition_ace)
        effects.append(effect_ace)

        return effects
