from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_007(CardScript):
    """BT13-007 King Drasil_7D6"""

    @staticmethod
    def _has_trait(obj: Any, trait: str) -> bool:
        if not obj:
            return False
        t = trait.lower()
        for attr in ("traits", "type_eng", "types", "card_types"):
            vals = getattr(obj, attr, None)
            if isinstance(vals, list) and any(str(v).lower() == t for v in vals):
                return True
        return t in str(getattr(obj, "name", "")).lower()

    @staticmethod
    def _is_option_card(obj: Any) -> bool:
        if not obj:
            return False
        kind = getattr(obj, "card_kind", None)
        if kind == 2:
            return True
        for attr in ("kind", "type_name", "card_type"):
            val = str(getattr(obj, attr, "")).lower()
            if "option" in val:
                return True
        return False

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Breeding][Your Turn] All of your Digimon can't digivolve.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-007 Your Digimon can't digivolve")
        effect0.set_effect_description("[Breeding][Your Turn] All of your Digimon can't digivolve.")

        def condition0(context: Dict[str, Any]) -> bool:
            return bool(card and card.owner and card.owner.is_my_turn)

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player') or getattr(card, 'owner', None)
            if not player:
                return
            for attr in ("digimons", "digimon_list", "battle_digimons", "field_digimons"):
                digimons = getattr(player, attr, None)
                if isinstance(digimons, list):
                    for d in digimons:
                        try:
                            setattr(d, "cannot_digivolve", True)
                        except Exception:
                            pass
                    break

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [Breeding][Your Turn][Once Per Turn] Royal Knight Digimon play cost reduction.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-007 Royal Knight play cost reduction")
        effect1.set_effect_description("[Breeding][Your Turn][Once Per Turn] When a [Royal Knight] trait Digimon card would be played, you may reduce the play cost by 4. Further reduce it by 1 for each of this Digimon's digivolution cards.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("PlayCostReduction_BT13_007")

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            target = context.get("play_card") or context.get("card") or context.get("target_card")
            if not target:
                return False
            if getattr(target, "level", None) is None:
                return False
            return self._has_trait(target, "Royal Knight")

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            source_perm = ctx.get("permanent") or ctx.get("source")
            evo_cards = []
            if source_perm is not None:
                evo_cards = getattr(source_perm, "digivolution_cards", None) or getattr(source_perm, "evolution_cards", None) or []
            reduction = 4 + len(evo_cards)
            ctx["play_cost_reduction"] = ctx.get("play_cost_reduction", 0) + reduction
            if "play_cost" in ctx and isinstance(ctx.get("play_cost"), int):
                ctx["play_cost"] = max(0, ctx["play_cost"] - reduction)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [Breeding][Start of Your Main Phase] place top Digi-Egg and all your Royal Knights as bottom digivolution cards.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT13-007 Place Digi-Egg and Royal Knights under this Digimon")
        effect2.set_effect_description("[Breeding][Start of Your Main Phase] Reveal the top card of your Digi-Egg deck, then place that card and all of your [Royal Knight] trait Digimon as this Digimon's bottom digivolution cards.")

        def condition2(context: Dict[str, Any]) -> bool:
            return bool(card and card.owner and card.owner.is_my_turn)

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player') or getattr(card, 'owner', None)
            perm = ctx.get('permanent')
            if not player or not perm:
                return

            digi_egg_deck = getattr(player, "digi_egg_deck", None) or getattr(player, "egg_deck", None)
            if isinstance(digi_egg_deck, list) and digi_egg_deck:
                top = digi_egg_deck.pop(0)
                bottom = getattr(perm, "digivolution_cards", None)
                if isinstance(bottom, list):
                    bottom.append(top)
                else:
                    setattr(perm, "digivolution_cards", [top])

            board = None
            board_attr = None
            for attr in ("digimons", "digimon_list", "battle_digimons", "field_digimons"):
                val = getattr(player, attr, None)
                if isinstance(val, list):
                    board = val
                    board_attr = attr
                    break
            if not isinstance(board, list):
                return

            to_move = [d for d in list(board) if d is not perm and self._has_trait(d, "Royal Knight")]
            for d in to_move:
                try:
                    board.remove(d)
                except ValueError:
                    continue
                evo = getattr(perm, "digivolution_cards", None)
                if isinstance(evo, list):
                    evo.append(d)
                else:
                    setattr(perm, "digivolution_cards", [d])
            if board_attr:
                setattr(player, board_attr, board)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # [Breeding][Your Turn][Once Per Turn] When a Royal Knight Option is placed in battle area, gain 1 memory.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT13-007 Memory +1")
        effect3.set_effect_description("[Breeding][Your Turn][Once Per Turn] When an Option card with the [Royal Knight] trait is placed in the battle area, gain 1 memory.")
        effect3.is_inherited_effect = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("Memory+1_BT13_007")
        effect3.is_on_play = True

        def condition3(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            placed = context.get("played_card") or context.get("card") or context.get("target_card")
            if not placed:
                return False
            location = str(context.get("to_zone", context.get("zone", ""))).lower()
            if location and "battle" not in location:
                return False
            return self._is_option_card(placed) and self._has_trait(placed, "Royal Knight")

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player') or getattr(card, 'owner', None)
            if player:
                player.add_memory(1)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
