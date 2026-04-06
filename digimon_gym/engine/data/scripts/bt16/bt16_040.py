from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, GamePhase

if TYPE_CHECKING:
    from ....core.card_source import CardSource

# Trash selection action-ID offset (mirrors game.py SEL_TRASH_START)
_SEL_TRASH_START = 130


class BT16_040(CardScript):
    """BT16-040 Wormmon | Lv.3 Digimon | Green | DP 2000 | Cost 3

    [Start of Your Main Phase] [On Play] If it's your turn, 1 of your Digimon
    may digivolve into a level 4 Digimon card with the [Insectoid] or [Free]
    trait in your trash with the digivolution cost reduced by 1.
    [Inherited] [When Attacking] [Once Per Turn] Suspend 1 of your opponent's
    Digimon.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Alt-digi: from [Minomon] for cost 0 ─────────────────────────────
        effect0 = ICardEffect()
        effect0.set_effect_name("BT16-040 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 0
        effect0._alt_digi_name = "Minomon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not permanent:
                return False
            return permanent.contains_card_name("Minomon")

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # ── Inherited: [When Attacking] [Once Per Turn] Suspend 1 opponent Digimon ──
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnUseAttack)
        effect1.set_effect_name("BT16-040 Suspend 1 opponent Digimon")
        effect1.set_effect_description(
            "[When Attacking] [Once per turn] Suspend 1 of your opponent's Digimon."
        )
        effect1.is_inherited_effect = True
        effect1.is_on_attack = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Suspend_BT16-040")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            game.effect_select_opponent_permanent(
                player,
                lambda target_perm: target_perm.suspend(),
                filter_fn=lambda p: p.is_digimon,
                is_optional=False,
                prompt="Select 1 of your opponent's Digimon to suspend.",
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ── Shared logic for trash-digivolve ─────────────────────────────────
        def _trash_card_filter(c) -> bool:
            """Level 4 with Insectoid or Free trait."""
            if getattr(c, 'level', None) != 4:
                return False
            # Check Insectoid in type_eng (card_traits)
            traits = getattr(c, 'card_traits', []) or []
            if any('Insectoid' in t for t in traits):
                return True
            # Check Free in attribute_eng (not in card_traits/type_eng)
            attrs = c.c_entity_base.attribute_eng if c.c_entity_base else []
            if 'Free' in attrs:
                return True
            return False

        def _make_trash_digivolve_process():
            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return

                # Step 1: Select which of your Digimon will digivolve.
                def perm_filter(p):
                    if not p.is_digimon:
                        return False
                    # Must have at least one qualifying trash card
                    return any(_trash_card_filter(c) for c in player.trash_cards)

                def on_perm_selected(target_perm):
                    # Step 2: Select the trash card.
                    valid_indices = []
                    for i, c in enumerate(player.trash_cards):
                        if _trash_card_filter(c):
                            action_id = _SEL_TRASH_START + i
                            valid_indices.append(action_id)
                    if not valid_indices:
                        return

                    def on_trash_selected(action_id_raw: int):
                        # Callback receives raw action_id; subtract offset
                        idx = action_id_raw - _SEL_TRASH_START
                        if not (0 <= idx < len(player.trash_cards)):
                            return
                        chosen_card = player.trash_cards[idx]
                        # Remove from trash
                        player.trash_cards.remove(chosen_card)
                        # Calculate digivolution cost with -1 reduction
                        base_cost = getattr(chosen_card, 'get_cost_itself', 0)
                        cost = max(0, base_cost - 1)
                        # Stack onto permanent (digivolve)
                        target_perm.add_card_source(chosen_card)
                        player.lose_memory(cost)
                        game.logger.log(
                            f"[Effect Digivolve] {game._card_ref(chosen_card)} "
                            f"from trash onto {game._perm_ref(target_perm)} "
                            f"(cost: {cost})"
                        )
                        # Draw 1 (digivolution bonus)
                        player.draw()
                        game.execute_effects(
                            EffectTiming.WhenDigivolving,
                            {"digivolved_permanent": target_perm},
                        )

                    game.request_selection(
                        GamePhase.SelectTrash,
                        player,
                        on_trash_selected,
                        valid_indices,
                        is_optional=False,
                        prompt=(
                            "Select a Lv.4 [Insectoid] or [Free] card from "
                            "trash to digivolve with."
                        ),
                    )

                game.effect_select_own_permanent(
                    player,
                    on_perm_selected,
                    filter_fn=perm_filter,
                    is_optional=True,
                    prompt="Select 1 of your Digimon to digivolve from trash.",
                )

            return process

        def _make_trash_digivolve_condition():
            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                owner = card.owner if card else None
                if not owner or not owner.is_my_turn:
                    return False
                has_trash = any(_trash_card_filter(c) for c in owner.trash_cards)
                has_digimon = any(p.is_digimon for p in owner.battle_area)
                return has_trash and has_digimon
            return condition

        # ── [Start of Your Main Phase] ────────────────────────────────────────
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnStartMainPhase)
        effect2.set_effect_name("BT16-040 Digivolve from trash (Start of Main)")
        effect2.set_effect_description(
            "[Start of Main Phase] If it's your turn, 1 of your Digimon may digivolve "
            "into a level 4 Digimon card with the [Insectoid] or [Free] trait from your "
            "trash with the digivolution cost reduced by 1."
        )
        effect2.is_optional = True
        effect2.set_hash_string("Digivolve_BT16_040_SOM")
        effect2.set_can_use_condition(_make_trash_digivolve_condition())
        effect2.set_on_process_callback(_make_trash_digivolve_process())
        effects.append(effect2)

        # ── [On Play] ─────────────────────────────────────────────────────────
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.is_on_play = True
        effect3.set_effect_name("BT16-040 Digivolve from trash (On Play)")
        effect3.set_effect_description(
            "[On Play] If it's your turn, 1 of your Digimon may digivolve "
            "into a level 4 Digimon card with the [Insectoid] or [Free] trait from your "
            "trash with the digivolution cost reduced by 1."
        )
        effect3.is_optional = True
        effect3.set_hash_string("Digivolve_BT16_040_OnPlay")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner or not owner.is_my_turn:
                return False
            has_trash = any(_trash_card_filter(c) for c in owner.trash_cards)
            has_digimon = any(p.is_digimon for p in owner.battle_area)
            return has_trash and has_digimon

        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_make_trash_digivolve_process())
        effects.append(effect3)

        return effects
