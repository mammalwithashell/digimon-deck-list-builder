from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_073(CardScript):
    """EX11-073 ExMaquinamon | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Jogress Condition
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-073 Jogress Condition")
        effect0.set_effect_description("Jogress Condition")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Jogress condition marker — no runtime action."""
            pass

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Effect — link slot markers (up to 3 links). These are condition-gate
        # markers for the linking mechanic; the actual linking happens in effect4.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX11-073 Link slot 1 [Maquinamon]")
        effect1.set_effect_description("Effect")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Link slot marker — no runtime action."""
            pass

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        effect2 = ICardEffect()
        effect2.set_effect_name("EX11-073 Link slot 2 [Maquinamon]")
        effect2.set_effect_description("Effect")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Link slot marker — no runtime action."""
            pass

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        effect3 = ICardEffect()
        effect3.set_effect_name("EX11-073 Link slot 3 [Maquinamon]")
        effect3.set_effect_description("Effect")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Link slot marker — no runtime action."""
            pass

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may link up to 3 cards with [Maquinamon] in name
        # from your hand, trash, or this Digimon's digivolution cards.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("EX11-073 You may link up to 3 [Maquinamon] from hand, trash or digivolution cards")
        effect4.set_effect_description("Effect")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Link up to 3 [Maquinamon] cards from hand, trash, or digi cards."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            def is_maquinamon(c):
                names = getattr(c, 'card_names', []) or []
                return any('Maquinamon' in n for n in names)

            links_remaining = [3]  # mutable counter

            def link_next():
                if links_remaining[0] <= 0:
                    return
                # Gather candidates from hand
                hand_candidates = [
                    c for c in player.hand_cards if is_maquinamon(c)
                ]
                # Gather candidates from trash
                trash_candidates = [
                    c for c in player.trash_cards if is_maquinamon(c)
                ]
                # Gather candidates from this permanent's digivolution cards (excluding top)
                digi_candidates = []
                if perm and len(perm.card_sources) > 1:
                    digi_candidates = [
                        c for c in perm.card_sources[:-1] if is_maquinamon(c)
                    ]

                if not (hand_candidates or trash_candidates or digi_candidates):
                    return

                # Prefer hand selection since we have an API for it
                if hand_candidates:
                    def on_hand_select(selected_card):
                        if selected_card in player.hand_cards:
                            player.hand_cards.remove(selected_card)
                        perm.link_card(selected_card)
                        links_remaining[0] -= 1
                        link_next()

                    game.effect_select_hand_card(
                        player,
                        filter_fn=is_maquinamon,
                        callback=on_hand_select,
                        is_optional=True,
                        prompt="Select a [Maquinamon] card from hand to link.")
                elif trash_candidates:
                    # Link from trash — take the first matching card
                    for tc in trash_candidates:
                        if tc in player.trash_cards:
                            player.trash_cards.remove(tc)
                            perm.link_card(tc)
                            links_remaining[0] -= 1
                            link_next()
                            return
                elif digi_candidates:
                    # Link from digivolution cards — take the first matching card
                    for dc in digi_candidates:
                        if dc in perm.card_sources:
                            perm.card_sources.remove(dc)
                            perm.link_card(dc)
                            links_remaining[0] -= 1
                            link_next()
                            return

            link_next()

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEndTurn
        # [End of Opponent's Turn] [Once Per Turn] For each of this Digimon's linked cards,
        # trash 1 of your security cards to place 1 of your opponent's Digimon at the
        # bottom of its owner's deck.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnEndTurn)
        effect5.set_effect_name("EX11-073 Trash a security and bottom deck a digimon per link card")
        effect5.set_effect_description("Trash security + bottom-deck opponent Digimon per linked card")
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("EX11_073_EOOT_TRASH_BOUNCE")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # End of opponent's turn
            if card and card.owner and card.owner.is_my_turn:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: For each linked card, trash 1 security to bottom-deck 1 opponent Digimon."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            link_count = len(perm.linked_cards) if perm else 0
            if link_count <= 0:
                return

            iterations_left = [link_count]

            def do_iteration():
                if iterations_left[0] <= 0:
                    return
                # Must have security to trash
                if not player.security_cards:
                    return
                # Must have opponent Digimon to bottom-deck
                enemy = player.enemy if player else None
                if not enemy:
                    return
                opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
                if not opp_digimon:
                    return
                # Trash top security card
                trashed = player.security_cards.pop()
                player.trash_cards.append(trashed)
                iterations_left[0] -= 1
                # Select opponent Digimon to place at bottom of deck
                def target_filter(p):
                    return p.is_digimon
                def on_bottom_deck(target_perm):
                    enemy.return_permanent_to_deck_bottom(target_perm)
                    do_iteration()
                game.effect_select_opponent_permanent(
                    player, on_bottom_deck, filter_fn=target_filter, is_optional=False)

            do_iteration()

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
