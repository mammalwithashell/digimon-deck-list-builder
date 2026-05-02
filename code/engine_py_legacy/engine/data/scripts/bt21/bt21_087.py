from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_087(CardScript):
    """BT21-087 Zenith | Tamer

    [Start of Your Turn] If your memory is at 2 or less, it becomes 3.

    [On Play] Reveal the top 3 cards of your deck. Among them, play 1
    [Vemmon] without paying the cost or add 1 card with [Vemmon] in its text
    to the hand. Trash the rest.

    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _has_vemmon_text(c) -> bool:
            return 'Vemmon' in getattr(c, 'card_text', '')

        def _is_vemmon_name(c) -> bool:
            return any('Vemmon' in n for n in getattr(c, 'card_names', []))

        # ─── Effect 0: [Start of Your Turn] If memory <= 2, set to 3.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("BT21-087 Set memory to 3")
        effect0.set_effect_description("[Start of Your Turn] If your memory is at 2 or less, it becomes 3.")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Set memory to 3 if <= 2."""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and game.memory <= 2:
                game.memory = 3
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ─── Effect 1: [On Play] Reveal top 3, play 1 [Vemmon] free OR add 1
        #     Vemmon-text to hand. Trash the rest.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT21-087 Reveal top 3: play Vemmon or add Vemmon-text to hand")
        effect1.set_effect_description("[On Play] Reveal the top 3 cards of your deck. Among them, play 1 [Vemmon] without paying the cost or add 1 card with [Vemmon] in its text to the hand. Trash the rest.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Reveal top 3, play 1 [Vemmon] free or add 1 Vemmon-text to hand. Trash rest."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            # Reveal top 3
            revealed = []
            for _ in range(min(3, len(player.library_cards))):
                revealed.append(player.library_cards.pop(0))
            if not revealed:
                return

            # Filter: can select a [Vemmon] to play free, or a Vemmon-text card to add to hand
            # Both qualify for selection; the selection determines what happens.
            # For RL simplicity: prefer playing a [Vemmon] if available, else add Vemmon-text to hand.
            vemmon_cards = [c for c in revealed if _is_vemmon_name(c)]
            vemmon_text_cards = [c for c in revealed if _has_vemmon_text(c)]

            if vemmon_cards:
                # Play 1 [Vemmon] without paying the cost
                chosen = vemmon_cards[0]
                revealed.remove(chosen)
                # Trash the rest
                for c in revealed:
                    player.trash_cards.append(c)
                played = player.play_card_from_source(chosen, pay_cost=False)
                if played:
                    game.execute_effects(EffectTiming.OnEnterFieldAnyone,
                        {"played_card": chosen, "played_permanent": played, "event_player": player})
            elif vemmon_text_cards:
                # Add 1 card with Vemmon text to hand
                chosen = vemmon_text_cards[0]
                revealed.remove(chosen)
                player.hand_cards.append(chosen)
                # Trash the rest
                for c in revealed:
                    player.trash_cards.append(c)
            else:
                # No qualifying cards; trash all
                for c in revealed:
                    player.trash_cards.append(c)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ─── Effect 2: [Security] Play this card without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT21-087 Security: Play this card")
        effect2.set_effect_description("[Security] Play this card without paying the cost.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
