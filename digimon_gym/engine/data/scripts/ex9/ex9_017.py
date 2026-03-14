from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_017(CardScript):
    """EX9-017 Garurumon | Lv.4 | Beast/DM/Ver.2
    <Training>
    [On Play][When Digivolving] By placing 1 hand card face down as bottom digi
    card, for each face-down digi card, trash 1 digi card from opponent's Digimon.
    Inherited: <Jamming>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # <Training>
        effect0 = ICardEffect()
        effect0.set_effect_name("EX9-017 Training")
        effect0.set_effect_description("<Training>")
        effect0._is_training = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        def _place_and_strip(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            if not player.hand_cards:
                return

            def on_hand_selected(selected_card):
                if selected_card is None:
                    return
                if selected_card in player.hand_cards:
                    player.hand_cards.remove(selected_card)
                    perm.add_card_source_bottom(selected_card)

                # Count face-down digi cards (cards that are flipped/face-down)
                # In the engine, face-down cards placed by Training or this effect
                # are tracked. We count all digivolution cards excluding the top card,
                # since the C# counts all face-down sources.
                digi_cards = getattr(perm, 'card_sources', []) or []
                face_down_count = len([c for c in digi_cards if c is not perm.top_card])

                # Trash that many digi cards from opponent's Digimon
                _strip_opponent_sources(player, game, enemy, face_down_count)

            game.effect_select_hand_card(
                player, lambda c: True, on_hand_selected, is_optional=True,
                prompt="Select 1 card to place face down as bottom digivolution card.")

        def _strip_opponent_sources(player, game, enemy, remaining):
            """Select opponent's Digimon to trash a digi card from, `remaining` times."""
            if remaining <= 0:
                return

            def target_filter(p):
                return p.is_digimon and len(getattr(p, 'card_sources', [])) > 1

            opp_with_sources = [p for p in enemy.battle_area if target_filter(p)]
            if not opp_with_sources:
                return

            def on_strip(target_perm):
                digi_cards = getattr(target_perm, 'card_sources', [])
                if len(digi_cards) > 1:
                    # Trash the bottom-most digivolution card (not top card)
                    removed = digi_cards.pop(0)
                    enemy.trash_cards.append(removed)
                # Continue selecting for remaining count
                _strip_opponent_sources(player, game, enemy, remaining - 1)

            game.effect_select_opponent_permanent(
                player, on_strip, filter_fn=target_filter, is_optional=False,
                prompt="Select opponent's Digimon to trash 1 digivolution card from.")

        # [On Play]
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX9-017 On Play: Place hand, strip opponent sources")
        effect1.set_effect_description(
            "[On Play] By placing 1 card in your hand face down as this "
            "Digimon's bottom digivolution card, for each of its face-down "
            "digivolution cards, trash any 1 digivolution card from your "
            "opponent's Digimon."
        )
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner and len(card.owner.hand_cards) < 1:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_place_and_strip)
        effects.append(effect1)

        # [When Digivolving]
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX9-017 When Digivolving: Place hand, strip opponent sources")
        effect2.set_effect_description(
            "[When Digivolving] By placing 1 card in your hand face down as "
            "this Digimon's bottom digivolution card, for each of its face-down "
            "digivolution cards, trash any 1 digivolution card from your "
            "opponent's Digimon."
        )
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner and len(card.owner.hand_cards) < 1:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_place_and_strip)
        effects.append(effect2)

        # Inherited: <Jamming>
        effect3 = ICardEffect()
        effect3.set_effect_name("EX9-017 Inherited: Jamming")
        effect3.set_effect_description("Inherited: <Jamming>")
        effect3.is_inherited_effect = True
        effect3._is_jamming = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
