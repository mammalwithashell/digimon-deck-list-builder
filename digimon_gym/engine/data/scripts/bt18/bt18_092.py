from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT18_092(CardScript):
    """BT18-092 Zenith | Tamer | Black | Cost 4

    [Start of Your Main Phase] By trashing 1 [Vemmon] in your hand, Draw 1
    and gain 1 memory.

    [Your Turn] When any of your Digimon attack, by suspending this Tamer and
    returning 2 [Vemmon] from that Digimon's digivolution cards to the bottom
    of the deck, De-Digivolve 1 1 of your opponent's Digimon.

    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _is_vemmon_name(c) -> bool:
            return any('Vemmon' in n for n in getattr(c, 'card_names', []))

        # ─── Effect 0: [Start of Your Main Phase] Trash 1 [Vemmon] from hand →
        #     Draw 1, gain 1 memory.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("BT18-092 Trash Vemmon from hand, Draw 1, Memory +1")
        effect0.set_effect_description(
            "[Start of Your Main Phase] By trashing 1 [Vemmon] in your hand, "
            "Draw 1 and gain 1 memory.")
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            # This tamer must be on the field
            if card and card.permanent_of_this_card() is None:
                return False
            # Must be your turn
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Must have a [Vemmon] in hand
            owner = card.owner
            return any(_is_vemmon_name(c) for c in owner.hand_cards)

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def vemmon_filter(c):
                return _is_vemmon_name(c)

            def on_select(selected):
                # Trash the selected Vemmon from hand
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
                # Draw 1 and gain 1 memory
                player.draw()
                player.add_memory(1)

            game.effect_select_hand_card(
                player, vemmon_filter, on_select,
                is_optional=True,
                prompt="Select 1 [Vemmon] from your hand to trash.")

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ─── Effect 1: [Your Turn] When any of your Digimon attack, by suspending
        #     this Tamer and returning 2 [Vemmon] from that Digimon's digi-cards
        #     to deck bottom, De-Digivolve 1 an opponent's Digimon.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDeclaration)
        effect1.set_effect_name("BT18-092 Suspend + return 2 Vemmon from attacker → De-Digivolve 1")
        effect1.set_effect_description(
            "[Your Turn] When any of your Digimon attack, by suspending this Tamer "
            "and returning 2 [Vemmon] from that Digimon's digivolution cards to the "
            "bottom of the deck, De-Digivolve 1 1 of your opponent's Digimon.")
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            # This tamer must be on the field
            if card and card.permanent_of_this_card() is None:
                return False
            # Must be your turn
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # This tamer must not be suspended
            tamer_perm = card.permanent_of_this_card()
            if tamer_perm is None or tamer_perm.is_suspended:
                return False
            # The attacker must be your Digimon
            attacker = context.get('permanent')
            if attacker is None or not attacker.is_digimon:
                return False
            owner = card.owner
            if attacker not in owner.battle_area:
                return False
            # Attacker must have 2+ [Vemmon] in its digi-stack (card_sources excluding top)
            vemmon_in_stack = [
                cs for cs in attacker.card_sources[:-1]
                if _is_vemmon_name(cs)
            ]
            if len(vemmon_in_stack) < 2:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            attacker = ctx.get('permanent')
            if not (player and game and attacker):
                return

            # Suspend this Tamer (cost)
            tamer_perm = card.permanent_of_this_card()
            if tamer_perm:
                tamer_perm.suspend()

            # Return 2 [Vemmon] from attacker's card_sources to deck bottom
            # card_sources is bottom-to-top; exclude top card
            vemmon_sources = [
                cs for cs in attacker.card_sources[:-1]
                if _is_vemmon_name(cs)
            ]
            returned = []
            for cs in vemmon_sources[:2]:
                attacker.card_sources.remove(cs)
                player.library_cards.append(cs)
                returned.append(cs)

            # Fire OnDigivolutionCardReturnToDeckBottom for each returned card
            for ret_card in returned:
                game.execute_effects(
                    EffectTiming.OnDigivolutionCardReturnToDeckBottom,
                    {"permanent": attacker, "returned_card": ret_card})

            # De-Digivolve 1 an opponent's Digimon
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)

            game.effect_select_opponent_permanent(
                player, on_de_digivolve,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ─── Effect 2: [Security] Play this card without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT18-092 Security: Play this card")
        effect2.set_effect_description("[Security] Play this card without paying the cost.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
