from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_003(CardScript):
    """EX10-003 Tumblemon | Lv.2

    Inherited Effect [Opponent's Turn] [Once Per Turn] When one of your
    opponent's Digimon attacks, by trashing 3 [Mineral] or [Rock] trait
    cards from this Digimon's digivolution cards, end that attack.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Inherited: [Opponent's Turn] [Once Per Turn] When opponent attacks,
        # by trashing 3 [Mineral]/[Rock] sources, end that attack.
        # Uses OnAllyAttack timing to fire during the attacker's declaration window
        # (execute_effects(OnAllyAttack) in combat.py _start_attack scans both
        # players' battle areas, so defender permanents pick this up).
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnAllyAttack)
        effect0.set_effect_name("EX10-003 By trashing 3 Mineral/Rock sources, end attack")
        effect0.set_effect_description(
            "[Opponent's Turn] [Once Per Turn] When one of your opponent's "
            "Digimon attacks, by trashing 3 [Mineral] or [Rock] trait cards "
            "from this Digimon's digivolution cards, end that attack."
        )
        effect0.is_inherited_effect = True
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("ESS_EX10-003")

        def _is_mineral_or_rock(c) -> bool:
            traits = getattr(c, 'card_traits', []) or []
            return 'Mineral' in traits or 'Rock' in traits

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only on opponent's turn
            owner = card.owner if card else None
            if not owner or owner.is_my_turn:
                return False
            # Check this Digimon has at least 3 Mineral/Rock digivolution cards
            # (excluding top card)
            perm = card.permanent_of_this_card()
            if not perm:
                return False
            count = 0
            for c in perm.digivolution_cards:
                if c is perm.top_card:
                    continue
                if _is_mineral_or_rock(c):
                    count += 1
                    if count >= 3:
                        return True
            return False

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Trash 3 Mineral/Rock digivolution cards (player-selected) and end the attack."""
            from ....data.enums import GamePhase
            from ....game.constants import SOURCES_PER_FIELD

            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            # Find field index of this permanent
            field_idx = None
            for fi, fp in enumerate(player.battle_area):
                if fp is perm:
                    field_idx = fi
                    break
            if field_idx is None:
                return

            trashed_cards = []

            def _select_next():
                """Chain selection: pick one Mineral/Rock source at a time."""
                if len(trashed_cards) >= 3:
                    # All 3 trashed — end the attack
                    game.force_end_attack()
                    return

                # Rebuild valid indices (card_sources may have changed)
                base = 2000 + field_idx * SOURCES_PER_FIELD
                top = perm.top_card
                valid = []
                for i, cs in enumerate(perm.card_sources):
                    if cs is top:
                        continue
                    if _is_mineral_or_rock(cs) and (base + i) < 2168:
                        valid.append(base + i)
                if not valid:
                    # Not enough valid cards left — abort (shouldn't happen
                    # given condition checked 3+ exist)
                    return

                def on_source_selected(action_id):
                    idx = action_id - base
                    if not (0 <= idx < len(perm.card_sources)):
                        return
                    selected = perm.card_sources[idx]
                    perm.card_sources.remove(selected)
                    player.trash_cards.append(selected)
                    trashed_cards.append(selected)
                    # Chain to next selection
                    _select_next()

                remaining = 3 - len(trashed_cards)
                game.request_selection(
                    GamePhase.SelectSource, player, on_source_selected,
                    valid, is_optional=False,
                    prompt=f"Select a [Mineral] or [Rock] trait card to trash from digivolution cards ({remaining} remaining).")

            _select_next()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
