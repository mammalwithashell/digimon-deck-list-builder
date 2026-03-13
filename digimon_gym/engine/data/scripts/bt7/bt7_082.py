from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT7_082(CardScript):
    """BT7-082 Sistermon Blanc (Awakened) | Lv.4 White Digimon

    [On Play] You may place 1 [Sistermon Blanc] from your hand or trash
        at the bottom of this Digimon's digivolution cards to
        <Recovery +1 (Deck)>.

    [On Deletion] Return 1 card with [Jesmon], [Huckmon], or [Sistermon]
        in its name other than [Sistermon Blanc (Awakened)] from your
        trash to your hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _is_sistermon_blanc(c) -> bool:
            """Check if a card is named exactly 'Sistermon Blanc'."""
            names = getattr(c, 'card_names', []) or []
            return 'Sistermon Blanc' in names or 'SistermonBlanc' in names

        # --- Effect 0: [On Play] Place Sistermon Blanc under this, then Recovery +1 ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT7-082 On Play: place Sistermon Blanc, Recovery +1")
        effect0.set_effect_description(
            "[On Play] You may place 1 [Sistermon Blanc] from your hand or "
            "trash at the bottom of this Digimon's digivolution cards to "
            "<Recovery +1 (Deck)>."
        )
        effect0.is_on_play = True
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            perm = card.permanent_of_this_card()
            if not perm or not perm.is_digimon:
                return False
            # Must have Sistermon Blanc in hand or trash
            has_in_hand = any(_is_sistermon_blanc(c) for c in player.hand_cards)
            has_in_trash = any(_is_sistermon_blanc(c) for c in player.trash_cards)
            return has_in_hand or has_in_trash
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return

            # Find Sistermon Blanc in hand or trash, pick from whichever zone has one
            hand_blancs = [c for c in player.hand_cards if _is_sistermon_blanc(c)]
            trash_blancs = [c for c in player.trash_cards if _is_sistermon_blanc(c)]

            chosen = None
            if hand_blancs and trash_blancs:
                # Both zones available - use effect_choose_branch to let agent decide
                def on_branch(choice: int):
                    nonlocal chosen
                    if choice == 0 and hand_blancs:
                        chosen = hand_blancs[0]
                        player.hand_cards.remove(chosen)
                    elif choice == 1 and trash_blancs:
                        chosen = trash_blancs[0]
                        player.trash_cards.remove(chosen)
                    if chosen:
                        perm.add_card_source_bottom(chosen)
                        player.recovery(1)

                game.effect_choose_branch(
                    player, 2,
                    callback=on_branch,
                    branch_labels=["From hand", "From trash"])
                return
            elif hand_blancs:
                chosen = hand_blancs[0]
                player.hand_cards.remove(chosen)
            elif trash_blancs:
                chosen = trash_blancs[0]
                player.trash_cards.remove(chosen)

            if chosen:
                perm.add_card_source_bottom(chosen)
                player.recovery(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [On Deletion] Return 1 Jesmon/Huckmon/Sistermon (not self) from trash ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDestroyedAnyone)
        effect1.set_effect_name("BT7-082 On Deletion: return from trash to hand")
        effect1.set_effect_description(
            "[On Deletion] Return 1 card with [Jesmon], [Huckmon], or "
            "[Sistermon] in its name other than [Sistermon Blanc (Awakened)] "
            "from your trash to your hand."
        )
        effect1.is_on_deletion = True

        def condition1(context: Dict[str, Any]) -> bool:
            player = card.owner if card else None
            if not player:
                return False
            # Card must be in trash (just deleted)
            if card not in player.trash_cards:
                return False

            def trash_filter(c):
                if c is card:
                    return False
                names = getattr(c, 'card_names', []) or []
                # Exclude "Sistermon Blanc (Awakened)"
                if 'Sistermon Blanc (Awakened)' in names or 'SistermonBlanc(Awakened)' in names:
                    return False
                if c.contains_card_name('Jesmon'):
                    return True
                if c.contains_card_name('Huckmon'):
                    return True
                if c.contains_card_name('Sistermon'):
                    return True
                return False

            return any(trash_filter(c) for c in player.trash_cards)
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not player:
                return

            def trash_filter(c):
                if c is card:
                    return False
                names = getattr(c, 'card_names', []) or []
                if 'Sistermon Blanc (Awakened)' in names or 'SistermonBlanc(Awakened)' in names:
                    return False
                if c.contains_card_name('Jesmon'):
                    return True
                if c.contains_card_name('Huckmon'):
                    return True
                if c.contains_card_name('Sistermon'):
                    return True
                return False

            qualifying = [c for c in player.trash_cards if trash_filter(c)]
            if qualifying:
                chosen = qualifying[0]
                player.trash_cards.remove(chosen)
                player.hand_cards.append(chosen)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
