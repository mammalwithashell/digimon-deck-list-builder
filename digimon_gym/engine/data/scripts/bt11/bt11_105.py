from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_105(CardScript):
    """BT11-105 Fusionize"""

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

        # ─── Effect 0: Cost reduction — if you have a [Snatchmon] in play,
        # reduce this option's play cost by 1.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("BT11-105 Cost -1 with Snatchmon")
        effect0.set_effect_description(
            "If you have a [Snatchmon] in play, reduce this card's play cost by 1.")
        effect0.cost_reduction = 1

        def condition0(context: Dict[str, Any]) -> bool:
            # Leak guard: only apply when THIS card is being played
            if context.get('card_source') is not card:
                return False
            owner = getattr(card, 'owner', None)
            return _has_snatchmon_in_play(owner)

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Cost reduction handled via cost_reduction property."""
            pass

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ─── Effect 1: [Main] By placing 1 [Vemmon] or [Destromon] from your
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
            """Place 1 Vemmon/Destromon from trash as bottom digi-card, then digivolve from trash into Destromon/Galacticmon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Step 1: Place 1 [Vemmon] or [Destromon] from trash under 1 of
            # your Digimon as bottom digi-card
            qualifying_trash = [c for c in player.trash_cards if _is_vemmon_or_destromon(c)]
            if not qualifying_trash or not player.battle_area:
                return

            # Pick the first qualifying card from trash
            chosen_card = qualifying_trash[0]
            # Pick a Digimon to place it under (first available)
            target_perm = None
            for p in player.battle_area:
                if p.is_digimon:
                    target_perm = p
                    break
            if not target_perm:
                return

            player.trash_cards.remove(chosen_card)
            target_perm.add_card_source_bottom(chosen_card)

            # Step 2: Digivolve 1 of your Digimon into [Destromon] or
            # [Galacticmon] from trash for its digivolution cost.
            # Find qualifying cards in trash
            digi_candidates = [
                c for c in player.trash_cards
                if _is_destromon_or_galacticmon(c) and getattr(c, 'is_digimon', False)
            ]
            if not digi_candidates or not player.battle_area:
                return

            # Pick a Digimon to digivolve (prefer the one we just placed under)
            digi_target_perm = target_perm
            digi_card = digi_candidates[0]

            # Pay digivolution cost and digivolve
            evo_costs = getattr(digi_card, 'evo_costs', []) or []
            cost = evo_costs[0] if evo_costs else getattr(digi_card, 'get_cost_itself', 0)
            if isinstance(cost, dict):
                cost = cost.get('cost', 0)
            if isinstance(cost, property):
                cost = 0

            player.trash_cards.remove(digi_card)
            digi_target_perm.add_card_source(digi_card)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ─── Effect 2: [Security] You may reveal the top 3 cards of your deck.
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
