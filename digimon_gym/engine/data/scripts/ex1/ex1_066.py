from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX1_066(CardScript):
    """EX1-066 Analog Youth | Tamer White

    [On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card
        among them to your hand. Trash the remaining cards.
    [All Turns] When one of your level 5 or higher Digimon with
        digivolution cards is deleted, you may suspend this Tamer. If
        you do, gain 1 memory, then hatch 1 Digi-Egg card to an empty
        space in your Breeding Area.
    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [On Play] Reveal top 3, add 1 Digimon, trash rest ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX1-066 Reveal top 3, add 1 Digimon to hand")
        effect0.set_effect_description(
            "[On Play] Reveal the top 3 cards of your deck. Add 1 Digimon "
            "card among them to your hand. Trash the remaining cards."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Reveal top 3
            count = min(3, len(player.library_cards))
            revealed = player.library_cards[:count]
            if not revealed:
                return

            def digimon_filter(c):
                return getattr(c, 'is_digimon', False)

            has_digimon = any(digimon_filter(c) for c in revealed)

            if has_digimon:
                # Use engine API - select 1 Digimon, remaining get trashed
                def on_selected(selected, remaining):
                    player.hand_cards.append(selected)
                    for c in remaining:
                        player.trash_cards.append(c)

                game.effect_reveal_and_select(
                    player, count, digimon_filter, on_selected,
                    is_optional=False,
                    prompt="Select 1 Digimon card to add to your hand.")
            else:
                # No Digimon among revealed - trash all
                for c in revealed:
                    player.library_cards.remove(c)
                    player.trash_cards.append(c)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [All Turns] On ally Lv5+ deletion, suspend for memory + hatch ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDestroyedAnyone)
        effect1.set_effect_name("EX1-066 On ally Lv5+ deletion: gain memory + hatch")
        effect1.set_effect_description(
            "[All Turns] When one of your level 5 or higher Digimon with "
            "digivolution cards is deleted, you may suspend this Tamer. If "
            "you do, gain 1 memory, then hatch 1 Digi-Egg card to an empty "
            "space in your Breeding Area."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            # Must not already be suspended
            if perm and perm.is_suspended:
                return False
            # The deleted Digimon must be ours, lv5+, with digi cards
            deleted = context.get('event_permanent')
            if deleted is None:
                return False
            event_player = context.get('event_player')
            owner = card.owner if card else None
            if event_player is None or event_player is not owner:
                return False
            if not deleted.is_digimon:
                return False
            if deleted.level is None or deleted.level < 5:
                return False
            if deleted.has_no_digivolution_cards:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game and card):
                return
            perm = card.permanent_of_this_card()
            if not perm:
                return
            # Suspend this Tamer
            perm.suspend()
            # Gain 1 memory
            player.add_memory(1)
            # Hatch: if breeding area is empty and there are eggs
            if player.breeding_area is None and player.digitama_library_cards:
                player.hatch()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Play this card without paying the cost ---
        effect2 = ICardEffect()
        effect2.set_effect_name("EX1-066 Security Effect")
        effect2.set_effect_description("[Security] Play this card without paying the cost.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
