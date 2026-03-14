from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_105(CardScript):
    """BT11-105 Fusionize

    [Main] By placing 1 [Vemmon] or [Destromon] from your trash under 1 of
    your Digimon as its bottom digivolution card, you may digivolve 1 of your
    Digimon into 1 [Destromon] or [Galacticmon] from your trash for its
    digivolution cost.

    [Security] You may reveal the top 3 cards of your deck. Play 1 [Vemmon]
    among them without paying the cost. Trash the rest.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _is_vemmon_name(c) -> bool:
            return any('Vemmon' in n for n in getattr(c, 'card_names', []))

        def _is_vemmon_or_destromon(c) -> bool:
            names = getattr(c, 'card_names', [])
            return any('Vemmon' in n or 'Destromon' in n for n in names)

        def _is_destromon_or_galacticmon(c) -> bool:
            names = getattr(c, 'card_names', [])
            return any('Destromon' in n or 'Galacticmon' in n for n in names)

        def _has_snatchmon_in_play(owner) -> bool:
            if not owner:
                return False
            for p in owner.battle_area:
                if p.contains_card_name('Snatchmon'):
                    return True
            return False

        # --- Effect 0: Cost reduction --- if you have a [Snatchmon] in play,
        # reduce this option's play cost by 1.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("BT11-105 Cost -1 with Snatchmon")
        effect0.set_effect_description(
            "If you have a [Snatchmon] in play, reduce this card's play cost by 1.")
        effect0.cost_reduction = 1

        def condition0(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            owner = getattr(card, 'owner', None)
            return _has_snatchmon_in_play(owner)

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            pass

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Main] By placing 1 [Vemmon] or [Destromon] from your
        # trash under 1 of your Digimon as its bottom digivolution card, you may
        # digivolve 1 of your Digimon into 1 [Destromon] or [Galacticmon] from
        # your trash for its digivolution cost.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("BT11-105 Place from trash + Digivolve from trash")
        effect1.set_effect_description(
            "[Main] By placing 1 [Vemmon] or [Destromon] from your trash under "
            "1 of your Digimon as its bottom digivolution card, you may digivolve "
            "1 of your Digimon into 1 [Destromon] or [Galacticmon] from your "
            "trash for its digivolution cost.")

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Place 1 Vemmon/Destromon from trash as bottom digi-card, then digivolve from trash."""
            from ....data.enums import GamePhase
            from ....game.constants import SEL_TRASH_START

            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Step 1: Select and place 1 [Vemmon] or [Destromon] from trash under
            # 1 of your Digimon as bottom digi-card.
            qualifying_indices = [
                SEL_TRASH_START + i
                for i, c in enumerate(player.trash_cards)
                if _is_vemmon_or_destromon(c)
            ]
            if not qualifying_indices or not player.battle_area:
                return

            def perm_filter(p):
                return p.is_digimon and not getattr(p, 'is_token', False)

            def on_trash_selected(action_id: int):
                idx = action_id - SEL_TRASH_START
                if not (0 <= idx < len(player.trash_cards)):
                    return
                chosen_card = player.trash_cards[idx]
                player.trash_cards.remove(chosen_card)

                # Select a Digimon to place it under
                def on_perm_selected(target_perm):
                    if target_perm is None:
                        return
                    target_perm.add_card_source_bottom(chosen_card)

                    # Step 2: "you may" digivolve 1 of your Digimon into
                    # [Destromon] or [Galacticmon] from trash for its digivolution cost.
                    digi_indices = [
                        SEL_TRASH_START + i
                        for i, c in enumerate(player.trash_cards)
                        if _is_destromon_or_galacticmon(c) and getattr(c, 'is_digimon', False)
                    ]
                    if not digi_indices or not player.battle_area:
                        return

                    def on_digi_trash_selected(action_id2: int):
                        idx2 = action_id2 - SEL_TRASH_START
                        if not (0 <= idx2 < len(player.trash_cards)):
                            return
                        digi_card = player.trash_cards[idx2]

                        # Find digivolution cost
                        evo_costs = getattr(digi_card, 'evo_costs', []) or []
                        cost = 0
                        if evo_costs:
                            first_evo = evo_costs[0]
                            if hasattr(first_evo, 'memory_cost'):
                                cost = first_evo.memory_cost
                            elif isinstance(first_evo, dict):
                                cost = first_evo.get('cost', first_evo.get('memory_cost', 0))
                            elif isinstance(first_evo, int):
                                cost = first_evo

                        player.trash_cards.remove(digi_card)

                        # Select which Digimon to digivolve (player choice)
                        def on_digi_target(digi_target_perm):
                            if digi_target_perm is None:
                                return
                            # Pay digivolution cost (deduct from memory)
                            if cost > 0:
                                player.lose_memory(cost)
                            digi_target_perm.add_card_source(digi_card)

                        game.effect_select_own_permanent(
                            player, on_digi_target, filter_fn=perm_filter,
                            is_optional=True,
                            prompt="Select 1 of your Digimon to digivolve into [Destromon] or [Galacticmon] from trash.")

                    game.request_selection(
                        GamePhase.SelectTrash, player, on_digi_trash_selected,
                        digi_indices, is_optional=True,
                        prompt="Select 1 [Destromon] or [Galacticmon] from trash to digivolve into.")

                game.effect_select_own_permanent(
                    player, on_perm_selected, filter_fn=perm_filter,
                    is_optional=True,
                    prompt="Select 1 of your Digimon to place a [Vemmon]/[Destromon] from trash under as bottom digi-card.")

            game.request_selection(
                GamePhase.SelectTrash, player, on_trash_selected,
                qualifying_indices, is_optional=False,
                prompt="Select 1 [Vemmon] or [Destromon] from trash to place under a Digimon.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] You may reveal the top 3 cards of your deck.
        # Play 1 [Vemmon] among them without paying the cost. Trash the rest.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("BT11-105 Security: Reveal top 3, play 1 Vemmon free")
        effect2.set_effect_description(
            "[Security] You may reveal the top 3 cards of your deck. Play 1 "
            "[Vemmon] among them without paying the cost. Trash the rest.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """[Security] Reveal top 3, play 1 [Vemmon] free, trash the rest."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Reveal top 3 cards
            revealed = player.library_cards[:3]
            if not revealed:
                return
            for c in revealed:
                player.library_cards.remove(c)

            pool = list(revealed)

            # Find [Vemmon] among revealed
            vemmon_candidates = [c for c in pool if _is_vemmon_name(c)]
            if vemmon_candidates:
                # Play 1 [Vemmon] without paying cost
                chosen = vemmon_candidates[0]
                pool.remove(chosen)
                player.play_card_from_source(chosen, pay_cost=False)

            # Trash the rest
            for c in pool:
                player.trash_cards.append(c)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
