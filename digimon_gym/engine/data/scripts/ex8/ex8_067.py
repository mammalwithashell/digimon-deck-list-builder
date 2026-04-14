from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_067(CardScript):
    """EX8-067 Close | Tamer"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Start of Your Turn] If you have 2 or less memory, set it to 3.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartTurn)
        effect0.set_effect_name("EX8-067 Set memory to 3 if 2 or less")
        effect0.set_effect_description("[Start of Your Turn] If you have 2 or less memory, set it to 3.")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner or not owner.is_my_turn:
                return False
            return owner.memory <= 2

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player and player.memory <= 2:
                player.add_memory(3 - player.memory)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [Your Turn] When any of your Digimon digivolve into a [Mineral] or [Rock] trait Digimon,
        # by suspending this Tamer, place up to 2 cards with the [Mineral] or [Rock] trait from
        # your trash as that Digimon's bottom digivolution cards.
        #
        # Uses WhenDigivolving timing without is_when_digivolving flag so it fires from all
        # permanents (including this tamer). digivolved_permanent is available in context.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.WhenDigivolving)
        effect1.set_effect_name("EX8-067 Suspend tamer to place up to 2 Mineral/Rock sources")
        effect1.set_effect_description(
            "[Your Turn] When any of your Digimon digivolve into a [Mineral] or [Rock] trait "
            "Digimon, by suspending this Tamer, place up to 2 cards with the [Mineral] or [Rock] "
            "trait from your trash as that Digimon's bottom digivolution cards."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner or not owner.is_my_turn:
                return False
            # Tamer must not already be suspended (it will be suspended as cost)
            tamer_perm = card.permanent_of_this_card()
            if tamer_perm and tamer_perm.is_suspended:
                return False
            # The digivolving permanent must exist and have [Mineral] or [Rock] trait
            digivolved_perm = context.get('digivolved_permanent')
            if digivolved_perm is None:
                return False
            if not (digivolved_perm.has_trait('Mineral') or digivolved_perm.has_trait('Rock')):
                return False
            # Must own the digivolving permanent
            if digivolved_perm not in owner.battle_area:
                return False
            # Must have at least 1 Mineral/Rock card in trash to place
            return any(
                'Mineral' in getattr(c, 'card_traits', []) or 'Rock' in getattr(c, 'card_traits', [])
                for c in owner.trash_cards
            )

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            digivolved_perm = ctx.get('digivolved_permanent')
            if not (player and game and digivolved_perm):
                return
            # Cost: suspend this tamer
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return
            tamer_perm.suspend()

            # Place up to 2 Mineral/Rock cards from trash as bottom digivolution cards
            # Uses recursive request_selection pattern for player-driven choice
            def _mineral_rock_filter(c):
                traits = getattr(c, 'card_traits', [])
                return 'Mineral' in traits or 'Rock' in traits

            placed_holder = [0]

            def _place_one():
                if placed_holder[0] >= 2:
                    return
                eligible = [c for c in player.trash_cards if _mineral_rock_filter(c)]
                if not eligible:
                    return

                from ....data.enums import GamePhase
                from ....game.constants import SEL_TRASH_START
                valid_trash = []
                for i, c in enumerate(player.trash_cards):
                    if _mineral_rock_filter(c):
                        valid_trash.append(SEL_TRASH_START + i)
                if not valid_trash:
                    return

                def on_trash_selected(action_id):
                    idx = action_id - SEL_TRASH_START
                    if 0 <= idx < len(player.trash_cards):
                        selected = player.trash_cards[idx]
                        player.trash_cards.remove(selected)
                        digivolved_perm.add_card_source_bottom(selected)
                        placed_holder[0] += 1
                        _place_one()  # recurse for next selection

                game.request_selection(
                    GamePhase.SelectTrash, player, on_trash_selected,
                    valid_trash, is_optional=True,
                    prompt=f"Select a [Mineral] or [Rock] card from trash to place as bottom digivolution card ({placed_holder[0]+1}/2).")

            _place_one()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Security: Play this card without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("EX8-067 Security: Play this card")
        effect2.set_effect_description("Security: Play this card without paying the cost.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
